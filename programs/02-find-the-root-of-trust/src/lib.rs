use anchor_lang::prelude::*;

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
        instructions::deposit(ctx, amount)
    }

    /// Withdraws tokens from the `["vault"]` PDA only after verifying the canonical Instructions sysvar,
    /// an Ed25519 signature from a key in `config.allowed_keys` (loaded from the `["config"]` PDA),
    /// and an exact match on the withdrawal message.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw(ctx, amount)
    }
}
