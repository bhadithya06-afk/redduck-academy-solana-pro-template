use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::{
    self as ix_sysvar, load_current_index_checked, load_instruction_unchecked,
};
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{parse_ed25519, ClaimError, Config, Device};

/// Pays the difference between the oracle-signed cumulative `total` and `device.paid`,
/// advancing `device.paid` monotonically to `total`. Order-independent and idempotent without nonces.
#[derive(Accounts)]
pub struct Claim<'info> {
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"device", device.owner.as_ref()], bump = device.bump)]
    pub device: Account<'info, Device>,
    #[account(address = config.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = device.owner)]
    pub owner_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    /// CHECK: verified against the canonical instructions sysvar address before reading.
    #[account(address = ix_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

/// The message the oracle signs for a cumulative claim: device pubkey and cumulative total (u64 LE).
pub fn claim_message(device: &Pubkey, total: u64) -> Vec<u8> {
    let mut message = Vec::with_capacity(40);
    message.extend_from_slice(device.as_ref());
    message.extend_from_slice(&total.to_le_bytes());
    message
}

pub fn claim(ctx: Context<Claim>, total: u64) -> Result<()> {
    // Compare the sysvar account address before reading any instruction data from it.
    require_keys_eq!(
        ctx.accounts.instructions.key(),
        ix_sysvar::ID,
        ClaimError::NotEd25519
    );

    let current_ix = load_current_index_checked(&ctx.accounts.instructions)? as usize;
    require!(current_ix > 0, ClaimError::NotEd25519);

    let signature_check = load_instruction_unchecked(current_ix - 1, &ctx.accounts.instructions)?;
    let signed = parse_ed25519(&signature_check)?;
    require_keys_eq!(
        signed.pubkey,
        ctx.accounts.config.oracle,
        ClaimError::WrongSigner
    );
    let expected = claim_message(&ctx.accounts.device.key(), total);
    require!(signed.message == expected, ClaimError::MessageMismatch);

    let device = &mut ctx.accounts.device;
    if total <= device.paid {
        return Ok(());
    }

    let payout = total
        .checked_sub(device.paid)
        .ok_or(ClaimError::MathOverflow)?;
    device.paid = total;

    let seeds: &[&[u8]] = &[b"config", &[ctx.accounts.config.bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.owner_tokens.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            &[seeds],
        ),
        payout,
        ctx.accounts.mint.decimals,
    )
}
