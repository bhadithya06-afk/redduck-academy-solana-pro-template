use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Market {
    pub admin: Pubkey,
    pub collateral_mint: Pubkey,
    pub debt_mint: Pubkey,
    pub collateral_feed: [u8; 32],
    pub debt_feed: [u8; 32],
    pub max_price_age: u64,
    pub borrow_ltv_bps: u16,
    pub liquidation_ltv_bps: u16,
    /// Market-maintained time-weighted average of collateral price (6 decimals, USD).
    pub avg_collateral_price: u64,
    /// Timestamp when `avg_collateral_price` was last updated.
    pub last_price_time: i64,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct MarketParams {
    pub collateral_feed: [u8; 32],
    pub debt_feed: [u8; 32],
    pub max_price_age: u64,
    pub borrow_ltv_bps: u16,
    pub liquidation_ltv_bps: u16,
}
