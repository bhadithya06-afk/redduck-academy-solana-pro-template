import { assert } from "chai";
import { Clock } from "solana-bankrun";
import { Keypair } from "@solana/web3.js";

import { expectReverted, getTime, oneToken, timeTravel, tokenBalance } from "../helpers";
import { RoundFixture, setupRounds } from "./fixture";
import { join, openRound, settle } from "./actions";

describe("09-write-the-exit", () => {
  let env: RoundFixture;

  beforeEach(async () => {
    env = await setupRounds();
  });

  it("pays the whole pot to the winner the executor names", async () => {
    const [alice, bob] = env.players;
    const deadline = (await getTime(env.context)) + 60n;
    await openRound(env, 1, deadline);
    await join(env, 1, alice, oneToken().muln(100));
    await join(env, 1, bob, oneToken().muln(50));

    await timeTravel(env.context, deadline);
    const before = await tokenBalance(env.context, bob.tokens);
    await settle(env, 1, bob);
    const after = await tokenBalance(env.context, bob.tokens);

    assert.equal((after - before).toString(), oneToken().muln(150).toString());
    const round = await env.program.account.round.fetch(env.round(1));
    assert.isTrue(round.settled);
  });

  it("refunds player stake when rollup never settles and enforces timelock on DEX change", async () => {
    const [alice] = env.players;
    const deadline = (await getTime(env.context)) + 60n;
    await openRound(env, 2, deadline);
    await join(env, 2, alice, oneToken().muln(75));

    const beforeRefund = await tokenBalance(env.context, alice.tokens);

    // Advance past deadline, but before 300s refund delay: refused
    await timeTravel(env.context, deadline + 10n);
    const refundIxEarly = await env.program.methods
      .refund()
      .accountsPartial({
        player: alice.keypair.publicKey,
        round: env.round(2),
        entry: env.entry(env.round(2), alice.keypair.publicKey),
        mint: env.mint,
        playerTokens: alice.tokens,
      })
      .instruction();
    await expectReverted(
      { revertedWith: { message: "RefundTooEarly" } },
      (async () => {
        const { sendIxs } = await import("../helpers");
        await sendIxs(env.context, [alice.keypair], refundIxEarly);
      })(),
    );

    // Advance past 300s grace window: player recovers stake without executor
    await timeTravel(env.context, deadline + 301n);
    const { sendIxs } = await import("../helpers");
    const refundIxOk = await env.program.methods
      .refund()
      .accountsPartial({
        player: alice.keypair.publicKey,
        round: env.round(2),
        entry: env.entry(env.round(2), alice.keypair.publicKey),
        mint: env.mint,
        playerTokens: alice.tokens,
      })
      .instruction();
    await sendIxs(env.context, [alice.keypair], refundIxOk);

    const afterRefund = await tokenBalance(env.context, alice.tokens);
    assert.equal(
      (afterRefund - beforeRefund).toString(),
      oneToken().muln(75).toString(),
    );

    // Propose replacement DEX venue and verify immediate apply is refused before 86400s delay
    const newDex = Keypair.generate().publicKey;
    const proposeIx = await env.program.methods
      .proposeDex(newDex)
      .accountsPartial({ admin: env.payer.publicKey })
      .instruction();
    await sendIxs(env.context, [env.payer], proposeIx);

    const applyIx = await env.program.methods.applyDex().instruction();
    await expectReverted(
      { revertedWith: { message: "DexChangeNotReady" } },
      sendIxs(env.context, [env.payer], applyIx),
    );

    // Advance clock by 86400s and apply replacement DEX venue
    const clock = await env.context.banksClient.getClock();
    await env.context.setClock(
      new Clock(
        clock.slot + 50n,
        clock.epochStartTimestamp,
        clock.epoch,
        clock.leaderScheduleEpoch,
        clock.unixTimestamp + 86_405n,
      ),
    );
    await sendIxs(env.context, [env.payer], applyIx);
    const cfg = await env.program.account.config.fetch(env.config);
    assert.equal(cfg.dexProgram.toBase58(), newDex.toBase58());
  });
});
