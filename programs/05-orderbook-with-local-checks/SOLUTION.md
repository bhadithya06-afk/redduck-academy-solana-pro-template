# Solution

## The two complexities

- **Off-chain search complexity:** $O(n)$ (or $O(\log n)$ if indexed off-chain) to scan the book and find the predecessor cell `after` such that `prev.price >= price > next.price`. Because the client pays for its own off-chain computation, moving the search off-chain shifts work away from scarce on-chain compute units.
- **On-chain placement complexity:** $O(1)$ time and $O(1)$ account cell reads/writes. `validate_hint` reads only `book.head` (when `hint` is `None`) or `book.cells[h]` and its immediate successor `book.cells[prev.next]` (when `hint` is `Some(h)`), without any loop over the 64 cells.
- **Why on-chain cost does not grow with book size:** Once `hint` is supplied, verifying local sortedness requires checking at most two adjacent nodes (`prev` and `next`), regardless of whether the list contains 2 orders or 64 orders.

## The checks

`validate_hint` performs the following local checks and rejects any violation with `BookError::BadHint`:

1. **For `hint == None` (head insertion):**
   - If `book.head != NONE`, verifies `head_cell.alive == 1` and `price > head_cell.price`.
   - Catches a `None` hint when the current head is cheaper than or equal to `price` (e.g., `price <= head_cell.price`), or when the head cell is dead/corrupt.
2. **For `hint == Some(h)` (insertion right after cell `h`):**
   - Verifies `(h as usize) < CELLS` and `prev_cell.alive == 1` — catches out-of-range hints or hints pointing to a cancelled/dead cell.
   - Verifies `prev_cell.price >= price` — catches hints where the predecessor order is cheaper than the new order (violating descending price order).
   - If `prev_cell.next != NONE`, verifies `next_cell.alive == 1` and `price > next_cell.price` — catches hints where the successor order is at least as expensive as `price` (meaning the new order belongs further down the list), as well as **stale hints** where another order was inserted after `h` in between computing the hint and landing the transaction.

## The fixed capacity

- **Why capacity is fixed (`CELLS = 64` in zero-copy `Book`):** On Solana, a single account has a fixed byte size (`Book::SIZE`) and a transaction has a fixed compute unit and account/serialization budget. Fixing `CELLS` bounds account rent, zero-copy memory layout, and worst-case execution cost so book operations never hit unbounded growth.
- **What an attacker could do without a fixed bound or fund lock:** If an attacker could grow an on-chain linked list indefinitely and force every new order to scan it on-chain ($O(n)$), they could bloat the list and price legitimate users out of placing orders (compute/DOS exhaustion).
- **What stops them from filling this book for free:** Every open order in `place_order` transfers `size` tokens (`require!(size > 0, BookError::BadHint)`) from `owner_tokens` into `vault` via `transfer_checked`, and only returns those tokens on `cancel_order`. Filling all 64 cells requires locking real tokens across all 64 positions rather than spamming empty entries for free.
