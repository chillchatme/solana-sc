import * as anchor from "@project-serum/anchor";
import { BN } from "@project-serum/anchor";
import {
  ASSOCIATED_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@project-serum/anchor/dist/cjs/utils/token";
import * as BufferLayout from "@solana/buffer-layout";
import {
  Keypair,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";

export const requestComputeUnitsInstruction = (
  units: number,
  additionalFee: number
): TransactionInstruction => {
  const programId = new PublicKey(
    "ComputeBudget111111111111111111111111111111"
  );

  const layout = BufferLayout.struct<{
    instruction: number;
    units: number;
    additionalFee: number;
  }>([
    BufferLayout.u8("instruction"),
    BufferLayout.u32("units"),
    BufferLayout.u32("additionalFee"),
  ]);

  const data = Buffer.alloc(layout.span);
  layout.encode({ instruction: 0, units, additionalFee }, data);
  return new TransactionInstruction({
    data,
    keys: [],
    programId,
  });
};

export async function getAssociatedTokenAddress(
  owner: PublicKey,
  mint: PublicKey
): Promise<PublicKey> {
  return (
    await PublicKey.findProgramAddress(
      [owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
      ASSOCIATED_PROGRAM_ID
    )
  )[0];
}

export async function airdrop(address: PublicKey, balance: number) {
  await anchor
    .getProvider()
    .connection.confirmTransaction(
      await anchor.getProvider().connection.requestAirdrop(address, balance)
    );
}

export async function transferLamports(
  from: Keypair,
  to: PublicKey,
  lamports: number
): Promise<TransactionSignature> {
  const transaction = new Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: from.publicKey,
      toPubkey: to,
      lamports,
    })
  );

  const connection = anchor.getProvider().connection;
  const signature = await anchor.web3.sendAndConfirmTransaction(
    connection,
    transaction,
    [from]
  );

  return signature;
}

export async function transferTokens(
  authority: Keypair,
  source: PublicKey,
  destination: PublicKey,
  amount: number
): Promise<TransactionSignature> {
  const tokenProgram = anchor.Spl.token();
  return await tokenProgram.methods
    .transfer(new BN(amount))
    .accounts({
      source,
      destination,
      authority: authority.publicKey,
    })
    .signers([authority])
    .rpc();
}

export async function keypairWithSol(): Promise<Keypair> {
  const keypair = Keypair.generate();
  await airdrop(keypair.publicKey, 1_000_000_000);
  return keypair;
}

export async function createMint(
  authority: PublicKey,
  decimals: number
): Promise<PublicKey> {
  const mint = Keypair.generate();
  const tokenProgram = anchor.Spl.token();
  await tokenProgram.methods
    .initializeMint(decimals, authority, null)
    .accounts({
      mint: mint.publicKey,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .preInstructions([await tokenProgram.account.mint.createInstruction(mint)])
    .signers([mint])
    .rpc();

  return mint.publicKey;
}

export async function createTokenAccount(
  owner: PublicKey,
  mint: PublicKey
): Promise<PublicKey> {
  const tokenAccount = Keypair.generate();
  const tokenProgram = anchor.Spl.token();

  await tokenProgram.methods
    .initializeAccount()
    .accounts({
      account: tokenAccount.publicKey,
      authority: owner,
      mint,
      rent: SYSVAR_RENT_PUBKEY,
    })
    .preInstructions([
      await tokenProgram.account.token.createInstruction(tokenAccount),
    ])
    .signers([tokenAccount])
    .rpc();

  return tokenAccount.publicKey;
}

export async function tokenBalance(address: PublicKey): Promise<number> {
  const connection = anchor.getProvider().connection;
  const info = await connection.getTokenAccountBalance(address);
  return parseInt(info.value.amount);
}

export async function mintTokens(
  authoirity: Keypair,
  mint: PublicKey,
  tokenAccount: PublicKey,
  amount: number
) {
  const tokenProgram = anchor.Spl.token();

  await tokenProgram.methods
    .mintTo(new BN(amount))
    .accounts({
      mint,
      authority: authoirity.publicKey,
      to: tokenAccount,
    })
    .signers([authoirity])
    .rpc();
}

export function randomNumber(max?: number): number {
  if (max == null) {
    max = 1_000_000;
  }
  return Math.floor(Math.random() * (max + 1));
}

export async function getCurrentTime(): Promise<number> {
  const provider = anchor.getProvider();
  const connection = provider.connection;

  const slot = await connection.getSlot();
  return await connection.getBlockTime(slot);
}
