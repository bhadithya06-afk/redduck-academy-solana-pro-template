use anchor_lang::prelude::*;

use crate::{Config, MultisigError, Proposal};

/// Cancels an unexecuted proposal and returns its rent lamports to `destination`.
/// Authorized for the proposer or any multisig member before expiry, or anyone after `expires_at`.
#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    /// CHECK: receives the closed proposal account's rent lamports.
    #[account(mut)]
    pub destination: UncheckedAccount<'info>,
    #[account(
        mut,
        close = destination,
        seeds = [b"proposal", proposal.id.to_le_bytes().as_ref()],
        bump = proposal.bump,
    )]
    pub proposal: Account<'info, Proposal>,
}

pub fn cancel(ctx: Context<Cancel>) -> Result<()> {
    require!(!ctx.accounts.proposal.executed, MultisigError::AlreadyExecuted);

    let now = Clock::get()?.unix_timestamp;
    let is_expired = now > ctx.accounts.proposal.expires_at;
    let caller_key = ctx.accounts.caller.key();
    let allowed = is_expired
        || caller_key == ctx.accounts.proposal.proposer
        || ctx.accounts.config.is_member(&caller_key);

    require!(allowed, MultisigError::NotAMember);
    Ok(())
}
