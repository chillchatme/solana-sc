import * as anchor from "@project-serum/anchor";
import { AnchorError, BN, Program } from "@project-serum/anchor";
import { ChillWallet } from "../target/types/chill_wallet";
import { NATIVE_MINT } from "@solana/spl-token";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import {
  airdrop,
  createMint,
  createTokenAccount,
  getWalletPubkey,
  keypairWithSol,
  mintTokens,
  tokenBalance,
  transferLamports,
  transferTokens,
} from "./utils";
import * as assert from "assert";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

describe("Proxy wallet", () => {
  anchor.setProvider(anchor.Provider.env());
  const program = anchor.workspace.ChillWallet as Program<ChillWallet>;
  const connection = program.provider.connection;

  const mintAuthority = Keypair.generate();
  const primaryWallet = Keypair.generate();
  const user = Keypair.generate();
  const lamports = 1_000_000;
  const chillTokensAmount = 1_000_000_000;
  const wrappedAmount = 100_000;

  let receiver: Keypair;
  let payer: Keypair;
  let wrongAuthorty: Keypair;
  let proxyWallet: PublicKey;

  let chillMint: PublicKey;
  let nftMint: PublicKey;

  let proxyWalletChillToken: PublicKey;
  let proxyWalletNftToken: PublicKey;
  let proxyWalletWrappedToken: PublicKey;
  let receiverChillToken: PublicKey;
  let receiverNftToken: PublicKey;
  let receiverWrappedToken: PublicKey;

  before(async () => {
    payer = await keypairWithSol();
    wrongAuthorty = await keypairWithSol();
    receiver = await keypairWithSol();
    proxyWallet = await getWalletPubkey(
      user.publicKey,
      primaryWallet.publicKey,
      program.programId,
    );

    chillMint = await createMint(mintAuthority.publicKey, 9);
    nftMint = await createMint(mintAuthority.publicKey, 0);

    proxyWalletChillToken = await createTokenAccount(proxyWallet, chillMint);
    proxyWalletNftToken = await createTokenAccount(proxyWallet, nftMint);
    proxyWalletWrappedToken = await createTokenAccount(
      proxyWallet,
      NATIVE_MINT,
    );
    await airdrop(proxyWalletWrappedToken, wrappedAmount);

    receiverNftToken = await createTokenAccount(receiver.publicKey, nftMint);
    receiverChillToken = await createTokenAccount(
      receiver.publicKey,
      chillMint,
    );
    receiverWrappedToken = await createTokenAccount(
      receiver.publicKey,
      NATIVE_MINT,
    );

    await mintTokens(
      mintAuthority,
      chillMint,
      proxyWalletChillToken,
      chillTokensAmount,
    );

    await mintTokens(mintAuthority, nftMint, proxyWalletNftToken, 1);
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
    await transferLamports(payer, proxyWallet, lamports);
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
    await assert.rejects(async () => {
      await program.methods.withdrawLamports(new BN(lamports)).accounts({
        authority: primaryWallet.publicKey,
        proxyWallet,
        receiver: proxyWallet,
      }).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "SendingToYourself");
      return true;
    });
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

  it("Withdraw wrapped SOL by primary wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await tokenBalance(receiverWrappedToken);
    const amount = wrappedAmount / 2;
    proxyAccount.totalMoneyWithdrawnPrimaryWallet.iadd(new BN(amount));

    await program.methods.withdrawFt(new BN(amount)).accounts({
      authority: primaryWallet.publicKey,
      mint: NATIVE_MINT,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletWrappedToken,
      receiverTokenAccount: receiverWrappedToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([primaryWallet]).rpc();

    const newReceiverBalance = await tokenBalance(receiverWrappedToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance + amount, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw wrapped SOL by user", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await tokenBalance(receiverWrappedToken);
    const amount = wrappedAmount / 2;
    proxyAccount.totalMoneyWithdrawnUser.iadd(new BN(amount));

    await program.methods.withdrawFt(new BN(amount)).accounts({
      authority: user.publicKey,
      mint: NATIVE_MINT,
      proxyWallet,
      proxyWalletTokenAccount: proxyWalletWrappedToken,
      receiverTokenAccount: receiverWrappedToken,
      tokenProgram: TOKEN_PROGRAM_ID,
    }).signers([user]).rpc();

    const newReceiverBalance = await tokenBalance(receiverWrappedToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance + amount, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw FT to proxy wallet", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawFt(new BN(1)).accounts({
        authority: primaryWallet.publicKey,
        mint: chillMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletChillToken,
        receiverTokenAccount: proxyWalletChillToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "SendingToYourself");
      return true;
    });
  });

  it("Withdraw FT by primary wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await tokenBalance(receiverChillToken);
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

    const newReceiverBalance = await tokenBalance(receiverChillToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance + amount, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw FT by user", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    const initialReceiverBalance = await tokenBalance(receiverChillToken);
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

    const newReceiverBalance = await tokenBalance(receiverChillToken);
    const newProxyState = await program.account.proxyWallet.fetch(proxyWallet);

    assert.equal(initialReceiverBalance + amount, newReceiverBalance);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });

  it("Withdraw NFT to proxy wallet", async () => {
    await assert.rejects(async () => {
      await program.methods.withdrawNft().accounts({
        authority: primaryWallet.publicKey,
        nftMint,
        proxyWallet,
        proxyWalletTokenAccount: proxyWalletNftToken,
        receiverTokenAccount: proxyWalletNftToken,
        tokenProgram: TOKEN_PROGRAM_ID,
      }).signers([primaryWallet]).rpc();
    }, (err: AnchorError) => {
      assert.equal(err.error.errorCode.code, "SendingToYourself");
      return true;
    });
  });

  it("Withdraw NFT by primary wallet", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await tokenBalance(receiverNftToken), 0);
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
    assert.equal(await tokenBalance(receiverNftToken), 1);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));

    // sending NFT back
    await transferTokens(receiver, receiverNftToken, proxyWalletNftToken, 1);
  });

  it("Withdraw NFT by user", async () => {
    const proxyAccount = await program.account.proxyWallet.fetch(proxyWallet);
    assert.equal(await tokenBalance(receiverNftToken), 0);
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
    assert.equal(await tokenBalance(receiverNftToken), 1);
    assert.equal(JSON.stringify(newProxyState), JSON.stringify(proxyAccount));
  });
});
