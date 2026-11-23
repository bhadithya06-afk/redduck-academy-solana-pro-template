import { assert } from "chai";
import {
  Ed25519Program,
  Keypair,
  SystemProgram,
  TransactionInstruction,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { expectReverted, oneToken, sendIxs, tokenBalance } from "../helpers";
import { VaultFixture, setupVault } from "./fixture";
import { withdraw, withdrawalMessage } from "./actions";

/** Serializes a single instruction into the Instructions sysvar binary layout. */
const encodeFakeSysvar = (ix: TransactionInstruction): Buffer => {
  const header = Buffer.alloc(4);
  header.writeUInt16LE(1, 0); // instruction count = 1
  header.writeUInt16LE(4, 2); // offset of instruction 0 = 4

  const accountsLen = Buffer.alloc(2);
  accountsLen.writeUInt16LE(ix.keys.length, 0);

  const accountMetas = ix.keys.map((meta) => {
    const flags = (meta.isSigner ? 1 : 0) | (meta.isWritable ? 2 : 0);
    return Buffer.concat([Buffer.from([flags]), meta.pubkey.toBuffer()]);
  });

  const dataLen = Buffer.alloc(2);
  dataLen.writeUInt16LE(ix.data.length, 0);

  return Buffer.concat([
    header,
    accountsLen,
    ...accountMetas,
    ix.programId.toBuffer(),
    dataLen,
    ix.data,
  ]);
};

describe("02-find-the-root-of-trust", () => {
  let env: VaultFixture;

  beforeEach(async () => {
    env = await setupVault();
  });

  it("pays out a withdrawal the oracle signed", async () => {
    const before = await tokenBalance(env.context, env.payerTokens);
    await withdraw(env, oneToken().muln(100), env.payerTokens);
    const after = await tokenBalance(env.context, env.payerTokens);

    assert.equal((after - before).toString(), oneToken().muln(100).toString());
    assert.equal(
      (await tokenBalance(env.context, env.vault)).toString(),
      oneToken().muln(900).toString(),
    );
  });

  it("rejects a fake instructions sysvar account", async () => {
    const rogue = Keypair.generate();
    const amount = oneToken().muln(100);
    const forgedEd25519Ix = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: rogue.secretKey,
      message: withdrawalMessage(env.vault, env.payerTokens, amount),
    });

    const fakeSysvar = Keypair.generate();
    env.context.setAccount(fakeSysvar.publicKey, {
      lamports: 1_000_000_000,
      data: encodeFakeSysvar(forgedEd25519Ix),
      owner: SystemProgram.programId,
      executable: false,
    });

    const ix = await env.program.methods
      .withdraw(amount)
      .accountsPartial({
        caller: env.payer.publicKey,
        mint: env.mint,
        recipientTokens: env.payerTokens,
        instructions: fakeSysvar.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .instruction();

    await expectReverted(
      { revertedWith: { message: "ConstraintAddress" } },
      sendIxs(env.context, [env.payer], ix),
    );
  });

  it("rejects a signature from a key not in config.allowed_keys", async () => {
    const unauthorizedOracle = Keypair.generate();
    await expectReverted(
      { revertedWith: { message: "KeyNotAllowed" } },
      withdraw(
        env,
        oneToken().muln(100),
        env.payerTokens,
        unauthorizedOracle,
      ),
    );
  });

  it("rejects a signed message that does not match the withdrawal", async () => {
    const signedForSmallerAmount = withdrawalMessage(
      env.vault,
      env.payerTokens,
      oneToken().muln(10),
    );
    await expectReverted(
      { revertedWith: { message: "MessageMismatch" } },
      withdraw(
        env,
        oneToken().muln(100),
        env.payerTokens,
        env.oracle,
        undefined,
        signedForSmallerAmount,
      ),
    );
  });
});
