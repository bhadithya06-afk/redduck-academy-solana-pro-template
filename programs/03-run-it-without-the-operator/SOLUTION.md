# Solution

## Why the caller does not matter

In `claim`, the destination accounts and payout amounts are determined strictly by on-chain state rather than by who signs the transaction:
- `user` is derived from `seeds = [b"user", user.owner.as_ref()], bump = user.bump`, and `user_tokens` is constrained to `token::mint = mint, token::authority = user.owner`.
- `user.pending` is read and set to `0` atomically before token transfers execute, preventing double claims regardless of how many callers race to submit the claim.
- The user always receives `pending - fee` in their own `user_tokens` account, while only the small executor fee (`fee`) goes to `caller_tokens`.

If the full reward were sent to the caller instead of `user.owner`, any third party watching pending rewards could drain every user's rewards to themselves. Routing the principal reward to `user_tokens` makes the executor's identity harmless: a stranger calling `claim` simply advances the user's payout and earns the small fee.

## The caller's fee

`compute_claim_split` sets the executor fee to `100` basis points (`1%` of `user.pending`), capped at `MAX_CLAIM_FEE` (`10` whole tokens, `10_000_000` base units at 6 decimals), and bounded by `total` (`fee.min(MAX_CLAIM_FEE).min(total)`).

- **What it covers:** Transaction signature fees, compute/priority fees, and the expected cost of lost races when multiple bots compete to submit a claim first.
- **Why it is capped:** A percentage fee without a cap would overcharge large claims disproportionately relative to the fixed on-chain execution cost of a single claim transaction. Capping at `10` tokens bounds protocol leakage on large payouts while leaving a `1%` reward (up to `10` tokens) on normal claims to incentivize independent executors.

## The price

The bot and `set_price` bot-authority check are completely removed. Instead, any caller brings a signed price payload (`price: u64 LE || publish_time: i64 LE`) inside an `Ed25519Program` instruction immediately preceding `set_price` or `buy` in the transaction:
- `apply_signed_price` verifies that `instructions` is the canonical `solana_program::sysvar::instructions::ID`, loads instruction `current_ix - 1`, confirms `program_id == ed25519_program::ID`, parses `(pubkey, message)` via `parse_ed25519`, and checks `signed.pubkey == config.oracle`.
- It decodes `(price, publish_time)` and checks freshness against `Clock::get()?.unix_timestamp`: `publish_time <= now && now.saturating_sub(publish_time) <= config.max_price_age`.
- If the update is older than `max_price_age` (or from the future), `apply_signed_price` / `buy` rejects it with `RewardsError::StalePrice`.

## What stays with the admin

Only `initialize` (bootstrapping the `["config"]` and `["vault"]` PDAs) and `accrue` (crediting rewards to users) remain under the `admin` key:
- `accrue` defines who earned what reward—a protocol accounting/source decision that cannot be permissionless without minting free claims out of thin air.
- Neither reward distribution (`claim`) nor price availability (`set_price` / `buy`) depends on an always-online admin or bot server; once rewards are accrued and signed oracle prices exist, any user or third-party executor can keep payouts and trades running without the team's server.
