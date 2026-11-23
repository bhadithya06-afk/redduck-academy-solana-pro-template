import { assert } from "chai";
import { Clock } from "solana-bankrun";

import { expectReverted, oneToken, tokenBalance } from "../helpers";
import { MarketFixture, setupMarket } from "./fixture";
import { borrow, deposit, liquidate } from "./actions";

describe("08-value-the-position", () => {
  let env: MarketFixture;

  beforeEach(async () => {
    env = await setupMarket();
  });

  it("borrows against deposited collateral", async () => {
    await env.setPrices([150, 1], [1, 0.001]);
    await deposit(env, oneToken().muln(10));

    const before = await tokenBalance(env.context, env.payerDebt);
    await borrow(env, oneToken().muln(1_000));
    const after = await tokenBalance(env.context, env.payerDebt);

    assert.equal((after - before).toString(), oneToken().muln(1_000).toString());
    const position = await env.program.account.position.fetch(
      env.position(env.payer.publicKey),
    );
    assert.equal(position.collateral.toString(), oneToken().muln(10).toString());
    assert.equal(position.debt.toString(), oneToken().muln(1_000).toString());
  });

  it("refuses stale prices, wide bands, short price spikes, and liquidates above liquidation LTV", async () => {
    // 1. Stale price refused
    await env.setPrices([100, 0], [1, 0], 120);
    await deposit(env, oneToken().muln(10));
    await expectReverted(
      { revertedWith: { message: "StalePrice" } },
      borrow(env, oneToken().muln(500)),
    );

    // 2. Establish baseline price (100) with a borrow inside borrow LTV (75% of 1000 = 750)
    await env.setPrices([100, 0], [1, 0], 0);
    await borrow(env, oneToken().muln(750));

    // 3. Wide confidence band lowers collateral low price so a larger borrow is refused
    await env.setPrices([100, 40], [1, 0], 0);
    await expectReverted(
      { revertedWith: { message: "ExceedsBorrowLtv" } },
      borrow(env, oneToken().muln(50)),
    );

    // 4. Short price spike: collateral jumps to 500, but TWAP has not lasted across window
    await env.setPrices([500, 0], [1, 0], 0);
    await expectReverted(
      { revertedWith: { message: "ExceedsBorrowLtv" } },
      borrow(env, oneToken().muln(2_000)),
    );

    // 5. Position between borrow LTV (75%) and liquidation LTV (85%):
    // 10 collateral * 80 = 800; debt = 640 is 80% LTV -> above 75% borrow, below 85% liquidation
    const freshEnv = await setupMarket();
    await freshEnv.setPrices([100, 0], [1, 0], 0);
    await deposit(freshEnv, oneToken().muln(10));
    await borrow(freshEnv, oneToken().muln(640));
    await freshEnv.setPrices([80, 0], [1, 0], 0);
    await expectReverted(
      { revertedWith: { message: "NotLiquidatable" } },
      liquidate(freshEnv, freshEnv.payer.publicKey),
    );

    // 6. Advance time and drop collateral price so position crosses liquidation LTV (>85%)
    const clock = await freshEnv.context.banksClient.getClock();
    await freshEnv.context.setClock(
      new Clock(
        clock.slot + 10n,
        clock.epochStartTimestamp,
        clock.epoch,
        clock.leaderScheduleEpoch,
        clock.unixTimestamp + 10n,
      ),
    );
    await freshEnv.setPrices([70, 0], [1, 0], 0);
    await liquidate(freshEnv, freshEnv.payer.publicKey);

    const posAfterLiq = await freshEnv.program.account.position.fetch(
      freshEnv.position(freshEnv.payer.publicKey),
    );
    assert.equal(posAfterLiq.debt.toString(), "0");
    assert.equal(posAfterLiq.collateral.toString(), "0");
  });
});
