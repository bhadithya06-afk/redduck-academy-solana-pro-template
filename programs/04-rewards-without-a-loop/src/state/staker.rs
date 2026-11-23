use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Staker {
    pub owner: Pubkey,
    pub stake: u64,
    /// Snapshot of `pool.reward_per_token` at the staker's last settlement.
    pub reward_debt: u128,
    /// Rewards settled up to `reward_debt` and awaiting transfer on `claim`.
    pub accrued: u64,
    pub bump: u8,
}
