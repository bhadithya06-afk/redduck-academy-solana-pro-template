use anchor_lang::prelude::*;
use pyth_solana_receiver_sdk::price_update::PriceUpdateV2;

use crate::{MarketError, PRICE_DECIMALS};

pub struct Band {
    pub low: u64,
    pub mid: u64,
    pub high: u64,
}

pub fn read_band(
    update: &PriceUpdateV2,
    feed_id: &[u8; 32],
    max_age: u64,
    clock: &Clock,
) -> Result<Band> {
    let msg = &update.price_message;
    require!(msg.feed_id == *feed_id, MarketError::WrongFeed);
    require!(msg.publish_time <= clock.unix_timestamp, MarketError::StalePrice);
    let age = clock
        .unix_timestamp
        .checked_sub(msg.publish_time)
        .ok_or(MarketError::StalePrice)?;
    require!(age >= 0 && (age as u64) <= max_age, MarketError::StalePrice);
    require!(msg.price > 0, MarketError::InvalidPrice);

    let price_u128 = msg.price as i128;
    let conf_i128 = msg.conf as i128;
    let low_raw = price_u128.saturating_sub(conf_i128).max(1) as u64;
    let mid_raw = msg.price as u64;
    let high_raw = price_u128.saturating_add(conf_i128).max(1) as u64;

    Ok(Band {
        low: scale(low_raw, msg.exponent)?,
        mid: scale(mid_raw, msg.exponent)?,
        high: scale(high_raw, msg.exponent)?,
    })
}

pub fn value(amount: u64, price: u64, decimals: u8) -> Result<u128> {
    Ok((amount as u128)
        .checked_mul(price as u128)
        .ok_or(MarketError::MathOverflow)?
        / 10u128.pow(decimals as u32))
}

pub fn scale(value: u64, exponent: i32) -> Result<u64> {
    let target = -(PRICE_DECIMALS as i32);
    let value = value as u128;
    let scaled = if exponent < target {
        value / 10u128.pow((target - exponent) as u32)
    } else {
        value
            .checked_mul(10u128.pow((exponent - target) as u32))
            .ok_or(MarketError::MathOverflow)?
    };
    u64::try_from(scaled).map_err(|_| MarketError::MathOverflow.into())
}
