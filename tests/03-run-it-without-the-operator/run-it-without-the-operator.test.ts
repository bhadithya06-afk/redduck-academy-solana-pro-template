import { assert } from "chai";
import { Clock } from "solana-bankrun";
import { LAMPORTS_PER_SOL } from "@solana/web3.js";

import { expectReverted, oneToken, toBN, tokenBalance } from "../helpers";
import { MAX_PRICE_AGE, RewardsFixture, setupRewards } from "./fixture";
import { accrue, buyWithSignedPrice, claim } from "./actions";

describe("03-run-it-without-the-operator", () => {
  let env: RewardsFixture;

  beforeEach(async () => {
    env = await setupRewards();
  });

  it("lets a stranger claim for a user, keeps the reward with the user, pays caller fee, and trades with no bot", async () => {
    await accrue(env, env.user.publicKey, oneToken().muln(100));

    const userBefore = await tokenBalance(env.context, env.userTokens);
    const strangerBefore = await tokenBalance(env.context, env.strangerTokens);

    // Stranger (neither admin nor bot) claims on behalf of user
    await claim(env, env.user.publicKey, env.userTokens, env.stranger, env.strangerTokens);

    const userAfter = await tokenBalance(env.context, env.userTokens);
    const strangerAfter = await tokenBalance(env.context, env.strangerTokens);

    // 100 tokens -> 1% fee (1 token, below 10 token cap) to stranger, 99 tokens to user
    assert.equal((strangerAfter - strangerBefore).toString(), oneToken().muln(1).toString());
    assert.equal((userAfter - userBefore).toString(), oneToken().muln(99).toString());

    // Re-claiming immediately fails (no double payment)
    await expectReverted(
      { revertedWith: { message: "NothingToPay" } },
      claim(env, env.user.publicKey, env.userTokens, env.stranger, env.strangerTokens),
    );

    // Trade with a signed oracle price update pulled in the same tx, with no bot running
    const clock = await env.context.banksClient.getClock();
    const price = toBN(LAMPORTS_PER_SOL / 100); // 0.01 SOL per whole token
    await buyWithSignedPrice(
      env,
      env.stranger,
      env.strangerTokens,
      toBN(LAMPORTS_PER_SOL / 10),
      price,
      clock.unixTimestamp,
    );

    const strangerFinal = await tokenBalance(env.context, env.strangerTokens);
    assert.equal(
      (strangerFinal - strangerAfter).toString(),
      oneToken().muln(10).toString(),
    );
  });

  it("rejects a price update older than max_price_age", async () => {
    const clock = await env.context.banksClient.getClock();
    const staleTime = Number(clock.unixTimestamp) - (MAX_PRICE_AGE + 1);
    const price = toBN(LAMPORTS_PER_SOL / 100);

    await expectReverted(
      { revertedWith: { message: "StalePrice" } },
      buyWithSignedPrice(
        env,
        env.stranger,
        env.strangerTokens,
        toBN(LAMPORTS_PER_SOL / 10),
        price,
        staleTime,
      ),
    );

    // Advance clock past max_price_age and confirm freshness check uses current clock
    await env.context.setClock(
      new Clock(
        clock.slot + 10n,
        clock.epochStartTimestamp,
        clock.epoch,
        clock.leaderScheduleEpoch,
        clock.unixTimestamp + BigInt(MAX_PRICE_AGE + 5),
      ),
    );
  });
});
