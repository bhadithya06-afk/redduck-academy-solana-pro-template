# Solution

## Why anyone may execute

Once a proposal has collected `threshold` approvals from distinct valid `Config` members, the decision has already been committed on-chain. Letting any wallet send `execute` removes liveness bottlenecks and admin single-points-of-failure without weakening security:
- **What the executor can do:** Pay transaction fees and submit the approved `Proposal` once `proposal.approvals >= config.threshold` and `!proposal.executed`.
- **What the executor cannot change:** The target `program_id`, the stored `metas` (`pubkey` and `is_writable` must match one-for-one), the stored `data`, the `vault` PDA authority, or the approval count. Any altered account list or program fails with `MultisigError::AccountMismatch`.

## What the PDA signs

At execution, the program invokes the stored instruction via `invoke_signed` using only the `["vault", &[config.vault_bump]]` PDA seeds.
- That grants a programmatic signer privilege **solely** for the `vault` PDA—not for any member's wallet, not for any arbitrary PDA, and not for any instruction payload outside `proposal.program_id`, `proposal.metas`, and `proposal.data` stored in the approved `Proposal` account.

## What stays with the members

Creating proposals (`create_proposal`) and approving them (`approve`) still require a valid member keypair in `config.members`:
- Members define **what** action is proposed and **whether** the multisig consents to it up to `config.threshold`.
- Once consent is recorded on-chain in the `Proposal` account, execution becomes a mechanical, deterministic step that any caller can trigger.

## The exits

An unexecuted proposal that never reaches `threshold` (or is no longer needed) can be closed via `cancel`, returning its rent lamports to `destination`:
- Before `expires_at`, the `proposer` or any `config` member can cancel an unexecuted proposal.
- After `expires_at` (`now > proposal.expires_at`), **anyone** can call `cancel` to close the expired proposal and reclaim its rent so stale proposal accounts do not stay locked indefinitely.
