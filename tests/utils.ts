import * as anchor from "@project-serum/anchor";
import { BN } from "@project-serum/anchor";
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
  additionalFee: number,
): TransactionInstruction => {
  const programId = new PublicKey(
    "ComputeBudget111111111111111111111111111111",
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
  chillMint: PublicKey,
  programId: PublicKey,
): Promise<PublicKey> {
  return (await PublicKey.findProgramAddress([
    anchor.utils.bytes.utf8.encode("config"),
    chillMint.toBytes(),
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

const nftTypes = [
  "character",
  "pet",
  "emote",
  "tileset",
  "item",
  "world",
] as const;

export const MAX_RECIPIENTS = 3;

export type Fees = { [K in typeof nftTypes[number]]: BN };
export type NftType = Pick<keyof Fees, never>;

export type Recipient = {
  address: PublicKey;
  mintShare: number;
  transactionShare: number;
};

export type NftArgs = {
  name: string;
  symbol: string;
  uri: string;
  fees: number;
};

export function randomNumber(max?: number): number {
  if (max == null) {
    max = 1_000_000;
  }
  return Math.floor(Math.random() * (max + 1));
}

export function randomFees(): Fees {
  const fees = {};
  for (const index in nftTypes) {
    fees[nftTypes[index]] = new BN(randomNumber());
  }
  return fees as Fees;
}

export function feesOf(fees: Fees, nftType: NftType): BN {
  const nftTypeKey = Object.keys(nftType)[0];
  return fees[nftTypeKey];
}

export function randomNftType(): NftType {
  const index = randomNumber(nftTypes.length - 1);
  const nftType = {};
  nftType[nftTypes[index]] = {};
  return nftType;
}

export function randomRecipients(amount?: number): Recipient[] {
  if (amount == null) {
    amount = MAX_RECIPIENTS;
  }

  const recipients: Recipient[] = [];
  let mintShare = 100;
  let transactionShare = 100;

  for (let i = 1; i < amount; i++) {
    const recipient = {
      address: Keypair.generate().publicKey,
      mintShare: randomNumber(mintShare),
      transactionShare: randomNumber(transactionShare),
    };

    mintShare -= recipient.mintShare;
    transactionShare -= recipient.transactionShare;
    recipients.push(recipient);
  }

  const lastRecipient = {
    address: Keypair.generate().publicKey,
    mintShare,
    transactionShare,
  };
  recipients.push(lastRecipient);

  return recipients;
}

export function randomNftArgs(): NftArgs {
  return {
    name: "NAME_" + randomNumber(1000),
    symbol: "SYM_" + randomNumber(1000),
    uri: "https://arweave.org/" + Keypair.generate().publicKey.toString(),
    fees: randomNumber(10000),
  };
}
