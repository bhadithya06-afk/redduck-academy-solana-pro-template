# Solution

## Which end of the band each value uses

- **Collateral in `borrow` and `liquidate`:** Uses the **low end** of the Pyth confidence band (`price - conf`, scaled to 6 decimals), and in `borrow` takes `col_band.low.min(avg_collateral_price)`. Valuing collateral at its lower bound avoids overestimating collateral value when oracle uncertainty is wide.
- **Debt in `borrow` and `liquidate`:** Uses the **high end** of the Pyth confidence band (`price + conf`, scaled to 6 decimals). Valuing debt at its upper bound ensures liabilities are never understated when confidence intervals widen.

## The two lines

- `borrow_ltv_bps` (`7500` = `75%`) is stricter than `liquidation_ltv_bps` (`8500` = `85%`) to create a safety buffer between opening/expanding a loan and liquidating a distressed one. Without hysteresis between the two thresholds, normal price noise near the borrowing boundary would cause positions to oscillate in and out of liquidation.
- `borrow` enforces `debt_val * BPS <= collateral_val * borrow_ltv_bps` against the TWAP-bounded collateral value (`low.min(avg)`), whereas `liquidate` checks `debt_val * BPS > collateral_val * liquidation_ltv_bps` using spot band valuation (`col_band.low` and `debt_band.high`) without new-borrow limits. Liquidations must never inherit borrow-only limits because liquidators need to clear unhealthy debt promptly even during volatile market conditions.

## The value from the market's own state

- The `Market` account stores `avg_collateral_price: u64` and `last_price_time: i64`. Whenever `borrow` or `liquidate` reads a fresh Pyth collateral price, `update_collateral_average` blends `avg_collateral_price` and `col_band.mid` over `dt.min(AVERAGE_WINDOW_SECONDS)` (`300` seconds).
- `borrow` uses `avg_collateral_price` (via `col_band.low.min(avg_collateral_price)`) to prevent a sudden upward price spike from immediately unlocking oversized borrows before the price has persisted.
- If the price genuinely falls and stays low, `avg_collateral_price` trends downward over time toward the lower price, and `liquidate` relies on `col_band.low` against `liquidation_ltv_bps` so undercollateralized positions remain liquidatable.

## A price that is too old

- `read_band` checks `publish_time <= clock.unix_timestamp` and `clock.unix_timestamp - publish_time <= max_age` (using `market.max_price_age`), returning `MarketError::StalePrice` if the update is too old.
- Any caller or price publisher/keeper can fix it by posting a fresh oracle price update to the Pyth price account and retrying the transaction.
