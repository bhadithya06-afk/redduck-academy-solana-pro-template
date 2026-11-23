use anchor_lang::prelude::*;
use anchor_spl::token::{transfer_checked, Mint, Token, TokenAccount, TransferChecked};

use crate::{Book, BookError, CELLS, NONE};

#[derive(Accounts)]
pub struct PlaceOrder<'info> {
    pub owner: Signer<'info>,
    #[account(mut, seeds = [b"book"], bump = book.load()?.bump)]
    pub book: AccountLoader<'info, Book>,
    #[account(address = book.load()?.mint)]
    pub mint: Account<'info, Mint>,
    #[account(mut, token::mint = mint, token::authority = owner)]
    pub owner_tokens: Account<'info, TokenAccount>,
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

/// Validates a caller-supplied `hint` in O(1) time by checking only the relevant
/// neighbor cell(s) without scanning the book.
fn validate_hint(book: &Book, price: u64, hint: Option<u16>) -> Result<u16> {
    match hint {
        None => {
            let head = book.head;
            if head != NONE {
                require!((head as usize) < CELLS, BookError::BadHint);
                let head_cell = book.cells[head as usize];
                require!(head_cell.alive == 1, BookError::BadHint);
                require!(price > head_cell.price, BookError::BadHint);
            }
            Ok(NONE)
        }
        Some(h) => {
            require!((h as usize) < CELLS, BookError::BadHint);
            let prev_cell = book.cells[h as usize];
            require!(prev_cell.alive == 1, BookError::BadHint);
            require!(prev_cell.price >= price, BookError::BadHint);

            let next = prev_cell.next;
            if next != NONE {
                require!((next as usize) < CELLS, BookError::BadHint);
                let next_cell = book.cells[next as usize];
                require!(next_cell.alive == 1, BookError::BadHint);
                require!(price > next_cell.price, BookError::BadHint);
            }
            Ok(h)
        }
    }
}

/// Places an order after `hint` (or at the head when `hint` is `None`), checking only
/// the local neighbor cells and locking `size` tokens in the vault.
pub fn place_order(
    ctx: Context<PlaceOrder>,
    price: u64,
    size: u64,
    hint: Option<u16>,
) -> Result<()> {
    require!(size > 0, BookError::BadHint);

    let (cell, after) = {
        let book = ctx.accounts.book.load()?;
        let free_cell = book.free_head;
        require!(free_cell != NONE, BookError::BookFull);
        let validated_after = validate_hint(&book, price, hint)?;
        (free_cell, validated_after)
    };

    transfer_checked(
        CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            TransferChecked {
                from: ctx.accounts.owner_tokens.to_account_info(),
                mint: ctx.accounts.mint.to_account_info(),
                to: ctx.accounts.vault.to_account_info(),
                authority: ctx.accounts.owner.to_account_info(),
            },
        ),
        size,
        ctx.accounts.mint.decimals,
    )?;

    let mut book = ctx.accounts.book.load_mut()?;
    let next_free = book.cells[cell as usize].next;
    book.free_head = next_free;

    link(
        &mut book,
        cell,
        after,
        ctx.accounts.owner.key(),
        price,
        size,
    );
    Ok(())
}

/// Links `cell` into the list right after `after` (or at the head when `after` is NONE).
pub fn link(book: &mut Book, cell: u16, after: u16, owner: Pubkey, price: u64, size: u64) {
    let next = if after == NONE {
        book.head
    } else {
        book.cells[after as usize].next
    };
    let c = &mut book.cells[cell as usize];
    c.owner = owner;
    c.price = price;
    c.size = size;
    c.prev = after;
    c.next = next;
    c.alive = 1;
    if after == NONE {
        book.head = cell;
    } else {
        book.cells[after as usize].next = cell;
    }
    if next != NONE {
        book.cells[next as usize].prev = cell;
    }
}
