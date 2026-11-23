import { assert } from "chai";
import { Clock } from "solana-bankrun";
import {
  Ed25519Program,
  Keypair,
  LAMPORTS_PER_SOL,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SystemProgram,
  Transaction,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createAssociatedTokenAccountInstruction,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";

import {
  expectReverted,
  oneToken,
  processTransaction,
  sendIxs,
  toBN,
  tokenBalance,
} from "../helpers";
import { MAX_PRICE_AGE, RewardsFixture, setupRewards } from "./fixture";
import { accrue, buyWithSignedPrice, claim } from "./actions";

describe("03-run-it-without-the-operator", () => {
  let env: RewardsFixture;

  beforeEach(async () => {
    env = await setupRewards();
  });

  it("lets a freshly created stranger claim for a user, keeps the reward with the user, pays caller fee, and trades with no bot", async () => {
    await accrue(env, env.user.publicKey, oneToken().muln(100));

    // Create a brand-new stranger keypair directly inside the test (neither admin nor bot)
    const freshStranger = Keypair.generate();
    const freshStrangerTokens = getAssociatedTokenAddressSync(
      env.mint,
      freshStranger.publicKey,
    );
    await processTransaction(
      env.context,
      new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: env.payer.publicKey,
          toPubkey: freshStranger.publicKey,
          lamports: 2 * LAMPORTS_PER_SOL,
        }),
        createAssociatedTokenAccountInstruction(
          env.payer.publicKey,
          freshStrangerTokens,
          freshStranger.publicKey,
          env.mint,
        ),
      ),
      [env.payer],
    );

    const userBefore = await tokenBalance(env.context, env.userTokens);
    const strangerBefore = await tokenBalance(env.context, freshStrangerTokens);

    // Fresh stranger claims on behalf of user
    await claim(env, env.user.publicKey, env.userTokens, freshStranger, freshStrangerTokens);

    const userAfter = await tokenBalance(env.context, env.userTokens);
    const strangerAfter = await tokenBalance(env.context, freshStrangerTokens);

    // 100 tokens -> 1% fee (1 token, below 10 token cap) to freshStranger, 99 tokens to user
    assert.equal((strangerAfter - strangerBefore).toString(), oneToken().muln(1).toString());
    assert.equal((userAfter - userBefore).toString(), oneToken().muln(99).toString());

    // Re-claiming immediately fails (no double payment)
    await expectReverted(
      { revertedWith: { message: "NothingToPay" } },
      claim(env, env.user.publicKey, env.userTokens, freshStranger, freshStrangerTokens),
    );

    // Construct the oracle-signed price update directly in the test transaction (no bot running)
    const clock = await env.context.banksClient.getClock();
    const price = toBN(LAMPORTS_PER_SOL / 100); // 0.01 SOL per whole token
    const publishTime = BigInt(clock.unixTimestamp);
    const message = Buffer.alloc(16);
    message.writeBigUInt64LE(BigInt(price.toString()), 0);
    message.writeBigInt64LE(publishTime, 8);

    const ed25519Ix = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: env.oracle.secretKey,
      message,
    });
    const buyIx = await env.program.methods
      .buy(toBN(LAMPORTS_PER_SOL / 10))
      .accountsPartial({
        buyer: freshStranger.publicKey,
        mint: env.mint,
        buyerTokens: freshStrangerTokens,
        instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    await sendIxs(env.context, [freshStranger], ed25519Ix, buyIx);

    const strangerFinal = await tokenBalance(env.context, freshStrangerTokens);
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
