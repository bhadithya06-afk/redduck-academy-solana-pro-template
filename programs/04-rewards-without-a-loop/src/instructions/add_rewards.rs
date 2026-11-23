use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{Pool, PoolError};

pub const REWARD_PRECISION: u128 = 1_000_000_000_000;

/// Adds rewards to the pool in O(1) time by raising `pool.reward_per_token`.
/// No staker accounts are touched; any remaining accounts passed by callers are ignored.
#[derive(Accounts)]
pub struct AddRewards<'info> {
    pub funder: Signer<'info>,
    #[account(mut, seeds = [b"pool"], bump = pool.bump)]
    pub pool: Account<'info, Pool>,
    #[account(address = pool.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = funder)]
    pub funder_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn add_rewards<'info>(
    ctx: Context<'_, '_, 'info, 'info, AddRewards<'info>>,
    amount: u64,
) -> Result<()> {
    let total_staked = ctx.accounts.pool.total_staked;
    require!(total_staked > 0, PoolError::NothingStaked);

    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.funder_tokens.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.funder.to_account_info(),
            },
        ),
        amount,
        ctx.accounts.mint.decimals,
    )?;

    let delta_rpt = (amount as u128)
        .checked_mul(REWARD_PRECISION)
        .ok_or(PoolError::MathOverflow)?
        .checked_div(total_staked as u128)
        .ok_or(PoolError::MathOverflow)?;

    ctx.accounts.pool.reward_per_token = ctx
        .accounts
        .pool
        .reward_per_token
        .checked_add(delta_rpt)
        .ok_or(PoolError::MathOverflow)?;

    Ok(())
}

/// Settles any rewards earned since `staker.reward_debt` using the staker's *current*
/// stake before any stake modification or claim occurs, then advances `staker.reward_debt`.
pub fn settle_staker(pool_reward_per_token: u128, staker: &mut crate::Staker) -> Result<()> {
    let delta_rpt = pool_reward_per_token
        .checked_sub(staker.reward_debt)
        .ok_or(PoolError::MathOverflow)?;
    let newly_earned = (staker.stake as u128)
        .checked_mul(delta_rpt)
        .ok_or(PoolError::MathOverflow)?
        / REWARD_PRECISION;
    let newly_earned_u64 = u64::try_from(newly_earned).map_err(|_| PoolError::MathOverflow)?;
    staker.accrued = staker
        .accrued
        .checked_add(newly_earned_u64)
        .ok_or(PoolError::MathOverflow)?;
    staker.reward_debt = pool_reward_per_token;
    Ok(())
}
