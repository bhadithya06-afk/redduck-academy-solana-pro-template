# Solution

## Dependency 1: the ephemeral rollup

**Would the product make sense without it?** Yes. A round-based staking game can settle outcomes on Solana directly without requiring an external ephemeral rollup to hold player funds hostage.

**Should the rounds run on the rollup at all?** Offloading high-frequency in-round moves or complex off-chain game state can avoid Solana's per-transaction compute unit and account write-lock limits if moves are numerous, whereas small/simple rounds can run directly on Solana without introducing external liveness risk. Crucially, fund custody and settlement/exit guarantees must never trap user stakes inside an uncooperative or offline rollup.

**The exit:** Triggered once a round is unsettled (`!round.settled`) and the current time reaches `round.deadline + REFUND_DELAY_SECONDS` (`300` seconds). Any staker (`entry.player`) can trigger `refund` on their own `Entry` without the rollup executor key, recovering `entry.amount` from `vault` back to `player_tokens` while `vault` holds the escrowed stakes until settlement or refund.

## Dependency 2: the external DEX

**Would the product make sense without it?** Yes. Players can already join directly with the game's base token via `join`; optional swap-assisted entry (`join_with_swap`) is a convenience feature, not essential to core gameplay.

**The exit:** `propose_dex` (by `admin`) queues `pending_dex_program` with `dex_change_ready_at = now + DEX_CHANGE_DELAY_SECONDS` (`86_400` seconds / 24 hours), and `apply_dex` activates it only after the delay elapses—replacing the swap venue without a program upgrade.

**If the key were stolen:** An attacker with the admin key cannot immediately point `join_with_swap` to a malicious swap program because `apply_dex` enforces the `86_400`-second timelock delay, giving users and guardians time to detect the queued change and warn users away from `join_with_swap`.

## Dependency 3: the upgrade authority

**Would the product make sense without it?** Yes. Once a program is deployed and verified, immutable or timelocked governance is viable and avoids unilateral key risk.

**The exit:** Move the upgrade authority out of an uncontrolled third-party 2-of-3 multisig with no timelock into a transparent governance multisig governed by a mandatory on-chain upgrade timelock (or renounce/freeze upgradeability once mature), so users can inspect proposed bytecode changes and exit before any upgrade takes effect.

**If the key were stolen:** Today, a compromised 2-of-3 keyset with no timelock could immediately push a malicious program upgrade in one transaction and arbitrarily alter program logic or drain funds without notice.

## What you implemented

- Added `REFUND_DELAY_SECONDS` (`300`s) and `DEX_CHANGE_DELAY_SECONDS` (`86_400`s) in `src/constants.rs`, plus corresponding error codes in `src/error.rs`.
- Extended `Config` in `src/state/config.rs` with `dex_program`, `pending_dex_program`, and `dex_change_ready_at` (initialized in `src/instructions/initialize.rs`), and updated `src/instructions/join_with_swap.rs` to validate against `config.dex_program`.
- Implemented `refund` in `src/instructions/refund.rs` (300s post-deadline self-refund without executor) and `propose_dex` / `apply_dex` in `src/instructions/change_dex.rs` (86,400s timelock delay so a compromised admin key cannot switch the swap venue instantaneously).
