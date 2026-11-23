use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub executor: Pubkey,
    pub mint: Pubkey,
    /// Active external swap venue used by `join_with_swap`.
    pub dex_program: Pubkey,
    /// Pending replacement swap venue queued under timelock.
    pub pending_dex_program: Pubkey,
    /// Earliest unix timestamp when `pending_dex_program` can be applied.
    pub dex_change_ready_at: i64,
    pub bump: u8,
}
