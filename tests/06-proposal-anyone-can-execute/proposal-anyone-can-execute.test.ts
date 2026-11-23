import { assert } from "chai";
import { Clock } from "solana-bankrun";
import { Keypair } from "@solana/web3.js";

import { expectReverted, getTime, oneToken, tokenBalance } from "../helpers";
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

    await createProposal(env, 1, bob, transfer);
    await approve(env, 1, admin);
    await approve(env, 1, bob);

    const before = await tokenBalance(env.context, bobTokens);
    await execute(env, 1, env.stranger, transfer);
    const after = await tokenBalance(env.context, bobTokens);

    assert.equal((after - before).toString(), oneToken().muln(50).toString());
    const proposal = await env.program.account.proposal.fetch(env.proposal(1));
    assert.isTrue(proposal.executed);
  });

  it("enforces threshold, one-time execution, exact account list, and cancel/expiry cleanup", async () => {
    const [admin, bob] = env.members;
    const bobTokens = env.memberTokens(bob.publicKey);
    const transfer = treasuryTransfer(env, bobTokens, oneToken().muln(25));

    await createProposal(env, 2, bob, transfer, 120);
    await approve(env, 2, admin);

    // Below threshold (1 of 2): refused
    await expectReverted(
      { revertedWith: { message: "NotApproved" } },
      execute(env, 2, env.stranger, transfer),
    );

    // Reach threshold (2 of 2)
    await approve(env, 2, bob);

    // Changed account list (wrong recipient) refused
    const wrongTransfer = treasuryTransfer(env, env.memberTokens(env.members[2].publicKey), oneToken().muln(25));
    await expectReverted(
      { revertedWith: { message: "AccountMismatch" } },
      execute(env, 2, env.stranger, transfer, [
        ...wrongTransfer.keys.map((k) => ({
          pubkey: k.pubkey,
          isSigner: false,
          isWritable: k.isWritable,
        })),
        { pubkey: transfer.programId, isSigner: false, isWritable: false },
      ]),
    );

    // Non-member stranger executes approved proposal
    await execute(env, 2, env.stranger, transfer);

    // Second execution of the same proposal is refused
    await expectReverted(
      { revertedWith: { message: "AlreadyExecuted" } },
      execute(env, 2, env.stranger, transfer),
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
    await cancelProposal(env, 4, env.stranger, randomCloser.publicKey);
    assert.isNull(await env.context.banksClient.getAccount(prop4Addr));
  });
});
