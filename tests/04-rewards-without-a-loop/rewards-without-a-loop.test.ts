import { assert } from "chai";

import { oneToken, tokenBalance } from "../helpers";
import { PoolFixture, setupPool } from "./fixture";
import { addRewards, claim, stake, unstake } from "./actions";

describe("04-rewards-without-a-loop", () => {
  let env: PoolFixture;

  beforeEach(async () => {
    env = await setupPool();
  });

  it("splits a reward between three stakers in proportion to their stake", async () => {
    const [a, b, c] = env.stakers;
    await stake(env, a, oneToken().muln(100));
    await stake(env, b, oneToken().muln(300));
    await stake(env, c, oneToken().muln(600));
    const before = await Promise.all(
      env.stakers.map((s) => tokenBalance(env.context, s.tokens)),
    );

    await addRewards(env, oneToken().muln(100));
    await claim(env, a);
    await claim(env, b);
    await claim(env, c);

    const after = await Promise.all(
      env.stakers.map((s) => tokenBalance(env.context, s.tokens)),
    );
    assert.equal(
      (after[0] - before[0]).toString(),
      oneToken().muln(10).toString(),
    );
    assert.equal(
      (after[1] - before[1]).toString(),
      oneToken().muln(30).toString(),
    );
    assert.equal(
      (after[2] - before[2]).toString(),
      oneToken().muln(60).toString(),
    );
  });

  it("handles rewards between actions, non-altering deposits, reverse claim order, and a late claimer", async () => {
    const [a, b, c] = env.stakers;

    // Initial stakes: A = 200, B = 200, C = 600 (total = 1000)
    await stake(env, a, oneToken().muln(200));
    await stake(env, b, oneToken().muln(200));
    await stake(env, c, oneToken().muln(600));

    const balBefore = await Promise.all(
      env.stakers.map((s) => tokenBalance(env.context, s.tokens)),
    );

    // Reward 1: 100 tokens -> A gets 20, B gets 20, C gets 60
    await addRewards(env, oneToken().muln(100));

    // A deposits 200 more tokens (to 400): settling first preserves A's 20 earned from Reward 1
    await stake(env, a, oneToken().muln(200));

    // B unstakes 100 tokens (to 100): settling first preserves B's 20 earned from Reward 1
    await unstake(env, b, oneToken().muln(100));

    // New total staked: A = 400, B = 100, C = 600 (total = 1100)
    // Reward 2: 110 tokens -> 0.1 per staked token:
    //   A earns +40 (total 60)
    //   B earns +10 (total 30)
    //   C earns +60 (total 120)
    await addRewards(env, oneToken().muln(110));

    // Claim in reverse/arbitrary order: B first, A second, C last (never claimed earlier)
    await claim(env, b);
    await claim(env, a);
    await claim(env, c);

    const balAfter = await Promise.all(
      env.stakers.map((s) => tokenBalance(env.context, s.tokens)),
    );

    // Net token deltas (rewards claimed minus net stake changes)
    const netRewardA = balAfter[0] - balBefore[0] + BigInt(oneToken().muln(400).toString());
    const netRewardB = balAfter[1] - balBefore[1] + BigInt(oneToken().muln(100).toString());
    const netRewardC = balAfter[2] - balBefore[2] + BigInt(oneToken().muln(600).toString());

    assert.equal(netRewardA.toString(), oneToken().muln(60).toString());
    assert.equal(netRewardB.toString(), oneToken().muln(30).toString());
    assert.equal(netRewardC.toString(), oneToken().muln(120).toString());
  });
});
