import { BN } from "@coral-xyz/anchor";
import {
  Ed25519Program,
  Keypair,
  PublicKey,
  SYSVAR_INSTRUCTIONS_PUBKEY,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";

import { sendIxs } from "../helpers";
import { VaultFixture } from "./fixture";

/** The message the oracle signs: vault, recipient token account, amount as u64 LE. */
export const withdrawalMessage = (
  vault: PublicKey,
  recipientTokens: PublicKey,
  amount: BN,
) =>
  Buffer.concat([
    vault.toBuffer(),
    recipientTokens.toBuffer(),
    amount.toArrayLike(Buffer, "le", 8),
  ]);

export const withdraw = async (
  env: VaultFixture,
  amount: BN,
  recipientTokens: PublicKey,
  signer: Keypair = env.oracle,
  instructions: PublicKey = SYSVAR_INSTRUCTIONS_PUBKEY,
  signedMessage: Buffer = withdrawalMessage(env.vault, recipientTokens, amount),
) => {
  const signatureCheck = Ed25519Program.createInstructionWithPrivateKey({
    privateKey: signer.secretKey,
    message: signedMessage,
  });
  const ix = await env.program.methods
    .withdraw(amount)
    .accountsPartial({
      caller: env.payer.publicKey,
      mint: env.mint,
      recipientTokens,
      instructions,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .instruction();
  await sendIxs(env.context, [env.payer], signatureCheck, ix);
};
