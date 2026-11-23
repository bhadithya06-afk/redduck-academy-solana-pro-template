# Solution

## The chain as you found it

In the original `withdraw` instruction, every check depended on accounts supplied directly by the caller:

1. **Instructions sysvar lookup (`load_instruction_unchecked(0, &ctx.accounts.instructions)`):**
   - **Account read:** `ctx.accounts.instructions` (`UncheckedAccount<'info>`).
   - **Caller-controllable?** Yes. Its address was never checked against `solana_program::sysvar::instructions::ID`, so the caller could pass any account containing bytes formatted like the Instructions sysvar.
2. **Ed25519 precompile verification (`parse_ed25519(&signature_check)`):**
   - **Account / data read:** Instruction `0` decoded from `ctx.accounts.instructions` (checking `ix.program_id == ed25519_program::ID` and extracting `pubkey` and `message`).
   - **Caller-controllable?** Yes. Because `instructions` was not verified to be the real runtime sysvar, the Ed25519 precompile never actually executed—the caller simply wrote `ed25519_program::ID` and arbitrary pubkey/message bytes into a fake account.
3. **Oracle key allowlist check (`ctx.accounts.allowlist.keys.contains(&signed.pubkey)`):**
   - **Account read:** `ctx.accounts.allowlist` (`Account<'info, Allowlist>`).
   - **Caller-controllable?** Yes. `Withdraw` placed no seed, address, or authority constraint on `allowlist`, and `create_allowlist` allowed any caller to create an `Allowlist` account containing their own public key.
4. **Withdrawal payload check (`signed.message == withdrawal_message(&vault, &recipient_tokens, amount)`):**
   - **Accounts read:** `ctx.accounts.vault` (PDA `["vault"]`), `ctx.accounts.recipient_tokens`, and `amount`.
   - **Caller-controllable?** Indirectly yes: once the caller controlled both `instructions` and `allowlist`, they could supply any matching message signed by their own key.

## Where the chain ended

The chain never reached an unforgeable root of trust—it terminated in two caller-controlled accounts (`instructions` and `allowlist`) whose identities were completely unverified.

In the exploit test, a caller constructs a regular `SystemProgram`-owned account whose data matches the binary layout of the Instructions sysvar (`u16` count `1`, `u16` offset `4`, `0` account metas, `Ed25519Program.programId`, data length, and an Ed25519 instruction header/payload for the attacker's keypair), alongside an `Allowlist` account created via `create_allowlist` holding that same attacker public key. Passing both accounts into `withdraw` satisfies every check without the Ed25519 precompile ever running or a legitimate oracle ever signing.

## The root

Two anchors now serve as the root of trust:

1. **The program-derived `Config` PDA (`seeds = [b"config"]`, `bump`):**
   - **Why the caller cannot create or forge it:** Only the `oracle_vault` program can sign for the `["config"]` PDA seeds, and `initialize` can run only once to create that PDA under `admin` authority. Allowed oracle keys (`allowed_keys`) now live inside `Config` rather than in an arbitrary external `Allowlist` account.
2. **The canonical Solana runtime Instructions sysvar (`solana_program::sysvar::instructions::ID`):**
   - **Why the caller cannot create or forge it:** Its address is a fixed protocol constant (`Sysvar1nstructions1111111111111111111111111`) populated exclusively by the Solana runtime with the actual instructions of the executing transaction.

Every check in `withdraw` now derives from these two roots:
- `instructions` is constrained with `#[account(address = ix_sysvar::ID)]` before `load_current_index_checked` and `load_instruction_unchecked` read the preceding instruction (`current_ix - 1`), guaranteeing the Ed25519 precompile genuinely verified the signature in the same transaction.
- `signed.pubkey` is checked against `config.allowed_keys` inside the `["config"]` PDA.
- `signed.message` is checked against `withdrawal_message(&vault.key(), &recipient_tokens.key(), amount)` where `vault` is constrained to `seeds = [b"vault"]` and `mint` is constrained to `config.mint`.

## One root for every instruction

Every instruction in `oracle_vault` derives `config` from the exact same canonical PDA (`#[account(seeds = [b"config"], bump = config.bump)]`):
- **`initialize`:** Creates the `["config"]` PDA (`init`, `seeds = [b"config"]`, `bump`) storing `admin`, `mint`, `bump`, and `allowed_keys`, and initializes the `["vault"]` token account with `token::authority = config`.
- **`deposit`:** Constrains `config` with `seeds = [b"config"], bump = config.bump`, binds `mint` via `#[account(address = config.mint)]`, and deposits into the `["vault"]` PDA.
- **`withdraw`:** Constrains `config` with `seeds = [b"config"], bump = config.bump`, binds `mint` via `#[account(address = config.mint)]`, verifies `signed.pubkey` against `config.allowed_keys`, and signs the SPL token transfer from `vault` using the `["config"]` PDA seeds.
