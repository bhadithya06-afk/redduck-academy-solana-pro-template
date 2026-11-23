# Solution

## The two numbers

- **Pool number (`pool.reward_per_token: u128`):** The cumulative reward per staked token across the lifetime of the pool, scaled by `REWARD_PRECISION` (`1_000_000_000_000` / $10^{12}$). When `add_rewards(amount)` runs, it adds `(amount * REWARD_PRECISION) / pool.total_staked` to `pool.reward_per_token` in $O(1)$ time without iterating over stakers.
- **Staker number (`staker.reward_debt: u128`):** The snapshot of `pool.reward_per_token` recorded at the staker's most recent settlement (`stake`, `unstake`, or `claim`).
- **Formula for newly owed rewards:**
  $$\text{newly\_earned} = \left\lfloor \frac{\text{staker.stake} \times (\text{pool.reward\_per\_token} - \text{staker.reward\_debt})}{\text{REWARD\_PRECISION}} \right\rfloor$$
  This is added to `staker.accrued`, and `staker.reward_debt` is updated to `pool.reward_per_token`.

## Settle first

`stake` and `unstake` must always call `settle_staker` **before** updating `staker.stake` so rewards earned on the prior stake are locked in using the prior `staker.stake`.

For example, suppose a staker holds `200` tokens (`reward_debt = 0`), a reward raises `reward_per_token` by `0.1` per token (so the staker has earned `20` tokens), and the staker then deposits `200` more tokens to reach `400` total stake:
- **Correct order (settle first, then stake):** `settle_staker` computes `200 * 0.1 = 20` tokens on the old `200` stake, records `accrued = 20`, sets `reward_debt = 0.1`, and then sets `stake = 400`. Future rewards of `0.1` per token accrue only on the new `400` stake (`+40`), giving `20 + 40 = 60` total.
- **Wrong order (stake first, then settle):** If `stake` increased `staker.stake` to `400` before settling, `settle_staker` would multiply the *new* `400` stake by the `0.1` delta and credit `40` tokens instead of `20`—minting `20` unearned tokens out of thin air. Similarly, unstaking before settling would underpay the staker on the stake they held when the reward arrived.

## The staker nobody touches

A staker who never claims while multiple rewards arrive and other stakers stake, unstake, or claim simply stays at their earlier `staker.reward_debt` checkpoint with their `staker.stake` unchanged. Because `add_rewards` only increments `pool.reward_per_token` and other stakers' actions only modify their own `Staker` PDAs and `pool.total_staked`, the untouched staker's `reward_per_token - staker.reward_debt` captures the full cumulative delta across all intervening rewards. When that staker eventually calls `stake`, `unstake`, or `claim`, `settle_staker` computes their exact share in $O(1)$ time.

## What lazy loading does not cover

Lazy reward-per-token accounting is pull-based / lazy for reward credit calculation, so it cannot cover **solvency / vault funding or external side-effects that must happen eagerly at a specific moment**—such as ensuring the `vault` actually holds enough tokens to cover accrued claims (or executing a cross-program transfer/hook at deposit or withdrawal time). In a pool like this, a reward distribution (`add_rewards`) or funding/rebalancing step still requires a caller or paid keeper/executor whenever real token liquidity must be deposited into `vault` or external state must be updated before claims can be paid out.
