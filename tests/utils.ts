import { transactions } from "@metaplex/js";
import * as anchor from "@project-serum/anchor";
import { BN } from "@project-serum/anchor";
import {
  Keypair,
  PublicKey,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionSignature,
} from "@solana/web3.js";

export async function getWalletPubkey(
  user: PublicKey,
  primaryWallet: PublicKey,
  programId: PublicKey,
): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress([
    anchor.utils.bytes.utf8.encode("wallet"),
    user.toBytes(),
    primaryWallet.toBytes(),
  ], programId))[0];
}

export async function getNftConfigPubkey(
  primaryWallet: PublicKey,
  programId: PublicKey,
): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress([
    anchor.utils.bytes.utf8.encode("config"),
    primaryWallet.toBytes(),
  ], programId))[0];
}

export async function getChillMetadataPubkey(
  nftMint: PublicKey,
  programId: PublicKey,
): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress([
    anchor.utils.bytes.utf8.encode("chill-metadata"),
    nftMint.toBytes(),
  ], programId))[0];
}

export async function airdrop(
  address: PublicKey,
  balance: number,
) {
  await anchor.getProvider().connection.confirmTransaction(
    await anchor.getProvider().connection.requestAirdrop(
      address,
      balance,
    ),
  );
}

export async function transferLamports(
  from: Keypair,
  to: PublicKey,
  lamports: number,
): Promise<TransactionSignature> {
  const transaction = new Transaction().add(
    anchor.web3.SystemProgram.transfer({
      fromPubkey: from.publicKey,
      toPubkey: to,
      lamports,
    }),
  );

  const connection = anchor.getProvider().connection;
  const signature = await anchor.web3.sendAndConfirmTransaction(
    connection,
    transaction,
    [from],
  );

  return signature;
}

export async function transferTokens(
  authority: Keypair,
  source: PublicKey,
  destination: PublicKey,
  amount: number,
): Promise<TransactionSignature> {
  const tokenProgram = anchor.Spl.token();
  return await tokenProgram.methods.transfer(new BN(amount)).accounts({
    source,
    destination,
    authority: authority.publicKey,
  }).signers([authority]).rpc();
}

export async function keypairWithSol(): Promise<Keypair> {
  const keypair = Keypair.generate();
  await airdrop(keypair.publicKey, 1_000_000_000);
  return keypair;
}

export async function createMint(
  authority: PublicKey,
  decimals: number,
): Promise<PublicKey> {
  const mint = Keypair.generate();
  const tokenProgram = anchor.Spl.token();
  await tokenProgram.methods.initializeMint(decimals, authority, null)
    .accounts(
      {
        mint: mint.publicKey,
        rent: SYSVAR_RENT_PUBKEY,
      },
    )
    .preInstructions([await tokenProgram.account.mint.createInstruction(mint)])
    .signers([mint])
    .rpc();

  return mint.publicKey;
}

export async function createTokenAccount(
  owner: PublicKey,
  mint: PublicKey,
): Promise<PublicKey> {
  const tokenAccount = Keypair.generate();
  const tokenProgram = anchor.Spl.token();

  await tokenProgram.methods.initializeAccount().accounts({
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

export async function tokenBalance(
  address: PublicKey,
): Promise<number> {
  const connection = anchor.getProvider().connection;
  const info = await connection.getTokenAccountBalance(address);
  return parseInt(info.value.amount);
}

export async function mintTokens(
  authoirity: Keypair,
  mint: PublicKey,
  tokenAccount: PublicKey,
  amount: number,
) {
  const tokenProgram = anchor.Spl.token();

  await tokenProgram.methods.mintTo(new BN(amount)).accounts({
    mint,
    authority: authoirity.publicKey,
    to: tokenAccount,
  })
    .signers([authoirity])
    .rpc();
}
