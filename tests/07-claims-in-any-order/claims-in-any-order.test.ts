import { assert } from "chai";

import { oneToken, tokenBalance } from "../helpers";
import { ClaimsFixture, setupClaims } from "./fixture";
import { claim } from "./actions";

describe("07-claims-in-any-order", () => {
  let env: ClaimsFixture;

  beforeEach(async () => {
    env = await setupClaims();
  });

  it("pays a reward the oracle signed", async () => {
    const total = oneToken().muln(10);
    await claim(env, total);
    assert.equal(
      (await tokenBalance(env.context, env.ownerTokens)).toString(),
      total.toString(),
    );
    const device = await env.program.account.device.fetch(env.device);
    assert.equal(device.paid.toString(), total.toString());
  });

  it("converges to the same paid amount and balance regardless of order or duplicates", async () => {
    const olderTotal = oneToken().muln(10);
    const newerTotal = oneToken().muln(25);

    // Run A: older total first, then newer total, then duplicates of both
    await claim(env, olderTotal);
    await claim(env, newerTotal);
    await claim(env, olderTotal);
    await claim(env, newerTotal);

    const balanceAfterA = await tokenBalance(env.context, env.ownerTokens);
    const deviceAfterA = await env.program.account.device.fetch(env.device);
    assert.equal(balanceAfterA.toString(), newerTotal.toString());
    assert.equal(deviceAfterA.paid.toString(), newerTotal.toString());

    // Run B on a fresh pool state: newer total first, then older total, then duplicates
    const envB = await setupClaims();
    await claim(envB, newerTotal);
    await claim(envB, olderTotal);
    await claim(envB, newerTotal);
    await claim(envB, olderTotal);

    const balanceAfterB = await tokenBalance(envB.context, envB.ownerTokens);
    const deviceAfterB = await envB.program.account.device.fetch(envB.device);
    assert.equal(balanceAfterB.toString(), balanceAfterA.toString());
    assert.equal(deviceAfterB.paid.toString(), deviceAfterA.paid.toString());
  });
});
