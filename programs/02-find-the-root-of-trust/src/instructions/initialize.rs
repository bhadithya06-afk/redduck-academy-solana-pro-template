use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::{Config, VaultError, MAX_KEYS};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(init, payer = admin, space = 8 + Config::INIT_SPACE, seeds = [b"config"], bump)]
    pub config: Account<'info, Config>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = admin,
        seeds = [b"vault"],
        bump,
        token::mint = mint,
        token::authority = config,
    )]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn initialize(ctx: Context<Initialize>, allowed_keys: Vec<Pubkey>) -> Result<()> {
    require!(allowed_keys.len() <= MAX_KEYS, VaultError::TooManyKeys);
    ctx.accounts.config.set_inner(Config {
        admin: ctx.accounts.admin.key(),
        mint: ctx.accounts.mint.key(),
        bump: ctx.bumps.config,
        allowed_keys,
    });
    Ok(())
}
