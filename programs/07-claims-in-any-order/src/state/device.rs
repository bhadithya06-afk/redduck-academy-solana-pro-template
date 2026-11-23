use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Device {
    pub owner: Pubkey,
    /// Cumulative reward total already paid out to this device (monotonically non-decreasing).
    pub paid: u64,
    pub bump: u8,
}
