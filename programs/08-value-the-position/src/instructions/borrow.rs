use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{
    pricing::{read_band, value},
    Market, MarketError, Position, AVERAGE_WINDOW_SECONDS, BPS,
};

#[derive(Accounts)]
pub struct Borrow<'info> {
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"market"], bump = market.bump)]
    pub market: Account<'info, Market>,
    #[account(
        mut,
        seeds = [b"position", owner.key().as_ref()],
        bump = position.bump,
        has_one = owner,
    )]
    pub position: Account<'info, Position>,
    #[account(address = market.collateral_mint)]
    pub collateral_mint: Account<'info, Mint>,
    #[account(address = market.debt_mint)]
    pub debt_mint: Account<'info, Mint>,
    #[account(mut, token::mint = debt_mint)]
    pub owner_debt: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"debt_vault"], bump)]
    pub debt_vault: Account<'info, TokenAccount>,
    pub collateral_price: Account<'info, PriceUpdateV2>,
    pub debt_price: Account<'info, PriceUpdateV2>,
    pub token_program: Program<'info, Token>,
}

pub fn update_collateral_average(market: &mut Market, mid_price: u64, now: i64) -> Result<u64> {
    if market.avg_collateral_price == 0 || market.last_price_time == 0 {
        market.avg_collateral_price = mid_price;
        market.last_price_time = now;
        return Ok(market.avg_collateral_price);
    }

    let elapsed = now.saturating_sub(market.last_price_time).max(0) as u128;
    let dt = elapsed.min(AVERAGE_WINDOW_SECONDS as u128);
    let window = AVERAGE_WINDOW_SECONDS as u128;
    let remaining = window.saturating_sub(dt);

    let weighted = ((market.avg_collateral_price as u128)
        .checked_mul(remaining)
        .ok_or(MarketError::MathOverflow)?)
    .checked_add(
        (mid_price as u128)
            .checked_mul(dt)
            .ok_or(MarketError::MathOverflow)?,
    )
    .ok_or(MarketError::MathOverflow)?
        / window;

    let avg = u64::try_from(weighted).map_err(|_| MarketError::MathOverflow)?;
    market.avg_collateral_price = avg;
    market.last_price_time = now;
    Ok(avg)
}

pub fn borrow(ctx: Context<Borrow>, amount: u64) -> Result<()> {
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
    let avg_price = update_collateral_average(market, col_band.mid, clock.unix_timestamp)?;

    // Value collateral conservatively using the lower of current low-band and market TWAP;
    // value debt using the high end of its band so any uncertainty works against the borrower.
    let effective_col_price = col_band.low.min(avg_price);
    let new_debt = ctx
        .accounts
        .position
        .debt
        .checked_add(amount)
        .ok_or(MarketError::MathOverflow)?;

    let collateral_val = value(
        ctx.accounts.position.collateral,
        effective_col_price,
        ctx.accounts.collateral_mint.decimals,
    )?;
    let debt_val = value(
        new_debt,
        debt_band.high,
        ctx.accounts.debt_mint.decimals,
    )?;

    require!(
        debt_val
            .checked_mul(BPS as u128)
            .ok_or(MarketError::MathOverflow)?
            <= collateral_val
                .checked_mul(market.borrow_ltv_bps as u128)
                .ok_or(MarketError::MathOverflow)?,
        MarketError::ExceedsBorrowLtv
    );

    ctx.accounts.position.debt = new_debt;

    let seeds: &[&[u8]] = &[b"market", &[market.bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.debt_vault.to_account_info(),
                mint: ctx.accounts.debt_mint.to_account_info(),
                to: ctx.accounts.owner_debt.to_account_info(),
                authority: ctx.accounts.market.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        ctx.accounts.debt_mint.decimals,
    )
}
