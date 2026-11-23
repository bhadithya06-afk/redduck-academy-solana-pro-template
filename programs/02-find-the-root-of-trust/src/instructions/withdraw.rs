use anchor_lang::prelude::*;
use anchor_lang::solana_program::sysvar::instructions::{
    self as ix_sysvar, load_current_index_checked,
};
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{load_instruction_unchecked, parse_ed25519, Config, VaultError};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub caller: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(address = config.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint)]
    pub recipient_tokens: Account<'info, TokenAccount>,
    /// CHECK: verified against the canonical Instructions sysvar address before reading.
    #[account(address = ix_sysvar::ID)]
    pub instructions: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

/// The message an oracle signs for one withdrawal: vault, recipient token account, amount.
pub fn withdrawal_message(vault: &Pubkey, recipient_tokens: &Pubkey, amount: u64) -> Vec<u8> {
    let mut message = Vec::with_capacity(72);
    message.extend_from_slice(vault.as_ref());
    message.extend_from_slice(recipient_tokens.as_ref());
    message.extend_from_slice(&amount.to_le_bytes());
    message
}

pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    let current_ix = load_current_index_checked(&ctx.accounts.instructions)? as usize;
    require!(current_ix > 0, VaultError::NotEd25519);

    let signature_check = load_instruction_unchecked(current_ix - 1, &ctx.accounts.instructions)?;
    let signed = parse_ed25519(&signature_check)?;
    require!(
        ctx.accounts.config.allowed_keys.contains(&signed.pubkey),
        VaultError::KeyNotAllowed
    );
    let expected = withdrawal_message(
        &ctx.accounts.vault.key(),
        &ctx.accounts.recipient_tokens.key(),
        amount,
    );
    require!(signed.message == expected, VaultError::MessageMismatch);

    let seeds: &[&[u8]] = &[b"config", &[ctx.accounts.config.bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.recipient_tokens.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            &[seeds],
        ),
        amount,
        ctx.accounts.mint.decimals,
    )
}
