import * as anchor from "@project-serum/anchor";
import * as utils from "./utils";
import { AnchorProvider, Program } from "@project-serum/anchor";
import { ChillNft } from "../target/types/chill_nft";
import {
  AccountMeta,
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { programs } from "@metaplex/js";
import * as assert from "assert";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

describe("NFT | Initialize", () => {
  anchor.setProvider(AnchorProvider.env());
  const program = anchor.workspace.ChillNft as Program<ChillNft>;

  const primaryWallet = Keypair.generate();

  let payer: Keypair;
  let config: PublicKey;
  let chillMint: PublicKey;

  before(async () => {
    payer = await utils.keypairWithSol();
    chillMint = await utils.createMint(primaryWallet.publicKey, 9);
    config = await utils.getNftConfigPubkey(chillMint, program.programId);
  });

  it("Try to initialize with wrong mint authority", async () => {
    const mintAuthority = Keypair.generate();
    const wrongChillMint = await utils.createMint(mintAuthority.publicKey, 9);
    const wrongConfig = await utils.getNftConfigPubkey(
      wrongChillMint,
      program.programId,
    );

    const fees = utils.randomFees();
    const recipients = utils.randomRecipients();
    await assert.rejects(async () => {
      await program.methods.initialize(fees, recipients)
        .accounts({
          primaryWallet: primaryWallet.publicKey,
          payer: payer.publicKey,
          config: wrongConfig,
          chillMint: wrongChillMint,
          systemProgram: SystemProgram.programId,
        })
        .signers([payer])
        .rpc();
    }, (err: anchor.AnchorError) => {
      assert.equal(err.error.errorCode.code, "ConstraintRaw");
      assert.equal(err.error.origin, "chill_mint");
      return true;
    });
  });

  it("Try to initialize with invalid recipients number", async () => {
    const fees = utils.randomFees();
    const recipients = utils.randomRecipients(utils.MAX_RECIPIENTS + 1);

    await assert.rejects(async () => {
      await program.methods.initialize(fees, recipients)
        .accounts({
          primaryWallet: primaryWallet.publicKey,
          payer: payer.publicKey,
          config,
          chillMint,
          systemProgram: SystemProgram.programId,
        })
        .signers([payer])
        .rpc();
    }, (err: anchor.AnchorError) => {
      assert.equal(err.error.errorCode.code, "MaximumRecipientsNumberExceeded");
      return true;
    });
  });

  it("Try to initialize with invalid recipient shares", async () => {
    const fees = utils.randomFees();

    for (let i = 0; i < 10; i++) {
      const recipients = utils.randomRecipients(utils.MAX_RECIPIENTS);
      let totalMintShare = 0;
      let totalTransactionShare = 0;

      for (const index in recipients) {
        const recipient = recipients[index];
        recipient.mintShare = utils.randomNumber(50);
        recipient.transactionShare = utils.randomNumber(50);
        totalMintShare += recipient.mintShare;
        totalTransactionShare += recipient.transactionShare;
      }

      if (totalMintShare == 100 && totalTransactionShare == 100) {
        continue;
      }

      await assert.rejects(async () => {
        await program.methods.initialize(fees, recipients)
          .accounts({
            primaryWallet: primaryWallet.publicKey,
            payer: payer.publicKey,
            config,
            chillMint,
            systemProgram: SystemProgram.programId,
          })
          .signers([payer])
          .rpc();
      }, (err: anchor.AnchorError) => {
        assert.equal(err.error.errorCode.code, "InvalidShares");
        return true;
      });
    }
  });

  it("Try to initialize with duplicated recipients", async () => {
    const fees = utils.randomFees();
    const recipients = utils.randomRecipients();
    recipients[0] = recipients[utils.MAX_RECIPIENTS - 1];

    assert.rejects(async () => {
      await program.methods.initialize(fees, recipients)
        .accounts({
          primaryWallet: primaryWallet.publicKey,
          payer: payer.publicKey,
          config,
          chillMint,
          systemProgram: SystemProgram.programId,
        })
        .signers([payer])
        .rpc();
    }, (err: anchor.AnchorError) => {
      assert.equal(err.error.errorCode.code, "DublicateRecipients");
      return true;
    });
  });

  it("Initialize", async () => {
    const fees = utils.randomFees();
    const recipients = utils.randomRecipients();

    await program.methods.initialize(fees, recipients)
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        payer: payer.publicKey,
        config,
        chillMint,
        systemProgram: SystemProgram.programId,
      })
      .signers([payer])
      .rpc();

    const configData = await program.account.config.fetch(config);
    assert.equal(JSON.stringify(configData.fees), JSON.stringify(fees));
    assert.equal(
      JSON.stringify(configData.recipients),
      JSON.stringify(recipients),
    );
  });

  it("Try to initialize twice", async () => {
    const fees = utils.randomFees();
    const recipients = utils.randomRecipients();

    await assert.rejects(async () => {
      await program.methods.initialize(fees, recipients)
        .accounts({
          primaryWallet: primaryWallet.publicKey,
          payer: payer.publicKey,
          config,
          chillMint,
          systemProgram: SystemProgram.programId,
        }).signers([payer])
        .rpc();
    });
  });
});

