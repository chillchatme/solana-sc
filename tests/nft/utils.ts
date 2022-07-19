import * as anchor from "@project-serum/anchor";
import { BN } from "@project-serum/anchor";
import { Keypair, PublicKey } from "@solana/web3.js";
import { randomNumber } from "../utils";

export async function getNftConfigPubkey(
  chillMint: PublicKey,
  programId: PublicKey
): Promise<PublicKey> {
  return (
    await PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("config"), chillMint.toBytes()],
      programId
    )
  )[0];
}

export async function getChillMetadataPubkey(
  nftMint: PublicKey,
  programId: PublicKey
): Promise<PublicKey> {
  return (
    await PublicKey.findProgramAddress(
      [anchor.utils.bytes.utf8.encode("chill-metadata"), nftMint.toBytes()],
      programId
    )
  )[0];
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

export async function getCurrentTime(): Promise<number> {
  const provider = anchor.getProvider();
  const connection = provider.connection;

  const slot = await connection.getSlot();
  return await connection.getBlockTime(slot);
}
