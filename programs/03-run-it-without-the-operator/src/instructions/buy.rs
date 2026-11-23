use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions as ix_sysvar;
use anchor_lang::system_program::{transfer, Transfer};
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{apply_signed_price, Config, RewardsError};

/// A user buys tokens from the vault with lamports using a fresh oracle-signed price
/// pulled in the same transaction (or an already-fresh price stored on `config`).
#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(address = config.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = buyer)]
    pub buyer_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    /// CHECK: verified against the canonical Instructions sysvar address when pulling a price.
    #[account(address = ix_sysvar::ID @ RewardsError::InvalidInstructionsSysvar)]
    pub instructions: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn buy(ctx: Context<Buy>, lamports: u64) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let instructions_info = ctx.accounts.instructions.to_account_info();

    // If a preceding Ed25519 instruction is present in the transaction, pull and validate
    // the oracle-signed price update; otherwise validate the currently stored price.
    let current_idx = ix_sysvar::load_current_index_checked(&instructions_info)? as usize;
    if current_idx > 0 {
        let prev_ix = ix_sysvar::load_instruction_unchecked(current_idx - 1, &instructions_info)?;
        if prev_ix.program_id == anchor_lang::solana_program::ed25519_program::ID {
            apply_signed_price(&mut ctx.accounts.config, &instructions_info, now)?;
        }
    }

    let config = &ctx.accounts.config;
    require!(
        config.price > 0
            && config.price_time <= now
            && now.saturating_sub(config.price_time) <= config.max_price_age,
        RewardsError::StalePrice
    );

    let tokens = (lamports as u128)
        .checked_mul(10u128.pow(ctx.accounts.mint.decimals as u32))
        .ok_or(RewardsError::MathOverflow)?
        / config.price as u128;
    let tokens = u64::try_from(tokens).map_err(|_| RewardsError::MathOverflow)?;

    transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.config.to_account_info(),
            },
        ),
        lamports,
    )?;

    let seeds: &[&[u8]] = &[b"config", &[ctx.accounts.config.bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.buyer_tokens.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            &[seeds],
        ),
        tokens,
        ctx.accounts.mint.decimals,
    )
}
