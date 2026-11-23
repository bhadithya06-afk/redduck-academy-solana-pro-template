# Solution

## Why a total commutes and a step does not

A cumulative lifetime total (`total`) represents a monotonic high-water mark (`max` / lattice join over cumulative earnings), whereas a daily step (`delta`) is an additive increment that requires exact sequential ordering and uniqueness:
- **In the given delta program:** Suppose day 1 has delta `+10` and day 2 has delta `+15` (true cumulative total `25`). If `+15` arrives first and `+10` arrives second with strict `nonce == device.nonce + 1`, the second message is rejected and `15` is lost; if a message is replayed (or a message is dropped), the balance either double-pays or misses rewards permanently.
- **In this cumulative total program:** Each message carries the device's cumulative earned total (`total`). `claim` pays `max(0, total - device.paid)` and sets `device.paid = max(device.paid, total)`.
  - **Older (`10`) then newer (`25`):** First claim pays `10 - 0 = 10`, `paid = 10`. Second claim pays `25 - 10 = 15`, `paid = 25`.
  - **Newer (`25`) then older (`10`):** First claim pays `25 - 0 = 25`, `paid = 25`. Second claim sees `10 <= 25`, pays `0`, and keeps `paid = 25`.
  - **Duplicates:** Re-sending `10` or `25` sees `total <= device.paid`, pays `0`, and leaves `paid = 25` and the token balance unchanged. Every arrival order and replay pattern converges to `paid = 25` and total payout `25`.

## What the program compares

`claim` first verifies `ctx.accounts.instructions.key() == ix_sysvar::ID` before reading any instruction from the sysvar, loads the preceding Ed25519 instruction, checks that `signed.pubkey == ctx.accounts.config.oracle`, and compares `signed.message` against `claim_message(&ctx.accounts.device.key(), total)` (`device_pubkey || total_le_bytes`).

For ordering and idempotency, `claim` compares the signed `total` **only** against `device.paid` (the cumulative amount already paid to that `Device` PDA). It never compares against a nonce, a counter, a timestamp, or any historical list/map of previously seen messages.

## Why the nonce could go

- **What the nonce guaranteed:** Only that a specific sequence number had not yet incremented `device.nonce`.
- **What it did not guarantee:** It could not tolerate out-of-order delivery or dropped messages without losing rewards or breaking ordering, and by itself a nonce does not bound the cumulative amount paid.
- **Why it is unnecessary now:** Because each signed `total` already subsumes all prior cumulative earnings up to that point and `device.paid` tracks the highest cumulative total already settled, any duplicate or stale message with `total <= device.paid` is naturally a no-op (`0` additional payout) without needing a nonce or message history.

## The boundary of the oracle's consent

The oracle signs `(device_pubkey, cumulative_total)`—consenting only to the fact that `device_pubkey` has earned at least `cumulative_total` tokens in total. A claim can never pay more than `cumulative_total` overall, can never transfer tokens to a different device's owner, can never pay out of any mint other than `config.mint`, and cannot exceed the `vault`'s token balance.
