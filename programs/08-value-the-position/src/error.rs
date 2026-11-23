use anchor_lang::prelude::*;

#[error_code]
pub enum MarketError {
    #[msg("Math overflow")]
    MathOverflow,
    #[msg("The borrow limit must be below the liquidation limit")]
    InvalidParams,
    #[msg("The price feed is stale")]
    StalePrice,
    #[msg("The price feed account does not match the expected feed")]
    WrongFeed,
    #[msg("Invalid oracle price")]
    InvalidPrice,
    #[msg("Borrow would exceed maximum borrow LTV")]
    ExceedsBorrowLtv,
    #[msg("Position is not eligible for liquidation")]
    NotLiquidatable,
    #[msg("Nothing to liquidate")]
    NothingToLiquidate,
}
