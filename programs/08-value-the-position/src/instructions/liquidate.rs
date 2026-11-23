use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    pricing::{read_band, value},
    update_collateral_average, Market, MarketError, Position, BPS,
};

#[derive(Accounts)]
pub struct Liquidate<'info> {
    pub liquidator: Signer<'info>,
    #[account(mut, seeds = [b"market"], bump = market.bump)]
    pub market: Account<'info, Market>,
    #[account(mut, seeds = [b"position", position.owner.as_ref()], bump = position.bump)]
    pub position: Account<'info, Position>,
    #[account(address = market.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    #[account(address = market.debt_mint)]
    pub debt_mint: Account<'info, Mint>,
    #[account(mut, token::mint = debt_mint, token::authority = liquidator)]
    pub liquidator_debt: Account<'info, TokenAccount>,
    #[account(mut, token::mint = collateral_mint)]
    pub liquidator_collateral: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"debt_vault"], bump)]
    pub debt_vault: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"collateral_vault"], bump)]
    pub collateral_vault: Account<'info, TokenAccount>,
    pub collateral_price: Account<'info, PriceUpdateV2>,
    pub debt_price: Account<'info, PriceUpdateV2>,
    pub token_program: Program<'info, Token>,
}

pub fn liquidate(ctx: Context<Liquidate>) -> Result<()> {
    let clock = Clock::get()?;
    let col_band = read_band(
        &ctx.accounts.collateral_price,
        &ctx.accounts.market.collateral_feed,
        ctx.accounts.market.max_price_age,
        &clock,
    )?;
    let debt_band = read_band(
        &ctx.accounts.debt_price,
        &ctx.accounts.market.debt_feed,
        ctx.accounts.market.max_price_age,
        &clock,
    )?;

    let market = &mut ctx.accounts.market;
    let _ = update_collateral_average(market, col_band.mid, clock.unix_timestamp)?;

    let debt_amount = ctx.accounts.position.debt;
    let collateral_amount = ctx.accounts.position.collateral;
    require!(debt_amount > 0, MarketError::NothingToLiquidate);

    // Liquidation uses spot low band for collateral and high band for debt, without borrow-only
    // spike/LTV rules so liquidations are never blocked by rules protecting new loans.
    let collateral_val = value(
        collateral_amount,
        col_band.low,
        ctx.accounts.collateral_mint.decimals,
    )?;
    let debt_val = value(
        debt_amount,
        debt_band.high,
        ctx.accounts.debt_mint.decimals,
    )?;

    require!(
        debt_val
            .checked_mul(BPS as u128)
            .ok_or(MarketError::MathOverflow)?
            > collateral_val
                .checked_mul(market.liquidation_ltv_bps as u128)
                .ok_or(MarketError::MathOverflow)?,
        MarketError::NotLiquidatable
    );

    // Repay full debt from liquidator to debt_vault
    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.liquidator_debt.to_account_info(),
                mint: ctx.accounts.debt_mint.to_account_info(),
                to: ctx.accounts.debt_vault.to_account_info(),
                authority: ctx.accounts.liquidator.to_account_info(),
            },
        ),
        debt_amount,
        ctx.accounts.debt_mint.decimals,
    )?;

    ctx.accounts.position.debt = 0;
    ctx.accounts.position.collateral = 0;

    let seeds: &[&[u8]] = &[b"market", &[market.bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.collateral_vault.to_account_info(),
                mint: ctx.accounts.collateral_mint.to_account_info(),
                to: ctx.accounts.liquidator_collateral.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[seeds],
        ),
        collateral_amount,
        ctx.accounts.collateral_mint.decimals,
    )
}