describe("NFT | Update", () => {
  anchor.setProvider(AnchorProvider.env());

  const program = anchor.workspace.ChillNft as Program<ChillNft>;
  const Metadata = programs.metadata;

  const primaryWallet = Keypair.generate();
  const chillPayer = Keypair.generate();
  const user = Keypair.generate();

  let payer: Keypair;
  let chillPayerTokenAccount: PublicKey;
  let config: PublicKey;
  let chillMint: PublicKey;

  const fees = utils.randomFees();
  const recipients = utils.randomRecipients();
  const initialTokenBalance = 1_000_000_000;
  const recipientsTokenAccounts: AccountMeta[] = [];

  let nftMint: PublicKey;
  let nftToken: PublicKey;
  let nftMetadata: PublicKey;

  before(async () => {
    payer = await utils.keypairWithSol();
    chillMint = await utils.createMint(primaryWallet.publicKey, 9);
    config = await utils.getNftConfigPubkey(chillMint, program.programId);

    for (let i = 0; i < recipients.length; i++) {
      const tokenAccount = await utils.createTokenAccount(
        recipients[i].address,
        chillMint,
      );

      const accountMeta = {
        pubkey: tokenAccount,
        isSigner: false,
        isWritable: true,
      };

      recipientsTokenAccounts.push(accountMeta);
    }

    await program.methods.initialize(fees, recipients)
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        payer: payer.publicKey,
        config,
        chillMint,
        systemProgram: SystemProgram.programId,
      })
      .signers([payer])
      .rpc();

    chillPayerTokenAccount = await utils.createTokenAccount(
      chillPayer.publicKey,
      chillMint,
    );

    await utils.mintTokens(
      primaryWallet,
      chillMint,
      chillPayerTokenAccount,
      initialTokenBalance,
    );
  });

  it("Mint NFT", async () => {
    nftMint = await utils.createMint(primaryWallet.publicKey, 0);
    nftToken = await utils.createTokenAccount(user.publicKey, nftMint);
    await utils.mintTokens(primaryWallet, nftMint, nftToken, 1);

    nftMetadata = await Metadata.Metadata.getPDA(nftMint);
    const nftMasterEdition = await Metadata.MasterEdition.getPDA(nftMint);
    const nftChillMetadata = await utils.getChillMetadataPubkey(
      nftMint,
      program.programId,
    );

    const nftType = utils.randomNftType();
    const nftArgs = utils.randomNftArgs();

    await program.methods.mintNft(nftType, nftArgs, null)
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        payer: payer.publicKey,
        chillPayer: chillPayer.publicKey,
        chillPayerTokenAccount,
        config,
        chillMint,
        nftMint,
        nftMetadata,
        nftMasterEdition,
        nftChillMetadata,
        rent: SYSVAR_RENT_PUBKEY,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        tokenMetadataProgram: Metadata.MetadataProgram.PUBKEY,
      }).signers([primaryWallet, payer, chillPayer])
      .remainingAccounts(recipientsTokenAccounts).rpc();

    const chillMetadata = await program.account.chillNftMetadata.fetch(
      nftChillMetadata,
    );

    assert.equal(
      JSON.stringify(chillMetadata.nftType),
      JSON.stringify(nftType),
    );

    const chillFeesAmount = utils.feesOf(fees, nftType).toNumber();
    const recipientsTokenAmounts: number[] = [];
    for (const index in recipientsTokenAccounts) {
      recipientsTokenAmounts.push(
        await utils.tokenBalance(recipientsTokenAccounts[index].pubkey),
      );
    }

    const chillPayerTokenAmount = await utils.tokenBalance(
      chillPayerTokenAccount,
    );
    assert.equal(chillPayerTokenAmount, initialTokenBalance - chillFeesAmount);

    let sum = 0;
    for (const index in recipientsTokenAmounts) {
      const share = recipients[index].mintShare;
      const amount = await utils.tokenBalance(
        recipientsTokenAccounts[index].pubkey,
      );
      const expectedAmount = Math.round(chillFeesAmount * share / 100);

      // Rust calculates amounts using integers, so they may slightly
      // differ from calculations with floating-point numbers
      assert.ok(expectedAmount + 3 > amount);
      assert.ok(expectedAmount - 3 < amount);

      sum += amount;
    }
    assert.equal(sum, chillFeesAmount);

    const metadata = await Metadata.Metadata.load(
      program.provider.connection,
      nftMetadata,
    );

    assert.equal(metadata.data.data.uri, nftArgs.uri);
    assert.equal(metadata.data.data.name, nftArgs.name);
    assert.equal(metadata.data.data.symbol, nftArgs.symbol);
    assert.equal(metadata.data.data.sellerFeeBasisPoints, nftArgs.fees);

    const creators = metadata.data.data.creators;
    assert.equal(creators.length, 1);
    assert.equal(creators[0].address, primaryWallet.publicKey.toString());
    assert.equal(creators[0].verified, true);
    assert.equal(creators[0].share, 100);
  });

  it("Update NFT", async () => {
    const newNftArgs = utils.randomNftArgs();
    await program.methods.updateNft(newNftArgs).accounts({
      primaryWallet: primaryWallet.publicKey,
      nftMetadata,
      tokenMetadataProgram: Metadata.MetadataProgram.PUBKEY,
    }).signers([primaryWallet]).rpc();

    const metadata = await Metadata.Metadata.load(
      program.provider.connection,
      nftMetadata,
    );

    assert.equal(metadata.data.data.uri, newNftArgs.uri);
    assert.equal(metadata.data.data.name, newNftArgs.name);
    assert.equal(metadata.data.data.symbol, newNftArgs.symbol);
    assert.equal(metadata.data.data.sellerFeeBasisPoints, newNftArgs.fees);

    const creators = metadata.data.data.creators;
    assert.equal(creators.length, 1);
    assert.equal(creators[0].address, primaryWallet.publicKey.toString());
    assert.equal(creators[0].verified, true);
    assert.equal(creators[0].share, 100);
  });
});
