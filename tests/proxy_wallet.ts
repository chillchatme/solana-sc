import * as anchor from "@project-serum/anchor";
import * as utils from "./utils";
import { AnchorError, BN, Program } from "@project-serum/anchor";
import { ChillWallet } from "../target/types/chill_wallet";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import * as assert from "assert";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

describe("Proxy wallet", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.ChillWallet as Program<ChillWallet>;
  const connection = program.provider.connection;

  const mintAuthority = Keypair.generate();
  const primaryWallet = Keypair.generate();
  const user = Keypair.generate();
  const lamports = 1_000_000;
  const chillTokensAmount = 1_000_000_000;

  let receiver: Keypair;
  let payer: Keypair;
  let wrongAuthorty: Keypair;
  let proxyWallet: PublicKey;

  let chillMint: PublicKey;
  let nftMint: PublicKey;

  let proxyWalletChillToken: PublicKey;
  let proxyWalletNftToken: PublicKey;
  let receiverChillToken: PublicKey;
  let receiverNftToken: PublicKey;

  before(async () => {
    payer = await utils.keypairWithSol();
    wrongAuthorty = await utils.keypairWithSol();
    receiver = await utils.keypairWithSol();
    proxyWallet = await utils.getWalletPubkey(
      user.publicKey,
      primaryWallet.publicKey,
      program.programId,
    );

    chillMint = await utils.createMint(mintAuthority.publicKey, 9);
    nftMint = await utils.createMint(mintAuthority.publicKey, 0);

    proxyWalletChillToken = await utils.createTokenAccount(
      proxyWallet,
      chillMint,
    );

    proxyWalletNftToken = await utils.createTokenAccount(proxyWallet, nftMint);

    receiverNftToken = await utils.createTokenAccount(
      receiver.publicKey,
      nftMint,
    );

    receiverChillToken = await utils.createTokenAccount(
      receiver.publicKey,
      chillMint,
    );

    await utils.mintTokens(
      mintAuthority,
      chillMint,
      proxyWalletChillToken,
      chillTokensAmount,
    );

    await utils.mintTokens(mintAuthority, nftMint, proxyWalletNftToken, 1);
  });

  it("Create wallet", async () => {
    await program.methods.createWallet().accounts({
      primaryWallet: primaryWallet.publicKey,
      user: user.publicKey,
      payer: payer.publicKey,
      proxyWallet,
      systemProgram: SystemProgram.programId,
    }).signers([payer]).rpc();

    const wallet = await program.account.proxyWallet.fetch(proxyWallet);
    assert.deepEqual(wallet.primaryWallet, primaryWallet.publicKey);
    assert.deepEqual(wallet.user, user.publicKey);
    assert.equal(wallet.totalMoneyWithdrawnUser.toNumber(), 0);
    assert.equal(wallet.totalMoneyWithdrawnPrimaryWallet.toNumber(), 0);
    assert.equal(wallet.totalFtWithdrawnUser.toNumber(), 0);
    assert.equal(wallet.totalFtWithdrawnPrimaryWallet.toNumber(), 0);
    assert.equal(wallet.totalNftWithdrawnUser.toNumber(), 0);
    assert.equal(wallet.totalNftWithdrawnPrimaryWallet.toNumber(), 0);
  });

  it("Try to withdraw lamports with wrong authority", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawLamports(new BN(1)).accounts({
        authority: wrongAuthorty.publicKey,
        proxyWallet,
        receiver: receiver.publicKey,
      }).signers([wrongAuthorty]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "WrongAuthority");
      return true;
    });
  });

  it("Try to withdraw FT with wrong authority", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawFt(new BN(1)).accounts({
        authority: wrongAuthorty.publicKey,
        mint: chillMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletChillToken,
        receiverTokenAccount: receiverChillToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([wrongAuthorty]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "WrongAuthority");
      return true;
    });
  });

  it("Try to withdraw NFT with wrong authority", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawNft().accounts({
        authority: wrongAuthorty.publicKey,
        nftMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletNftToken,
        receiverTokenAccount: receiverNftToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([wrongAuthorty]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "WrongAuthority");
      return true;
    });
  });

  it("Try to withdraw lamports from empty wallet", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawLamports(new BN(1)).accounts({
        authority: primaryWallet.publicKey,
        proxyWallet,
        receiver: receiver.publicKey,
      }).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "InsufficientFunds");
      return true;
    });
  });

  it("Try to withdraw too many lamports", async () => {
    await utils.transferLamports(payer, proxyWallet, lamports);
    await assert.rejects(async () => {
      await program.methods.withdrawLamports(new BN(lamports + 1)).accounts(
        {
          authority: primaryWallet.publicKey,
          proxyWallet,
          receiver: receiver.publicKey,
        },
      ).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "InsufficientFunds");
      return true;
    });
  });

  it("Try to withdraw too many FT", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawFt(new BN(chillTokensAmount + 1)).accounts({
        authority: primaryWallet.publicKey,
        mint: chillMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletChillToken,
        receiverTokenAccount: receiverChillToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([primaryWallet]).rpc();
    });
  });

  it("Try to withdraw NFT as FT", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawFt(new BN(1)).accounts({
        authority: primaryWallet.publicKey,
        mint: nftMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletNftToken,
        receiverTokenAccount: receiverNftToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "TokenIsNft");
      return true;
    });
  });

  it("Try to withdraw FT as NFT", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawNft().accounts({
        authority: primaryWallet.publicKey,
        nftMint: chillMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletChillToken,
        receiverTokenAccount: receiverChillToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "TokenIsNotNft");
      return true;
    });
  });

  it("Withdraw lamports to proxy wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await connection.getBalance(proxyWallet);

    await program.methods.withdrawLamports(new BN(lamports)).accounts({
      authority: primaryWallet.publicKey,
      proxyWallet,
      receiver: proxyWallet,
    }).signers([primaryWallet]).rpc();

    const newReceiverBalance = await connection.getBalance(proxyWallet);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(newReceiverBalance, initialReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw lamports by primary wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await connection.getBalance(
      receiver.publicKey,
    );

    const amount = lamports / 2;
    proxyAccount.totalMoneyWithdrawnPrimaryWallet.iadd(new BN(amount));

    await program.methods.withdrawLamports(new BN(amount)).accounts({
      authority: primaryWallet.publicKey,
      proxyWallet,
      receiver: receiver.publicKey,
    }).signers([primaryWallet]).rpc();

    const newReceiverBalance = await connection.getBalance(receiver.publicKey);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(newReceiverBalance - initialReceiverBalance, amount);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw lamports by user", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await connection.getBalance(
      receiver.publicKey,
    );

    const amount = lamports / 2;
    proxyAccount.totalMoneyWithdrawnUser.iadd(new BN(amount));

    await program.methods.withdrawLamports(new BN(amount)).accounts({
      authority: user.publicKey,
      proxyWallet,
      receiver: receiver.publicKey,
    }).signers([user]).rpc();

    const newReceiverBalance = await connection.getBalance(receiver.publicKey);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(newReceiverBalance - initialReceiverBalance, amount);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw FT to proxy wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await utils.tokenBalance(
      proxyWalletChillToken,
    );

    await program.methods.withdrawFt(new BN(chillTokensAmount)).accounts({
      authority: primaryWallet.publicKey,
      mint: chillMint,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletChillToken,
      receiverTokenAccount: proxyWalletChillToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([primaryWallet]).rpc();

    const newReceiverBalance = await utils.tokenBalance(proxyWalletChillToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw FT by primary wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await utils.tokenBalance(receiverChillToken);
    const amount = chillTokensAmount / 2;
    proxyAccount.totalFtWithdrawnPrimaryWallet.iadd(new BN(amount));

    await program.methods.withdrawFt(new BN(amount)).accounts({
      authority: primaryWallet.publicKey,
      mint: chillMint,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletChillToken,
      receiverTokenAccount: receiverChillToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([primaryWallet]).rpc();

    const newReceiverBalance = await utils.tokenBalance(receiverChillToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance + amount, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw FT by user", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await utils.tokenBalance(receiverChillToken);
    const amount = chillTokensAmount / 2;
    proxyAccount.totalFtWithdrawnUser.iadd(new BN(amount));

    await program.methods.withdrawFt(new BN(amount)).accounts({
      authority: user.publicKey,
      mint: chillMint,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletChillToken,
      receiverTokenAccount: receiverChillToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([user]).rpc();

    const newReceiverBalance = await utils.tokenBalance(receiverChillToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance + amount, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw NFT to proxy wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await utils.tokenBalance(proxyWalletNftToken), 1);

    await program.methods.withdrawNft().accounts({
      authority: primaryWallet.publicKey,
      nftMint,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletNftToken,
      receiverTokenAccount: proxyWalletNftToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([primaryWallet]).rpc();

    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await utils.tokenBalance(proxyWalletNftToken), 1);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw NFT by primary wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await utils.tokenBalance(receiverNftToken), 0);
    proxyAccount.totalNftWithdrawnPrimaryWallet.iaddn(1);

    await program.methods.withdrawNft().accounts({
      authority: primaryWallet.publicKey,
      nftMint,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletNftToken,
      receiverTokenAccount: receiverNftToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([primaryWallet]).rpc();

    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await utils.tokenBalance(receiverNftToken), 1);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));

    // sending NFT back
    await utils.transferTokens(
      receiver,
      receiverNftToken,
      proxyWalletNftToken,
      1,
    );
  });

  it("Withdraw NFT by user", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await utils.tokenBalance(receiverNftToken), 0);
    proxyAccount.totalNftWithdrawnUser.iaddn(1);

    await program.methods.withdrawNft().accounts({
      authority: user.publicKey,
      nftMint,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletNftToken,
      receiverTokenAccount: receiverNftToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([user]).rpc();

    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await utils.tokenBalance(receiverNftToken), 1);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });
});
