use anchor_lang::prelude::*;

pub const MAX_KEYS: usize = 4;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
    #[max_len(MAX_KEYS)]
    pub allowed_keys: Vec<Pubkey>,
}
