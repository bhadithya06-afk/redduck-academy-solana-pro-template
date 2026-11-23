use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{Config, Entry, Round, RoundError, REFUND_DELAY_SECONDS};

#[derive(Accounts)]
pub struct Settle<'info> {
    pub executor: Signer<'info>,
    #[account(seeds = [b"config"], bump = config.bump, has_one = executor)]
    pub config: Account<'info, Config>,
    #[account(mut, seeds = [b"round", round.id.to_le_bytes().as_ref()], bump = round.bump)]
    pub round: Account<'info, Round>,
    #[account(has_one = round)]
    pub winner_entry: Account<'info, Entry>,
    #[account(address = config.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = winner_entry.player)]
    pub winner_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn settle(ctx: Context<Settle>) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let refund_opens_at = ctx
        .accounts
        .round
        .deadline
        .checked_add(REFUND_DELAY_SECONDS)
        .ok_or(RoundError::MathOverflow)?;

    let round = &mut ctx.accounts.round;
    require!(!round.settled, RoundError::AlreadySettled);
    require!(now >= round.deadline, RoundError::RoundNotEnded);
    // Disallow settlement once the refund window opens so `settle` and `refund` cannot overlap.
    require!(now < refund_opens_at, RoundError::RefundWindowOpen);

    let payout = round.pot;
    round.settled = true;
    round.pot = 0;

    let seeds: &[&[u8]] = &[b"config", &[ctx.accounts.config.bump]];
    transfer_checked(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.vault.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.winner_tokens.to_account_info(),
                authority: ctx.accounts.config.to_account_info(),
            },
            &[seeds],
        ),
        payout,
        ctx.accounts.mint.decimals,
    )
}
