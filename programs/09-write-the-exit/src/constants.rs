use anchor_lang::prelude::*;

pub const DEX_PROGRAM_ID: Pubkey = pubkey!("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");

/// Grace delay after round `deadline` before players can self-refund an unsettled round (5 min).
pub const REFUND_DELAY_SECONDS: i64 = 300;

/// Timelock delay before a proposed replacement swap venue takes effect (24 hours).
pub const DEX_CHANGE_DELAY_SECONDS: i64 = 86_400;
