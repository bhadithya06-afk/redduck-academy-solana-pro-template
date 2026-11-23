use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    /// Oracle public key whose Ed25519 signatures authenticate pulled price updates.
    pub oracle: Pubkey,
    pub mint: Pubkey,
    /// Lamports per whole token (latest verified price).
    pub price: u64,
    pub price_time: i64,
    pub max_price_age: i64,
    pub bump: u8,
}
