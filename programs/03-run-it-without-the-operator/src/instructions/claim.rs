use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{Config, RewardsError, User};

/// 100 bps = 1.00% executor fee on claimed rewards, capped at MAX_FEE_UNITS.
pub const CLAIM_FEE_BPS: u64 = 100;
pub const BPS_DENOMINATOR: u64 = 10_000;
/// Cap the caller fee at 10 tokens (10 * 10^6 base units for a 6-decimal mint).
pub const MAX_CLAIM_FEE: u64 = 10_000_000;

pub fn compute_claim_split(total: u64) -> Result<(u64, u64)> {
    let raw_fee = total
        .checked_mul(CLAIM_FEE_BPS)
        .ok_or(RewardsError::MathOverflow)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(RewardsError::MathOverflow)?;
    let fee = raw_fee.min(MAX_CLAIM_FEE).min(total);
    let user_amount = total.checked_sub(fee).ok_or(RewardsError::MathOverflow)?;
    Ok((user_amount, fee))
}

/// Permissionless reward claim: any caller can trigger payout for `user.owner`.
/// `user_tokens` always receives `total - fee` and `caller_tokens` receives `fee`.
#[derive(Accounts)]
pub struct Claim<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"user", user.owner.as_ref()], bump = user.bump)]
    pub user: Account<'info, User>,
    #[account(address = config.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = user.owner)]
    pub user_tokens: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint, token::authority = caller)]
    pub caller_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn claim(ctx: Context<Claim>) -> Result<()> {
    let total = ctx.accounts.user.pending;
    require!(total > 0, RewardsError::NothingToPay);

    // Zero pending rewards before token transfers so a claim can never pay twice.
    ctx.accounts.user.pending = 0;

    let (user_amount, fee) = compute_claim_split(total)?;
    let seeds: &[&[u8]] = &[b"config", &[ctx.accounts.config.bump]];

    if user_amount > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.user_tokens.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                &[seeds],
            ),
            user_amount,
            ctx.accounts.mint.decimals,
        )?;
    }

    if fee > 0 {
        transfer_checked(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.vault.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.caller_tokens.to_account_info(),
                    authority: ctx.accounts.config.to_account_info(),
                },
                &[seeds],
            ),
            fee,
            ctx.accounts.mint.decimals,
        )?;
    }

    Ok(())
}
