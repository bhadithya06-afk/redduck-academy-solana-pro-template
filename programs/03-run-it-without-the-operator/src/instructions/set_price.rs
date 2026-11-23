use anchor_lang::prelude::*;
use anchor_lang::solana_program::ed25519_program;
use anchor_lang::solana_program::sysvar::instructions::{
    self as ix_sysvar, load_current_index_checked, load_instruction_unchecked,
};

use crate::{parse_ed25519, Config, RewardsError};

/// Encodes the signed oracle price payload: `price` (u64 LE) || `publish_time` (i64 LE).
pub fn encode_price_message(price: u64, publish_time: i64) -> Vec<u8> {
    let mut msg = Vec::with_capacity(16);
    msg.extend_from_slice(&price.to_le_bytes());
    msg.extend_from_slice(&publish_time.to_le_bytes());
    msg
}

pub fn decode_price_message(message: &[u8]) -> Result<(u64, i64)> {
    require!(message.len() == 16, RewardsError::InvalidPricePayload);
    let price = u64::from_le_bytes(
        message[0..8]
            .try_into()
            .map_err(|_| RewardsError::InvalidPricePayload)?,
    );
    let publish_time = i64::from_le_bytes(
        message[8..16]
            .try_into()
            .map_err(|_| RewardsError::InvalidPricePayload)?,
    );
    require!(price > 0, RewardsError::InvalidPricePayload);
    Ok((price, publish_time))
}

/// Verifies an oracle-signed Ed25519 instruction immediately preceding this instruction
/// in the canonical Instructions sysvar, enforces `max_price_age`, and updates `config`.
pub fn apply_signed_price(
    config: &mut Config,
    instructions_acc: &AccountInfo,
    now: i64,
) -> Result<(u64, i64)> {
    require_keys_eq!(
        instructions_acc.key(),
        ix_sysvar::ID,
        RewardsError::InvalidInstructionsSysvar
    );

    let current_ix = load_current_index_checked(instructions_acc)? as usize;
    require!(current_ix > 0, RewardsError::NotEd25519);

    let ed_ix = load_instruction_unchecked(current_ix - 1, instructions_acc)?;
    require_keys_eq!(ed_ix.program_id, ed25519_program::ID, RewardsError::NotEd25519);

    let signed = parse_ed25519(&ed_ix)?;
    require_keys_eq!(signed.pubkey, config.oracle, RewardsError::InvalidOracle);

    let (price, publish_time) = decode_price_message(&signed.message)?;
    require!(
        publish_time <= now && now.saturating_sub(publish_time) <= config.max_price_age,
        RewardsError::StalePrice
    );

    config.price = price;
    config.price_time = publish_time;
    Ok((price, publish_time))
}

/// Permissionless pull-price instruction: anyone can submit an oracle-signed Ed25519
/// price update in the same transaction without needing any bot or admin key.
#[derive(Accounts)]
pub struct SetPrice<'info> {
    pub caller: Signer<'info>,
    #[account(mut, seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    /// CHECK: verified against the canonical Instructions sysvar address before reading.
    #[account(address = ix_sysvar::ID @ RewardsError::InvalidInstructionsSysvar)]
    pub instructions: UncheckedAccount<'info>,
}

pub fn set_price(ctx: Context<SetPrice>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    apply_signed_price(&mut ctx.accounts.config, &ctx.accounts.instructions.to_account_info(), now)?;
    Ok(())
}
