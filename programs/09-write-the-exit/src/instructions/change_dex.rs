use anchor_lang::prelude::*;

use crate::{Config, RoundError, DEX_CHANGE_DELAY_SECONDS};

#[derive(Accounts)]
pub struct ProposeDex<'info> {
    pub admin: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.bump, has_one = admin)]
    pub config: Account<'info, Config>,
}

#[derive(Accounts)]
pub struct ApplyDex<'info> {
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
}

/// Queues a replacement swap venue under a timelock delay so `config.dex_program`
/// cannot be swapped instantaneously without notice.
pub fn propose_dex(ctx: Context<ProposeDex>, new_dex: Pubkey) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let ready_at = now
        .checked_add(DEX_CHANGE_DELAY_SECONDS)
        .ok_or(RoundError::MathOverflow)?;
    ctx.accounts.config.pending_dex_program = new_dex;
    ctx.accounts.config.dex_change_ready_at = ready_at;
    Ok(())
}

/// Applies a queued replacement swap venue once `DEX_CHANGE_DELAY_SECONDS` has elapsed.
pub fn apply_dex(ctx: Context<ApplyDex>) -> Result<()> {
    let config = &mut ctx.accounts.config;
    require!(
        config.pending_dex_program != Pubkey::default(),
        RoundError::NoPendingDexChange
    );
    let now = Clock::get()?.unix_timestamp;
    require!(
        now >= config.dex_change_ready_at,
        RoundError::DexChangeNotReady
    );

    config.dex_program = config.pending_dex_program;
    config.pending_dex_program = Pubkey::default();
    config.dex_change_ready_at = 0;
    Ok(())
}
