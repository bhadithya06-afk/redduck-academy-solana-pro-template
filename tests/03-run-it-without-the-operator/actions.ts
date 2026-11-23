import { BN } from "@coral-xyz/anchor";
import {
  Ed25519Program,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { sendIxs } from "../helpers";
import { RewardsFixture } from "./fixture";

export const encodePricePayload = (price: BN, publishTime: number | bigint): Buffer => {
  const buf = Buffer.alloc(16);
  buf.writeBigUInt64LE(BigInt(price.toString()), 0);
  buf.writeBigInt64LE(BigInt(publishTime), 8);
  return buf;
};

export const createOraclePriceIx = (
  oracle: Keypair,
  price: BN,
  publishTime: number | bigint,
) =>
  Ed25519Program.createInstructionWithPrivateKey({
    privateKey: oracle.secretKey,
    message: encodePricePayload(price, publishTime),
  });

export const accrue = async (
  env: RewardsFixture,
  owner: PublicKey,
  amount: BN,
) => {
  const ix = await env.program.methods
    .accrue(amount)
    .accountsPartial({ admin: env.payer.publicKey, owner })
    .instruction();
  await sendIxs(env.context, [env.payer], ix);
};

export const claim = async (
  env: RewardsFixture,
  owner: PublicKey,
  ownerTokens: PublicKey,
  caller: Keypair = env.stranger,
  callerTokens: PublicKey = env.strangerTokens,
) => {
  const ix = await env.program.methods
    .claim()
    .accountsPartial({
      caller: caller.publicKey,
      user: env.userAccount(owner),
      mint: env.mint,
      userTokens: ownerTokens,
      callerTokens,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
  await sendIxs(env.context, [caller], ix);
};

export const setPriceWithOracle = async (
  env: RewardsFixture,
  price: BN,
  publishTime: number | bigint,
  oracleSigner: Keypair = env.oracle,
  caller: Keypair = env.stranger,
) => {
  const edIx = createOraclePriceIx(oracleSigner, price, publishTime);
  const ix = await env.program.methods
    .setPrice()
    .accountsPartial({
      caller: caller.publicKey,
      instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
    })
    .instruction();
  await sendIxs(env.context, [caller], edIx, ix);
};

export const buyWithSignedPrice = async (
  env: RewardsFixture,
  buyer: Keypair,
  buyerTokens: PublicKey,
  lamports: BN,
  price: BN,
  publishTime: number | bigint,
  oracleSigner: Keypair = env.oracle,
) => {
  const edIx = createOraclePriceIx(oracleSigner, price, publishTime);
  const ix = await env.program.methods
    .buy(lamports)
    .accountsPartial({
      buyer: buyer.publicKey,
      mint: env.mint,
      buyerTokens,
      instructions: SYSVAR_INSTRUCTIONS_PUBKEY,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
  await sendIxs(env.context, [buyer], edIx, ix);
};
