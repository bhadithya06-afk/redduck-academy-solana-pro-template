use anchor_lang::prelude::*;

use crate::MAX_KEYS;

#[account]
#[derive(InitSpace)]
pub struct Config {
    pub admin: Pubkey,
    pub mint: Pubkey,
    pub bump: u8,
    #[max_len(MAX_KEYS)]
    pub allowed_keys: Vec<Pubkey>,
}
