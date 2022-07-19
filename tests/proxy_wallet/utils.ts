import * as anchor from "@project-serum/anchor";
import { PublicKey } from "@solana/web3.js";

export async function getWalletPubkey(
  user: PublicKey,
  primaryWallet: PublicKey,
  programId: PublicKey
): Promise<PublicKey> {
  return (
    await PublicKey.findProgramAddress(
      [
        anchor.utils.bytes.utf8.encode("wallet"),
        user.toBytes(),
        primaryWallet.toBytes(),
      ],
      programId
    )
  )[0];
}
