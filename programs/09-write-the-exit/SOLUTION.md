# Solution

## Dependency 1: the ephemeral rollup

**Would the product make sense without it?** Yes. A round-based staking game can settle outcomes on Solana directly without requiring an external ephemeral rollup to hold player funds hostage.

**Should the rounds run on the rollup at all?** Offloading high-frequency in-round moves or complex off-chain game state can avoid Solana's per-transaction compute unit and account write-lock limits if moves are numerous, whereas small/simple rounds can run directly on Solana without introducing external liveness risk. Crucially, fund custody and settlement/exit guarantees must never trap user stakes inside an uncooperative or offline rollup.

**The exit:** Triggered once a round is unsettled (`!round.settled`) and the current time reaches `round.deadline + REFUND_DELAY_SECONDS` (`300` seconds). Once that refund window opens, `settle` is also blocked (`now < round.deadline + REFUND_DELAY_SECONDS`) so a round cannot be settled after refunds become available. Any staker (`entry.player`) can trigger `refund` on their own `Entry` without the rollup executor key, recovering `entry.amount` from `vault` back to `player_tokens` while `vault` holds the escrowed stakes until settlement or refund.

## Dependency 2: the external DEX

**Would the product make sense without it?** Yes. Players can already join directly with the game's base token via `join`; optional swap-assisted entry (`join_with_swap`) is a convenience feature, not essential to core gameplay.

**The exit:** `propose_dex` (by `admin`) queues `pending_dex_program` with `dex_change_ready_at = now + DEX_CHANGE_DELAY_SECONDS` (`86_400` seconds / 24 hours), and `apply_dex` activates it only after the delay elapses—replacing the swap venue without a program upgrade.

**If the key were stolen:** An attacker with the admin key cannot immediately point `join_with_swap` to a malicious swap program because `apply_dex` enforces the `86_400`-second timelock delay, giving users and guardians time to detect the queued change and warn users away from `join_with_swap`.

## Dependency 3: the upgrade authority

**Would the product make sense without it?** Yes. Once a program is deployed and verified, immutable or timelocked governance is viable and avoids unilateral key risk.

**The exit:** Transfer upgrade authority from the third-party 2-of-3 multisig into a protocol-controlled timelock governance authority with a mandatory **48-hour (`172,800`-second)** upgrade delay triggered whenever a new program binary hash is queued for upgrade, and schedule an explicit freeze/immutability trigger after **7 days (`604,800` seconds)** of stable post-launch operation (or if the third-party operator's SLA/responsiveness fails) so upgradeability is permanently revoked (`set_upgrade_authority` to `None` / immutable).

**If the key were stolen:** Today, a compromised 2-of-3 keyset with no timelock can immediately deploy arbitrary bytecode in a single transaction, seizing vault/user funds or altering rules without warning or exit time.

## What you implemented

- Added `REFUND_DELAY_SECONDS` (`300`s) and `DEX_CHANGE_DELAY_SECONDS` (`86_400`s) in `src/constants.rs`, plus error codes in `src/error.rs`.
- Extended `Config` in `src/state/config.rs` with `dex_program`, `pending_dex_program`, and `dex_change_ready_at` (initialized in `src/instructions/initialize.rs`), and updated `src/instructions/join_with_swap.rs` to validate against `config.dex_program`.
- Implemented `refund` in `src/instructions/refund.rs` (300s post-deadline self-refund without executor) and guarded `settle` in `src/instructions/settle.rs` so settlement is rejected once the refund window opens (`now >= deadline + 300`).
- Implemented `propose_dex` and `apply_dex` in `src/instructions/change_dex.rs` with an `86_400`-second (24-hour) timelock delay so a compromised admin key cannot switch the swap venue instantaneously.
