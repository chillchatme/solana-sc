import * as assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as utils from "../utils";
import * as stakingUtils from "./utils";
import { BN, Program } from "@project-serum/anchor";
import { ChillStaking } from "../../target/types/chill_staking";
import { Keypair, PublicKey, SystemProgram } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@project-serum/anchor/dist/cjs/utils/token";

describe("Staking simulation | Staking with skips", () => {
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
  // 0               0            0
  // 1         +20_000            0
  // 2               0            0
  // 3           Boost            0
  // 4               0            0
  // 5               0            0
  // 6               0            0
  // 7               0            0
  // 8       Claim all            0
  // 9               0      +20_000
  // 10              0    Claim all

  it("Day 1", async () => {
    await stakingUtils.waitUntil(program, startDay + 1);
    await program.methods
      .stake(new BN(20_000))
      .accounts({
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
      })
      .signers([firstUser, payer])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const firstUserInfo = await program.account.userInfo.fetch(
      firstUserInfoPubkey
    );

    expectedFirstUserInfo.bump = firstUserInfo.bump;
    expectedFirstUserInfo.dailyStakingReward = new BN(5_555_555);
    expectedFirstUserInfo.stakedAmount = new BN(20_000);
    expectedFirstUserInfo.startDay = new BN(startDay + 1);
    expectedFirstUserInfo.totalStakedAmount = new BN(20_000);

    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.dailyUnspentReward = new BN(1_111_111);
    expectedStakingInfo.lastDailyReward = new BN(5_555_555);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 1);
    expectedStakingInfo.lastDayWithStake = new BN(startDay + 1);
    expectedStakingInfo.totalDaysWithNoReward = new BN(1);
    expectedStakingInfo.totalStakedAmount = new BN(20_000);
    expectedStakingInfo.totalStakesNumber = new BN(1);
    expectedStakingInfo.totalUnspentAmount = new BN(10_000_000);

    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 3", async () => {
    await stakingUtils.waitUntil(program, startDay + 3);

    await program.methods
      .boost()
      .accounts({
        user: firstUser.publicKey,
        userInfo: firstUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([firstUser])
      .rpc();

    const userInfo = await program.account.userInfo.fetch(firstUserInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedFirstUserInfo.totalBoostNumber = new BN(1);
    expectedStakingInfo.totalBoostNumber = new BN(1);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 3);
    expectedStakingInfo.rewardedUnspentAmount = new BN(2_222_222);

    stakingUtils.assertUserInfoEqual(userInfo, expectedFirstUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 8", async () => {
    await stakingUtils.waitUntil(program, startDay + 8);

    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      firstUserInfoPubkey,
      stakingInfoPubkey
    );

    let firstUserInfo = await program.account.userInfo.fetch(
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

    tokenBalance = await utils.tokenBalance(firstTokenAccount);
    assert.equal(tokenBalance, expectedBalance.toNumber());

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    firstUserInfo = await program.account.userInfo.fetch(firstUserInfoPubkey);

    expectedFirstUserInfo.startDay = null;
    expectedFirstUserInfo.stakedAmount = new BN(0);
    expectedFirstUserInfo.rewardedAmount = new BN(0);
    expectedFirstUserInfo.pendingAmount = new BN(0);
    expectedFirstUserInfo.totalRewardedAmount = new BN(44_444_440);

    expectedStakingInfo.activeStakesNumber = new BN(0);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 8);
    expectedStakingInfo.rewardedUnspentAmount = new BN(7_777_777);
    expectedStakingInfo.totalRewardedAmount = new BN(44_444_440);
    expectedStakingInfo.totalUnspentAmount = new BN(43_333_330);

    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
    stakingUtils.assertUserInfoEqual(firstUserInfo, expectedFirstUserInfo);
  });

  it("Day 9", async () => {
    await stakingUtils.waitUntil(program, startDay + 9);

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

    let stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    let secondUserInfo = await program.account.userInfo.fetch(
      secondUserInfoPubkey
    );

    // (100_000_000 - 5_555_555 * 7 - 5_555_555) / 2 = 27_777_780
    // But due to integer calculations, the daily staking reward = 27_777_776
    //
    // Calculations with fractional numbers:
    // (100_000_000 - 5_555_555.555555 * 8) / 2 = 27_777_777
    // (100_000_000 - 9 * 10_000_000 + 2 * 10_000_000 + 5_555_555.555555 * 6 - 1_111_111.111111 * 7) / 2 = 27_777_777
    //
    // Calculations with integer numbers:
    // (100_000_000 - 9 * 10_000_000 + 2 * 10_000_000 + 5_555_555 * 6 - 1_111_111 * 7) / 2 = 27_777_776
    expectedSecondUserInfo.bump = secondUserInfo.bump;
    expectedSecondUserInfo.dailyStakingReward = new BN(27_777_776);
    expectedSecondUserInfo.stakedAmount = new BN(20_000);
    expectedSecondUserInfo.startDay = new BN(startDay + 9);
    expectedStakingInfo.lastDayWithStake = new BN(startDay + 9);
    expectedSecondUserInfo.totalStakedAmount = new BN(20_000);

    // totalUnspentAmount = 2 * 10_000_000 + 6 * 5_555_555 = 53_333_330
    // dailyUnspentReward = (53_333_330 - 7_777_777) / (10 - 9) = 45_555_553
    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.dailyUnspentReward = new BN(45_555_553);
    expectedStakingInfo.totalUnspentAmount = new BN(53_333_330);
    expectedStakingInfo.lastDailyReward = new BN(27_777_776);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 9);
    expectedStakingInfo.totalDaysWithNoReward = new BN(2);
    expectedStakingInfo.totalStakedAmount = new BN(40_000);
    expectedStakingInfo.totalStakesNumber = new BN(2);

    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    await program.methods
      .boost()
      .accounts({
        user: secondUser.publicKey,
        userInfo: secondUserInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([secondUser])
      .rpc();

    stakingInfo = await program.account.stakingInfo.fetch(stakingInfoPubkey);

    secondUserInfo = await program.account.userInfo.fetch(secondUserInfoPubkey);

    expectedSecondUserInfo.totalBoostNumber = new BN(1);
    expectedStakingInfo.totalBoostNumber = new BN(2);

    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Day 10", async () => {
    await stakingUtils.waitUntil(program, startDay + 10);

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

    expectedSecondUserInfo.startDay = null;
    expectedSecondUserInfo.stakedAmount = new BN(0);
    expectedSecondUserInfo.rewardedAmount = new BN(0);
    expectedSecondUserInfo.pendingAmount = new BN(0);
    expectedSecondUserInfo.totalRewardedAmount = new BN(55_555_552);

    expectedStakingInfo.activeStakesNumber = new BN(0);
    expectedStakingInfo.totalRewardedAmount = new BN(99_999_992);

    stakingUtils.assertUserInfoEqual(secondUserInfo, expectedSecondUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });
});
