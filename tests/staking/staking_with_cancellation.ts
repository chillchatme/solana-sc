import * as assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as utils from "../utils";
import * as stakingUtils from "./utils";
import { BN, Program } from "@project-serum/anchor";
import { ChillStaking } from "../../target/types/chill_staking";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

describe("Staking simulation | Staking with cancellation", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.ChillStaking as Program<ChillStaking>;
  const primaryWallet = Keypair.generate();

  let payer: Keypair;
  let chillMint: PublicKey;
  let stakingInfoPubkey: PublicKey;
  let stakingTokenAuthority: PublicKey;
  let stakingTokenAccount: PublicKey;

  let firstUser: Keypair;
  let secondUser: Keypair;

  let firstUserInfoPubkey: PublicKey;
  let secondUserInfoPubkey: PublicKey;

  let firstTokenAccount: PublicKey;
  let secondTokenAccount: PublicKey;

  const expectedStakingInfo = stakingUtils.getDefaultStakingInfo();
  const expectedFirstUserInfo = stakingUtils.getDefaultUserInfo();
  const expectedSecondUserInfo = stakingUtils.getDefaultUserInfo();

  const initialBalance = 200_000;
  const totalDays = 10;
  let startDay: number;

  let firstStakeAccounts: stakingUtils.StakeAccounts;

  before(async () => {
    payer = await utils.keypairWithSol();
    chillMint = await utils.createMint(primaryWallet.publicKey, 9);

    [firstUser, firstTokenAccount] =
      await stakingUtils.createUserWithTokenAccount(
        chillMint,
        primaryWallet,
        initialBalance
      );

    [secondUser, secondTokenAccount] =
      await stakingUtils.createUserWithTokenAccount(
        chillMint,
        primaryWallet,
        initialBalance
      );

    stakingInfoPubkey = await stakingUtils.initializeStaking(
      primaryWallet,
      payer,
      totalDays,
      chillMint,
      program
    );

    stakingTokenAuthority = await stakingUtils.getStakingAuthority(
      stakingInfoPubkey,
      program.programId
    );

    stakingTokenAccount = await utils.getAssociatedTokenAddress(
      stakingTokenAuthority,
      chillMint
    );

    firstUserInfoPubkey = await stakingUtils.getUserInfoPubkey(
      firstUser.publicKey,
      stakingInfoPubkey,
      program.programId
    );

    secondUserInfoPubkey = await stakingUtils.getUserInfoPubkey(
      secondUser.publicKey,
      stakingInfoPubkey,
      program.programId
    );

    await stakingUtils.addRewardTokens(
      100_000_000,
      primaryWallet,
      chillMint,
      stakingInfoPubkey,
      program
    );

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    firstStakeAccounts = {
      user: firstUser.publicKey,
      payer: payer.publicKey,
      tokenAccountAuthority: firstUser.publicKey,
      userInfo: firstUserInfoPubkey,
      fromTokenAccount: firstTokenAccount,
      stakingInfo: stakingInfoPubkey,
      stakingTokenAuthority,
      stakingTokenAccount,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
    };

    startDay = stakingInfo.startDay.toNumber();

    expectedStakingInfo.primaryWallet = primaryWallet.publicKey;
    expectedStakingInfo.mint = chillMint;
    expectedStakingInfo.startDay = stakingInfo.startDay;
    expectedStakingInfo.endDay = stakingInfo.startDay.addn(totalDays);
    expectedStakingInfo.rewardTokensAmount = new BN(100_000_000);

    expectedFirstUserInfo.user = firstUser.publicKey;
    expectedFirstUserInfo.stakingInfo = stakingInfoPubkey;

    expectedSecondUserInfo.user = secondUser.publicKey;
    expectedSecondUserInfo.stakingInfo = stakingInfoPubkey;
  });

  // Day        User_1       User_2
  // 0         +20_000      +20_000
  // 1         +20_000            0
  // 2               0            0
  // 3          Cancel            0
  // 4               0            0
  // 5               0            0
  // 6               0            0
  // 7               0    Claim all

  it("Day 0", async () => {
    await stakingUtils.waitUntil(program, startDay);

    await program.methods
      .stake(new BN(20_000))
      .accounts(firstStakeAccounts)
      .signers([firstUser, payer])
      .rpc();

    await program.methods
      .stake(new BN(20_000))
      .accounts({
        user: secondUser.publicKey,
        payer: payer.publicKey,
        tokenAccountAuthority: secondUser.publicKey,
        userInfo: secondUserInfoPubkey,
        fromTokenAccount: secondTokenAccount,
        stakingInfo: stakingInfoPubkey,
        stakingTokenAuthority,
        stakingTokenAccount,
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([secondUser, payer])
      .rpc({ skipPreflight: true });

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const firstUserInfo = await program.account.userInfo.fetch(
      firstUserInfoPubkey
    );

    const secondUserInfo = await program.account.userInfo.fetch(
      secondUserInfoPubkey
    );

    expectedFirstUserInfo.bump = firstUserInfo.bump;
    expectedFirstUserInfo.dailyStakingReward = new BN(5_000_000);
    expectedFirstUserInfo.stakedAmount = new BN(20_000);
    expectedFirstUserInfo.startDay = new BN(startDay);
    expectedFirstUserInfo.totalStakedAmount = new BN(20_000);

    expectedSecondUserInfo.bump = secondUserInfo.bump;
    expectedSecondUserInfo.dailyStakingReward = new BN(5_000_000);
    expectedSecondUserInfo.stakedAmount = new BN(20_000);
    expectedSecondUserInfo.startDay = new BN(startDay);
    expectedSecondUserInfo.totalStakedAmount = new BN(20_000);

    expectedStakingInfo.activeStakesNumber = new BN(2);
    expectedStakingInfo.lastDailyReward = new BN(5_000_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay);
    expectedStakingInfo.totalStakedAmount = new BN(40_000);
    expectedStakingInfo.totalStakesNumber = new BN(2);

    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 1", async () => {
    await program.methods
      .stake(new BN(20_000))
      .accounts(firstStakeAccounts)
      .signers([firstUser, payer])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const firstUserInfo = await program.account.userInfo.fetch(
      firstUserInfoPubkey
    );

    expectedFirstUserInfo.pendingAmount = new BN(20_000);

    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 2", async () => {
    await program.methods
      .boost()
      .accounts({
        user: firstUser.publicKey,
        userInfo: firstUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([firstUser])
      .rpc();

    await program.methods
      .boost()
      .accounts({
        user: secondUser.publicKey,
        userInfo: secondUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([secondUser])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const firstUserInfo = await program.account.userInfo.fetch(
      firstUserInfoPubkey
    );

    const secondUserInfo = await program.account.userInfo.fetch(
      secondUserInfoPubkey
    );

    expectedFirstUserInfo.totalBoostAmount = new BN(1);
    expectedSecondUserInfo.totalBoostAmount = new BN(1);
    expectedStakingInfo.totalBoostAmount = new BN(2);

    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 3", async () => {
    await program.methods
      .cancel()
      .accounts({
        user: firstUser.publicKey,
        userInfo: firstUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([firstUser])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const firstUserInfo = await program.account.userInfo.fetch(
      firstUserInfoPubkey
    );

    expectedFirstUserInfo.pendingAmount = new BN(40_000);
    expectedFirstUserInfo.stakedAmount = new BN(0);
    expectedFirstUserInfo.startDay = null;
    expectedFirstUserInfo.totalBoostAmount = new BN(0);
    expectedFirstUserInfo.totalStakedAmount = new BN(0);

    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.totalBoostAmount = new BN(1);
    expectedStakingInfo.totalStakedAmount = new BN(20_000);
    expectedStakingInfo.totalStakesNumber = new BN(1);

    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 7", async () => {
    await stakingUtils.waitUntil(program, startDay + 7);

    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      secondUserInfoPubkey,
      stakingInfoPubkey
    );

    let secondUserInfo = await program.account.userInfo.fetch(
      secondUserInfoPubkey
    );

    const amount = reward
      .add(secondUserInfo.pendingAmount)
      .add(secondUserInfo.stakedAmount);

    let tokenBalance = await utils.tokenBalance(secondTokenAccount);
    const expectedBalance = amount.addn(tokenBalance);

    await program.methods
      .claim(amount)
      .accounts({
        user: secondUser.publicKey,
        userInfo: secondUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
        stakingTokenAuthority,
        stakingTokenAccount,
        recipientTokenAccount: secondTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([secondUser])
      .rpc();

    tokenBalance = await utils.tokenBalance(secondTokenAccount);
    assert.equal(tokenBalance, expectedBalance.toNumber());

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    secondUserInfo = await program.account.userInfo.fetch(secondUserInfoPubkey);

    expectedSecondUserInfo.pendingAmount = new BN(0);
    expectedSecondUserInfo.rewardedAmount = new BN(0);
    expectedSecondUserInfo.stakedAmount = new BN(0);
    expectedSecondUserInfo.startDay = null;
    expectedSecondUserInfo.totalRewardedAmount = new BN(40_000_000);

    expectedStakingInfo.activeStakesNumber = new BN(0);
    expectedStakingInfo.totalRewardedAmount = new BN(40_000_000);
    expectedStakingInfo.totalUnspentAmount = new BN(30_000_000);

    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });
});
