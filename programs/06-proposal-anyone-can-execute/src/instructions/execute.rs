use anchor_lang::prelude::*;
use anchor_lang::solana_program::instruction::{AccountMeta, Instruction};
use anchor_lang::solana_program::program::invoke_signed;

use crate::{Config, MultisigError, Proposal};

/// Runs the stored instruction with the vault PDA as the signer once threshold approvals
/// are met. Any caller may send this transaction once approved; no admin or member key is needed.
#[derive(Accounts)]
pub struct Execute<'info> {
    pub executor: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"proposal", proposal.id.to_le_bytes().as_ref()], bump = proposal.bump)]
    pub proposal: Account<'info, Proposal>,
    /// CHECK: the treasury PDA that signs the stored instruction via `invoke_signed`.
    #[account(seeds = [b"vault"], bump = config.vault_bump)]
    pub vault: UncheckedAccount<'info>,
}

pub fn execute<'info>(ctx: Context<'_, '_, 'info, 'info, Execute<'info>>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let threshold = ctx.accounts.config.threshold;
    let vault_key = ctx.accounts.vault.key();

    let (program_id, metas, data) = {
        let proposal = &ctx.accounts.proposal;
        require!(!proposal.executed, MultisigError::AlreadyExecuted);
        require!(now <= proposal.expires_at, MultisigError::ProposalExpired);
        require!(proposal.approvals >= threshold, MultisigError::NotApproved);

        // Expect each stored AccountMeta followed by the target program account.
        require!(
            ctx.remaining_accounts.len() == proposal.metas.len() + 1,
            MultisigError::AccountMismatch
        );

        for (stored, actual) in proposal
            .metas
            .iter()
            .zip(ctx.remaining_accounts[..proposal.metas.len()].iter())
        {
            require_keys_eq!(actual.key(), stored.pubkey, MultisigError::AccountMismatch);
            require!(
                actual.is_writable == stored.is_writable,
                MultisigError::AccountMismatch
            );
            // Only the vault PDA can be signed via `invoke_signed`; non-vault signers are rejected.
            if stored.is_signer {
                require_keys_eq!(actual.key(), vault_key, MultisigError::AccountMismatch);
            }
            require!(!actual.is_signer, MultisigError::AccountMismatch);
        }

        let target_program_acc = &ctx.remaining_accounts[proposal.metas.len()];
        require_keys_eq!(
            target_program_acc.key(),
            proposal.program_id,
            MultisigError::AccountMismatch
        );
        require!(!target_program_acc.is_signer, MultisigError::AccountMismatch);

        let metas: Vec<AccountMeta> = proposal
            .metas
            .iter()
            .map(|meta| AccountMeta {
                pubkey: meta.pubkey,
                is_signer: meta.is_signer && meta.pubkey == vault_key,
                is_writable: meta.is_writable,
            })
            .collect();

        (proposal.program_id, metas, proposal.data.clone())
    };

    // Mark executed before CPI so the proposal can never execute twice.
    ctx.accounts.proposal.executed = true;

    let seeds: &[&[u8]] = &[b"vault", &[ctx.accounts.config.vault_bump]];
    invoke_signed(
        &Instruction {
            program_id,
            accounts: metas,
            data,
        },
        ctx.remaining_accounts,
        &[seeds],
    )?;

    Ok(())
}
