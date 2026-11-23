use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

pub mod ed25519;
pub mod error;
pub mod instructions;
pub mod state;
pub mod sysvar;

pub use ed25519::*;
pub use error::*;
pub use instructions::*;
pub use state::*;
pub use sysvar::*;

declare_id!("GfxMGdUbRtXDtG8fx2HGLoHhDjpa9syxZPvtefw7yksq");

#[derive(Accounts)]
pub struct Deposit<'info> {
    pub depositor: Signer<'info>,
    /// Single root of trust: derived from canonical `["config"]` PDA seeds, never caller-chosen.
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(address = config.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = depositor)]
    pub depositor_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump, token::mint = mint, token::authority = config)]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[program]
pub mod oracle_vault {
    use super::*;

    /// Initializes the canonical `["config"]` PDA (storing `admin`, `mint`, `bump`, and `allowed_keys`)
    /// and the `["vault"]` token account whose authority is that `Config` PDA.
    pub fn initialize(ctx: Context<Initialize>, allowed_keys: Vec<Pubkey>) -> Result<()> {
        instructions::initialize(ctx, allowed_keys)
    }

    /// Deposits tokens into the `["vault"]` PDA; verifies `config` via `seeds = [b"config"], bump = config.bump`
    /// and `mint` via `address = config.mint` so deposits share the exact same `Config` PDA root.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        require!(
            ctx.accounts.config.allowed_keys.len() <= MAX_KEYS,
            VaultError::TooManyKeys
        );
        transfer_checked(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                TransferChecked {
                    from: ctx.accounts.depositor_tokens.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.depositor.to_account_info(),
                },
            ),
            amount,
            ctx.accounts.mint.decimals,
        )
    }

    /// Withdraws tokens from the `["vault"]` PDA only after verifying the canonical Instructions sysvar,
    /// an Ed25519 signature from a key in `config.allowed_keys` (loaded from the `["config"]` PDA),
    /// and an exact match on the withdrawal message.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)
    }
}
