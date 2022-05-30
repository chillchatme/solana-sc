import * as assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as utils from "../utils";
import * as stakingUtils from "./utils";
import { BN, Program } from "@project-serum/anchor";
import { ChillStaking } from "../../target/types/chill_staking";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

describe("Staking simulation | All days with active stake and boost", () => {
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
  let thirdUser: Keypair;

  let firstUserInfoPubkey: PublicKey;
  let secondUserInfoPubkey: PublicKey;
  let thirdUserInfoPubkey: PublicKey;

  const expectedStakingInfo = stakingUtils.getDefaultStakingInfo();
  const expectedFirstUserInfo = stakingUtils.getDefaultUserInfo();
  const expectedSecondUserInfo = stakingUtils.getDefaultUserInfo();
  const expectedThirdUserInfo = stakingUtils.getDefaultUserInfo();

  let firstTokenAccount: PublicKey;
  let secondTokenAccount: PublicKey;
  let thirdTokenAccount: PublicKey;

  const initialBalance = 200_000;
  const totalDays = 10;
  let startDay: number;

  let firstStakeAccounts: stakingUtils.StakeAccounts;
  let secondStakeAccounts: stakingUtils.StakeAccounts;
  let thirdStakeAccounts: stakingUtils.StakeAccounts;

  async function boost(
    user: Keypair,
    userInfoPubkey: PublicKey,
    expectedUserInfo: stakingUtils.UserInfo
  ) {
    await program.methods
      .boost()
      .accounts({
        user: user.publicKey,
        userInfo: userInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([user])
      .rpc();

    const currentDay = await stakingUtils.getCurrentDay(program);
    expectedUserInfo.totalBoostNumber.iaddn(1);
    expectedStakingInfo.totalBoostNumber.iaddn(1);
    expectedStakingInfo.lastUpdateDay = currentDay;

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  }

  async function firstBoost() {
    await boost(firstUser, firstUserInfoPubkey, expectedFirstUserInfo);
  }

  async function secondBoost() {
    await boost(secondUser, secondUserInfoPubkey, expectedSecondUserInfo);
  }

  async function thirdBoost() {
    await boost(thirdUser, thirdUserInfoPubkey, expectedThirdUserInfo);
  }

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

    [thirdUser, thirdTokenAccount] =
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

    thirdUserInfoPubkey = await stakingUtils.getUserInfoPubkey(
      thirdUser.publicKey,
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
    startDay = stakingInfo.startDay.toNumber();

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

    secondStakeAccounts = {
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
    };

    thirdStakeAccounts = {
      user: thirdUser.publicKey,
      payer: payer.publicKey,
      tokenAccountAuthority: thirdUser.publicKey,
      userInfo: thirdUserInfoPubkey,
      fromTokenAccount: thirdTokenAccount,
      stakingInfo: stakingInfoPubkey,
      stakingTokenAuthority,
      stakingTokenAccount,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
    };

    expectedStakingInfo.primaryWallet = primaryWallet.publicKey;
    expectedStakingInfo.mint = chillMint;
    expectedStakingInfo.startDay = stakingInfo.startDay;
    expectedStakingInfo.endDay = stakingInfo.startDay.addn(totalDays);
    expectedStakingInfo.rewardTokensAmount = new BN(100_000_000);

    expectedFirstUserInfo.user = firstUser.publicKey;
    expectedFirstUserInfo.stakingInfo = stakingInfoPubkey;

    expectedSecondUserInfo.user = secondUser.publicKey;
    expectedSecondUserInfo.stakingInfo = stakingInfoPubkey;

    expectedThirdUserInfo.user = thirdUser.publicKey;
    expectedThirdUserInfo.stakingInfo = stakingInfoPubkey;
  });

  // Day        User_1       User_2       User_3
  // 0         +20_000            0            0
  // 1               0      +20_000            0
  // 2               0            0            0
  // 3               0            0            0
  // 4               0            0      +30_000
  // 5         +20_000      +20_000            0
  // 6               0            0            0
  // 7               0            0            0
  // 8               0      +15_000            0
  // 9               0            0            0
  // 10      Claim all            0            0
  // 11              0    Claim all            0
  // 12              0            0            0
  // 13              0            0            0
  // 14              0            0    Claim all

  it("Day 0", async () => {
    await stakingUtils.waitUntil(program, startDay);
    await program.methods
      .stake(new BN(20_000))
      .accounts(firstStakeAccounts)
      .signers([firstUser, payer])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(firstUserInfoPubkey);

    expectedFirstUserInfo.bump = userInfo.bump;
    expectedFirstUserInfo.startDay = new BN(startDay);
    expectedFirstUserInfo.stakedAmount = new BN(20_000);
    expectedFirstUserInfo.totalStakedAmount = new BN(20_000);
    expectedFirstUserInfo.dailyStakingReward = new BN(5_000_000);

    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.totalStakesNumber = new BN(1);
    expectedStakingInfo.totalStakedAmount = new BN(20_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay);
    expectedStakingInfo.lastDayWithStake = new BN(startDay);
    expectedStakingInfo.lastDailyReward = new BN(5_000_000);

    stakingUtils.assertUserInfoEqual(userInfo, expectedFirstUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    await firstBoost();
  });

  it("Day 1", async () => {
    await stakingUtils.waitUntil(program, startDay + 1);

    await program.methods
      .stake(new BN(20_000))
      .accounts(secondStakeAccounts)
      .signers([secondUser, payer])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(secondUserInfoPubkey);

    expectedSecondUserInfo.bump = userInfo.bump;
    expectedSecondUserInfo.startDay = new BN(startDay + 1);
    expectedSecondUserInfo.stakedAmount = new BN(20_000);
    expectedSecondUserInfo.totalStakedAmount = new BN(20_000);
    expectedSecondUserInfo.dailyStakingReward = new BN(5_000_000);

    expectedStakingInfo.activeStakesNumber = new BN(2);
    expectedStakingInfo.totalStakesNumber = new BN(2);
    expectedStakingInfo.totalStakedAmount = new BN(40_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 1);
    expectedStakingInfo.lastDayWithStake = new BN(startDay + 1);

    stakingUtils.assertUserInfoEqual(userInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    await firstBoost();
    await secondBoost();
  });

  it("Day 2", async () => {
    await stakingUtils.waitUntil(program, startDay + 2);

    await firstBoost();
    await secondBoost();
  });

  it("Day 3", async () => {
    await stakingUtils.waitUntil(program, startDay + 3);

    await firstBoost();
    await secondBoost();
  });

  it("Day 4", async () => {
    await stakingUtils.waitUntil(program, startDay + 4);

    await program.methods
      .stake(new BN(30_000))
      .accounts(thirdStakeAccounts)
      .signers([thirdUser, payer])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(thirdUserInfoPubkey);

    expectedThirdUserInfo.bump = userInfo.bump;
    expectedThirdUserInfo.startDay = new BN(startDay + 4);
    expectedThirdUserInfo.stakedAmount = new BN(30_000);
    expectedThirdUserInfo.totalStakedAmount = new BN(30_000);
    expectedThirdUserInfo.dailyStakingReward = new BN(5_000_000);

    expectedStakingInfo.activeStakesNumber = new BN(3);
    expectedStakingInfo.totalStakesNumber = new BN(3);
    expectedStakingInfo.totalStakedAmount = new BN(70_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 4);
    expectedStakingInfo.lastDayWithStake = new BN(startDay + 4);

    stakingUtils.assertUserInfoEqual(userInfo, expectedThirdUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    await firstBoost();
    await secondBoost();
    await thirdBoost();
  });

  it("Day 5", async () => {
    await stakingUtils.waitUntil(program, startDay + 5);

    await program.methods
      .stake(new BN(20_000))
      .accounts(firstStakeAccounts)
      .signers([firstUser, payer])
      .rpc();

    await program.methods
      .stake(new BN(20_000))
      .accounts(secondStakeAccounts)
      .signers([secondUser, payer])
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

    expectedFirstUserInfo.pendingAmount = new BN(20_000);
    expectedSecondUserInfo.pendingAmount = new BN(20_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 5);

    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    await firstBoost();
    await secondBoost();
    await thirdBoost();
  });

  it("Day 6", async () => {
    await stakingUtils.waitUntil(program, startDay + 6);

    await firstBoost();
    await secondBoost();
    await thirdBoost();
  });

  it("Day 7", async () => {
    await stakingUtils.waitUntil(program, startDay + 7);

    await secondBoost();
    await thirdBoost();
  });

  it("Day 8", async () => {
    await stakingUtils.waitUntil(program, startDay + 8);

    await program.methods
      .stake(new BN(15_000))
      .accounts(secondStakeAccounts)
      .signers([secondUser, payer])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(secondUserInfoPubkey);

    // Reward
    // Day 1: 5_000_000 * 20_000 * 2 / 40_000 = 5_000_000
    // Day 2: 5_000_000 * 20_000 * 2 / 40_000 = 5_000_000
    // Day 3: 5_000_000 * 20_000 * 2 / 40_000 = 5_000_000
    // Day 4: 5_000_000 * 20_000 * 2 / 70_000 = 2_857_142
    // Day 5: 5_000_000 * 20_000 * 2 / 70_000 = 2_857_142
    // Day 6: 5_000_000 * 20_000 * 2 / 70_000 = 2_857_142
    // Day 7: 5_000_000 * 20_000 * 2 / 50_000 = 4_000_000
    // Total: 27_571_426

    expectedSecondUserInfo.startDay = new BN(startDay + 8);
    expectedSecondUserInfo.stakedAmount = new BN(55_000);
    expectedSecondUserInfo.pendingAmount = new BN(0);
    expectedSecondUserInfo.totalStakedAmount = new BN(75_000);
    expectedSecondUserInfo.dailyStakingReward = new BN(5_000_000);
    expectedSecondUserInfo.rewardedAmount = new BN(27_571_426);
    expectedSecondUserInfo.totalRewardedAmount = new BN(27_571_426);

    expectedStakingInfo.activeStakesNumber = new BN(3);
    expectedStakingInfo.totalStakesNumber = new BN(4);
    expectedStakingInfo.totalStakedAmount = new BN(125_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 8);
    expectedStakingInfo.lastDayWithStake = new BN(startDay + 8);
    expectedStakingInfo.totalRewardedAmount = new BN(27_571_426);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 8);

    stakingUtils.assertUserInfoEqual(userInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    await secondBoost();
    await thirdBoost();
  });

  it("Day 9", async () => {
    await stakingUtils.waitUntil(program, startDay + 9);
    await secondBoost();
    await thirdBoost();
  });

  it("Day 10", async () => {
    await stakingUtils.waitUntil(program, startDay + 10);

    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      firstUserInfoPubkey,
      stakingInfoPubkey
    );

    const firstUserInfo = await program.account.userInfo.fetch(
      firstUserInfoPubkey
    );

    const amount = reward
      .add(firstUserInfo.pendingAmount)
      .add(firstUserInfo.stakedAmount);

    let tokenBalance = await utils.tokenBalance(firstTokenAccount);
    const expectedBalance = amount.addn(tokenBalance);

    await program.methods
      .claim(amount)
      .accounts({
        user: firstUser.publicKey,
        userInfo: firstUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
        stakingTokenAuthority,
        stakingTokenAccount,
        recipientTokenAccount: firstTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([firstUser])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(firstUserInfoPubkey);

    // Reward
    // Day 1: 5_000_000 * 20_000 * 2 / 20_000 = 10_000_000
    // Day 1: 5_000_000 * 20_000 * 2 / 40_000 = 5_000_000
    // Day 2: 5_000_000 * 20_000 * 2 / 40_000 = 5_000_000
    // Day 3: 5_000_000 * 20_000 * 2 / 40_000 = 5_000_000
    // Day 4: 5_000_000 * 20_000 * 2 / 70_000 = 2_857_142
    // Day 5: 5_000_000 * 20_000 * 2 / 70_000 = 2_857_142
    // Day 6: 5_000_000 * 20_000 * 2 / 70_000 = 2_857_142
    // Total: 33_571_426

    expectedFirstUserInfo.startDay = null;
    expectedFirstUserInfo.stakedAmount = new BN(0);
    expectedFirstUserInfo.rewardedAmount = new BN(0);
    expectedFirstUserInfo.pendingAmount = new BN(0);
    expectedFirstUserInfo.totalRewardedAmount = new BN(33_571_426);

    expectedStakingInfo.activeStakesNumber = new BN(2);
    expectedStakingInfo.totalRewardedAmount = new BN(61_142_852);

    stakingUtils.assertUserInfoEqual(userInfo, expectedFirstUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    tokenBalance = await utils.tokenBalance(firstTokenAccount);
    assert.equal(tokenBalance, expectedBalance.toNumber());
  });

  it("Day 11", async () => {
    await stakingUtils.waitUntil(program, startDay + 11);

    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      secondUserInfoPubkey,
      stakingInfoPubkey
    );

    const secondUserInfo = await program.account.userInfo.fetch(
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

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(secondUserInfoPubkey);

    // Reward
    // Day 8: 5_000_000 * 55_000 * 2 / 85_000 = 6_470_588
    // Day 9: 5_000_000 * 55_000 * 2 / 85_000 = 6_470_588
    // Total: 12_941_176

    expectedSecondUserInfo.startDay = null;
    expectedSecondUserInfo.stakedAmount = new BN(0);
    expectedSecondUserInfo.rewardedAmount = new BN(0);
    expectedSecondUserInfo.pendingAmount = new BN(0);
    expectedSecondUserInfo.totalRewardedAmount = new BN(40_512_602);

    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.totalRewardedAmount = new BN(74_084_028);

    stakingUtils.assertUserInfoEqual(userInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    tokenBalance = await utils.tokenBalance(secondTokenAccount);
    assert.equal(tokenBalance, expectedBalance.toNumber());
  });

  it("Day 14", async () => {
    await stakingUtils.waitUntil(program, startDay + 14);

    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      thirdUserInfoPubkey,
      stakingInfoPubkey
    );

    const thirdUserInfo = await program.account.userInfo.fetch(
      thirdUserInfoPubkey
    );

    const amount = reward
      .add(thirdUserInfo.pendingAmount)
      .add(thirdUserInfo.stakedAmount);

    let tokenBalance = await utils.tokenBalance(thirdTokenAccount);
    const expectedBalance = amount.addn(tokenBalance);

    await program.methods
      .claim(amount)
      .accounts({
        user: thirdUser.publicKey,
        userInfo: thirdUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
        stakingTokenAuthority,
        stakingTokenAccount,
        recipientTokenAccount: thirdTokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([thirdUser])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const userInfo = await program.account.userInfo.fetch(thirdUserInfoPubkey);

    // Reward
    // Day 4: 5_000_000 * 30_000 * 2 / 70_000 = 4_285_714
    // Day 5: 5_000_000 * 30_000 * 2 / 70_000 = 4_285_714
    // Day 6: 5_000_000 * 30_000 * 2 / 70_000 = 4_285_714
    // Day 7: 5_000_000 * 30_000 * 2 / 50_000 = 6_000_000
    // Day 8: 5_000_000 * 30_000 * 2 / 85_000 = 3_529_411
    // Day 9: 5_000_000 * 30_000 * 2 / 85_000 = 3_529_411
    // Total: 25_915_964

    expectedThirdUserInfo.startDay = null;
    expectedThirdUserInfo.stakedAmount = new BN(0);
    expectedThirdUserInfo.rewardedAmount = new BN(0);
    expectedThirdUserInfo.pendingAmount = new BN(0);
    expectedThirdUserInfo.totalRewardedAmount = new BN(25_915_964);

    expectedStakingInfo.activeStakesNumber = new BN(0);
    expectedStakingInfo.totalRewardedAmount = new BN(99999992);

    stakingUtils.assertUserInfoEqual(userInfo, expectedThirdUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    tokenBalance = await utils.tokenBalance(thirdTokenAccount);
    assert.equal(tokenBalance, expectedBalance.toNumber());
  });
});
