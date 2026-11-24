import { assert } from "chai";
import { Clock } from "solana-bankrun";
import { Keypair, LAMPORTS_PER_SOL, SystemProgram, Transaction } from "@solana/web3.js";

import { expectReverted, getTime, oneToken, processTransaction, tokenBalance } from "../helpers";
import { MultisigFixture, setupMultisig } from "./fixture";
import {
  approve,
  cancelProposal,
  createProposal,
  execute,
  treasuryTransfer,
} from "./actions";

describe("06-proposal-anyone-can-execute", () => {
  let env: MultisigFixture;

  beforeEach(async () => {
    env = await setupMultisig();
  });

  it("executes an approved transfer from the treasury with the vault PDA signing", async () => {
    const [admin, bob] = env.members;
    const bobTokens = env.memberTokens(bob.publicKey);
    const transfer = treasuryTransfer(env, bobTokens, oneToken().muln(50));

    // Freshly generated non-member wallet distinct from every member
    const freshStranger = Keypair.generate();
    const memberKeys = new Set(env.members.map((m) => m.publicKey.toBase58()));
    assert.isFalse(memberKeys.has(freshStranger.publicKey.toBase58()));

    await processTransaction(
      env.context,
      new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: env.payer.publicKey,
          toPubkey: freshStranger.publicKey,
          lamports: 2 * LAMPORTS_PER_SOL,
        }),
      ),
      [env.payer],
    );

    await createProposal(env, 1, bob, transfer);
    await approve(env, 1, admin);
    await approve(env, 1, bob);

    const before = await tokenBalance(env.context, bobTokens);
    await execute(env, 1, freshStranger, transfer);
    const after = await tokenBalance(env.context, bobTokens);

    assert.equal((after - before).toString(), oneToken().muln(50).toString());
    const proposal = await env.program.account.proposal.fetch(env.proposal(1));
    assert.isTrue(proposal.executed);
  });

  it("enforces threshold, one-time execution, exact account list, and cancel/expiry cleanup", async () => {
    const [admin, bob] = env.members;
    const bobTokens = env.memberTokens(bob.publicKey);
    const transfer = treasuryTransfer(env, bobTokens, oneToken().muln(25));

    // Freshly generated non-member wallet distinct from every member
    const freshStranger = Keypair.generate();
    const memberKeys = new Set(env.members.map((m) => m.publicKey.toBase58()));
    assert.isFalse(memberKeys.has(freshStranger.publicKey.toBase58()));

    await processTransaction(
      env.context,
      new Transaction().add(
        SystemProgram.transfer({
          fromPubkey: env.payer.publicKey,
          toPubkey: freshStranger.publicKey,
          lamports: 2 * LAMPORTS_PER_SOL,
        }),
      ),
      [env.payer],
    );

    await createProposal(env, 2, bob, transfer, 120);
    await approve(env, 2, admin);

    // Below threshold (1 of 2): refused
    await expectReverted(
      { revertedWith: { message: "NotApproved" } },
      execute(env, 2, freshStranger, transfer),
    );

    // Reach threshold (2 of 2)
    await approve(env, 2, bob);

    // Changed account list (wrong recipient) refused
    const wrongTransfer = treasuryTransfer(
      env,
      env.memberTokens(env.members[2].publicKey),
      oneToken().muln(25),
    );
    await expectReverted(
      { revertedWith: { message: "AccountMismatch" } },
      execute(env, 2, freshStranger, transfer, [
        ...wrongTransfer.keys.map((k) => ({
          pubkey: k.pubkey,
          isSigner: false,
          isWritable: k.isWritable,
        })),
        { pubkey: transfer.programId, isSigner: false, isWritable: false },
      ]),
    );

    // Fresh non-member stranger executes approved proposal
    await execute(env, 2, freshStranger, transfer);

    // Second execution of the same proposal is refused
    await expectReverted(
      { revertedWith: { message: "AlreadyExecuted" } },
      execute(env, 2, freshStranger, transfer),
    );

    // Proposal 3: proposer cancels before expiry and recovers rent
    const transfer3 = treasuryTransfer(env, bobTokens, oneToken().muln(10));
    await createProposal(env, 3, bob, transfer3, 300);
    const prop3Addr = env.proposal(3);
    assert.isNotNull(await env.context.banksClient.getAccount(prop3Addr));

    await cancelProposal(env, 3, bob, bob.publicKey);
    assert.isNull(await env.context.banksClient.getAccount(prop3Addr));

    // Proposal 4: expires and anyone (including stranger) can close it
    await createProposal(env, 4, bob, transfer3, 10);
    const prop4Addr = env.proposal(4);
    const now = await getTime(env.context);
    const clock = await env.context.banksClient.getClock();
    await env.context.setClock(
      new Clock(
        clock.slot + 20n,
        clock.epochStartTimestamp,
        clock.epoch,
        clock.leaderScheduleEpoch,
        now + 15n,
      ),
    );

    const randomCloser = Keypair.generate();
    await cancelProposal(env, 4, freshStranger, randomCloser.publicKey);
    assert.isNull(await env.context.banksClient.getAccount(prop4Addr));
  });
});
