import * as assert from "assert";
import * as utils from "../utils";
import * as nftUtils from "./utils";
import * as walletUtils from "../proxy_wallet/utils";
import * as anchor from "@project-serum/anchor";
import { AnchorProvider, BN, Program } from "@project-serum/anchor";
import { ChillNft } from "../../target/types/chill_nft";
import { ChillWallet } from "../../target/types/chill_wallet";
import {
  AccountMeta,
  Connection,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  Transaction,
  TransactionInstruction,
} from "@solana/web3.js";
import NodeWallet from "@project-serum/anchor/dist/cjs/nodewallet";
import { programs } from "@metaplex/js";

describe("NFT | Mint with proxy wallet", () => {
  // Primary wallet will be pay for all transactions
  const primaryWallet = Keypair.generate();
  const wallet = new NodeWallet(primaryWallet);

  const url = process.env.ANCHOR_PROVIDER_URL;
  const commitmentOptions = AnchorProvider.defaultOptions();
  const connection = new Connection(url, commitmentOptions.commitment);
  const provider = new AnchorProvider(connection, wallet, commitmentOptions);
  anchor.setProvider(provider);

  const nftProgram = anchor.workspace.ChillNft as Program<ChillNft>;
  const walletProgram = anchor.workspace.ChillWallet as Program<ChillWallet>;
  const tokenProgram = anchor.Spl.token();

  const user = Keypair.generate();

  let chillMint: PublicKey;

  let primaryWalletChill: PublicKey;
  const initialTokensAmount = 10_000_000_000;

  let proxyWallet: PublicKey;
  let proxyWalletChill: PublicKey;
  const recipientsTokenAccounts: AccountMeta[] = [];

  const fees = nftUtils.randomFees();
  const recipients = nftUtils.randomRecipients();
  let config: PublicKey;

  before(async () => {
    await utils.airdrop(primaryWallet.publicKey, 1_000_000_000);
    await utils.airdrop(user.publicKey, 2_000_000_000);
  });

  it("Create $CHILL and primary wallet token account", async () => {
    const instructions: TransactionInstruction[] = [];

    const chillMintKeypair = Keypair.generate();
    chillMint = chillMintKeypair.publicKey;

    instructions.push(
      await tokenProgram.account.mint.createInstruction(chillMintKeypair)
    );

    instructions.push(
      await tokenProgram.methods
        .initializeMint(9, primaryWallet.publicKey, null)
        .accounts({ mint: chillMint, rent: SYSVAR_RENT_PUBKEY })
        .instruction()
    );

    const primaryWalletChillKeypair = Keypair.generate();
    primaryWalletChill = primaryWalletChillKeypair.publicKey;

    instructions.push(
      await tokenProgram.account.token.createInstruction(
        primaryWalletChillKeypair
      )
    );

    instructions.push(
      await tokenProgram.methods
        .initializeAccount()
        .accounts({
          account: primaryWalletChill,
          authority: primaryWallet.publicKey,
          mint: chillMint,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .instruction()
    );

    const transaction = new Transaction();
    transaction.add(...instructions);

    await provider.sendAndConfirm(transaction, [
      chillMintKeypair,
      primaryWalletChillKeypair,
    ]);

    // Initialize token accounts of treasury recipients for tests
    for (let i = 0; i < recipients.length; i++) {
      const tokenAccount = await utils.createTokenAccount(
        recipients[i].address,
        chillMint
      );

      const accountMeta = {
        pubkey: tokenAccount,
        isSigner: false,
        isWritable: true,
      };

      recipientsTokenAccounts.push(accountMeta);
    }
  });

  it("Create proxy wallet", async () => {
    const instructions: TransactionInstruction[] = [];
    proxyWallet = await walletUtils.getWalletPubkey(
      user.publicKey,
      primaryWallet.publicKey,
      walletProgram.programId
    );

    instructions.push(
      await walletProgram.methods
        .createWallet()
        .accounts({
          primaryWallet: primaryWallet.publicKey,
          payer: user.publicKey,
          user: user.publicKey,
          proxyWallet,
          systemProgram: SystemProgram.programId,
        })
        .instruction()
    );

    instructions.push(
      SystemProgram.transfer({
        fromPubkey: user.publicKey,
        toPubkey: proxyWallet,
        lamports: 1_000_000_000,
      })
    );

    const proxyWalletChillKeypair = Keypair.generate();
    proxyWalletChill = proxyWalletChillKeypair.publicKey;

    const space = tokenProgram.account.token.size;
    const rentCost = await connection.getMinimumBalanceForRentExemption(space);

    // The instruction from `tokenProgram.account.token.createInstructionuses`
    // uses `wallet` as a fee payer, so we create an account in the classic way

    instructions.push(
      SystemProgram.createAccount({
        fromPubkey: user.publicKey,
        newAccountPubkey: proxyWalletChill,
        space,
        lamports: rentCost,
        programId: tokenProgram.programId,
      })
    );

    instructions.push(
      await tokenProgram.methods
        .initializeAccount()
        .accounts({
          account: proxyWalletChill,
          authority: proxyWallet,
          mint: chillMint,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .instruction()
    );

    const transaction = new Transaction();
    transaction.recentBlockhash = (
      await connection.getLatestBlockhash()
    ).blockhash;
    transaction.feePayer = user.publicKey;
    transaction.add(...instructions);

    transaction.partialSign(user);
    transaction.partialSign(proxyWalletChillKeypair);

    const serializedTransaction = transaction.serialize();
    const signature = await connection.sendRawTransaction(
      serializedTransaction
    );

    // `provider.sendAndConfirm` also uses `wallet` as a fee payer, but we want
    // the user to be the payer
    await connection.confirmTransaction(signature);
  });

  it("Mint $CHILL tokens to the proxy wallet", async () => {
    const preBalance = await connection.getBalance(primaryWallet.publicKey);
    let ix = await tokenProgram.methods
      .mintTo(new BN(initialTokensAmount))
      .accounts({
        authority: primaryWallet.publicKey,
        mint: chillMint,
        to: proxyWalletChill,
      })
      .instruction();

    const transaction = new Transaction();
    transaction.feePayer = primaryWallet.publicKey;
    transaction.recentBlockhash = (
      await connection.getLatestBlockhash()
    ).blockhash;

    transaction.add(ix);

    // Calculate fees and withdraw them from the proxy wallet in a signle transaction
    const feeAmount = (
      await connection.getFeeForMessage(transaction.compileMessage())
    ).value;

    ix = await walletProgram.methods
      .withdrawLamports(new BN(feeAmount))
      .accounts({
        authority: primaryWallet.publicKey,
        proxyWallet,
        receiver: primaryWallet.publicKey,
      })
      .instruction();

    transaction.add(ix);
    await provider.sendAndConfirm(transaction);

    const postBalance = await connection.getBalance(primaryWallet.publicKey);

    assert.equal(preBalance, postBalance);
    assert.equal(
      await utils.tokenBalance(proxyWalletChill),
      initialTokensAmount
    );
  });

  it("Initialize NFT program", async () => {
    config = await nftUtils.getNftConfigPubkey(chillMint, nftProgram.programId);
    await nftProgram.methods
      .initialize(fees, recipients)
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        payer: primaryWallet.publicKey,
        config,
        chillMint,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  it("Mint NFT to user with proxy wallet as a payer", async () => {
    const preBalance = await connection.getBalance(primaryWallet.publicKey);

    const instructions: TransactionInstruction[] = [];
    const nftMint = Keypair.generate();
    const nftToken = Keypair.generate();

    instructions.push(
      await tokenProgram.account.mint.createInstruction(nftMint)
    );

    instructions.push(
      await tokenProgram.methods
        .initializeMint(0, primaryWallet.publicKey, null)
        .accounts({ mint: nftMint.publicKey, rent: SYSVAR_RENT_PUBKEY })
        .instruction()
    );

    instructions.push(
      await tokenProgram.account.token.createInstruction(nftToken)
    );

    instructions.push(
      await tokenProgram.methods
        .initializeAccount()
        .accounts({
          account: nftToken.publicKey,
          authority: user.publicKey,
          mint: nftMint.publicKey,
          rent: SYSVAR_RENT_PUBKEY,
        })
        .instruction()
    );

    instructions.push(
      await tokenProgram.methods
        .mintTo(new BN(1))
        .accounts({
          authority: primaryWallet.publicKey,
          mint: nftMint.publicKey,
          to: nftToken.publicKey,
        })
        .instruction()
    );

    const transaction = new Transaction();
    transaction.add(...instructions);

    let simulationResult = await provider.simulate(transaction, null, null, [
      primaryWallet.publicKey,
    ]);

    let simulatedBalance = simulationResult.accounts[0].lamports;
    let withdrawAmount = preBalance - simulatedBalance;

    let withdrawIx = await walletProgram.methods
      .withdrawLamports(new BN(withdrawAmount))
      .accounts({
        authority: primaryWallet.publicKey,
        proxyWallet,
        receiver: primaryWallet.publicKey,
      })
      .instruction();

    transaction.add(withdrawIx);
    await provider.sendAndConfirm(transaction, [nftMint, nftToken]);

    assert.equal(
      await connection.getBalance(primaryWallet.publicKey),
      preBalance
    );

    //
    // WithdrawFt
    //

    const nftType = nftUtils.randomNftType();
    const nftArgs = nftUtils.randomNftArgs();

    const chillAmount = nftUtils.feesOf(fees, nftType);
    const withdrawFtIx = await walletProgram.methods
      .withdrawFt(chillAmount)
      .accounts({
        authority: primaryWallet.publicKey,
        receiverTokenAccount: primaryWalletChill,
        mint: chillMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletChill,
        tokenProgram: tokenProgram.programId,
      })
      .instruction();

    const withdrawTransaction = new Transaction();
    withdrawTransaction.add(withdrawFtIx);

    simulationResult = await provider.simulate(
      withdrawTransaction,
      null,
      null,
      [primaryWallet.publicKey]
    );

    simulatedBalance = simulationResult.accounts[0].lamports;
    withdrawAmount = preBalance - simulatedBalance;

    withdrawIx = await walletProgram.methods
      .withdrawLamports(new BN(withdrawAmount))
      .accounts({
        authority: primaryWallet.publicKey,
        proxyWallet,
        receiver: primaryWallet.publicKey,
      })
      .instruction();

    withdrawTransaction.add(withdrawIx);
    await provider.sendAndConfirm(withdrawTransaction);

    assert.equal(
      chillAmount.toNumber(),
      await utils.tokenBalance(primaryWalletChill)
    );

    assert.equal(
      preBalance,
      await connection.getBalance(primaryWallet.publicKey)
    );

    //
    // Initialize Metadata
    //

    const Metadata = programs.metadata;
    const nftMetadata = await Metadata.Metadata.getPDA(nftMint.publicKey);
    const nftMasterEdition = await Metadata.MasterEdition.getPDA(
      nftMint.publicKey
    );
    const nftChillMetadata = await nftUtils.getChillMetadataPubkey(
      nftMint.publicKey,
      nftProgram.programId
    );

    const mintNftIx = await nftProgram.methods
      .mintNft(nftType, nftArgs, user.publicKey)
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        payer: primaryWallet.publicKey,
        chillPayer: primaryWallet.publicKey,
        chillPayerTokenAccount: primaryWalletChill,
        config,
        chillMint,
        nftMint: nftMint.publicKey,
        nftMetadata,
        nftMasterEdition,
        nftChillMetadata,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: tokenProgram.programId,
        tokenMetadataProgram: Metadata.MetadataProgram.PUBKEY,
      })
      .remainingAccounts(recipientsTokenAccounts)
      .instruction();

    const mintNftTransaction = new Transaction();
    mintNftTransaction.add(mintNftIx);

    simulationResult = await provider.simulate(mintNftTransaction, null, null, [
      primaryWallet.publicKey,
    ]);

    simulatedBalance = simulationResult.accounts[0].lamports;
    withdrawAmount = preBalance - simulatedBalance;

    withdrawIx = await walletProgram.methods
      .withdrawLamports(new BN(withdrawAmount))
      .accounts({
        authority: primaryWallet.publicKey,
        proxyWallet,
        receiver: primaryWallet.publicKey,
      })
      .instruction();

    mintNftTransaction.add(withdrawIx);
    await provider.sendAndConfirm(mintNftTransaction);

    assert.equal(await utils.tokenBalance(primaryWalletChill), 0);

    // The balance of the primary wallet has not changed
    assert.equal(
      preBalance,
      await connection.getBalance(primaryWallet.publicKey)
    );

    const metadata = await Metadata.Metadata.load(connection, nftMetadata);

    const creators = metadata.data.data.creators;
    assert.equal(creators.length, 2);
    assert.equal(creators[0].address, primaryWallet.publicKey.toString());
    assert.equal(creators[0].verified, true);
    assert.equal(creators[0].share, 2);
    assert.equal(creators[1].address, user.publicKey);
    assert.equal(creators[1].verified, false);
    assert.equal(creators[1].share, 98);
  });
});
