import * as assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as utils from "../utils";
import * as stakingUtils from "./utils";
import { AnchorError, BN, Program } from "@project-serum/anchor";
import { ChillStaking } from "../../target/types/chill_staking";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
  TransactionInstruction,
} from "@solana/web3.js";
import {
  ASSOCIATED_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@project-serum/anchor/dist/cjs/utils/token";

describe("Staking simulation | Edge cases", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.ChillStaking as Program<ChillStaking>;
  const primaryWallet = Keypair.generate();

  let payer: Keypair;
  let chillMint: PublicKey;

  const stakingInfoKeypair = Keypair.generate();
  const stakingInfoPubkey = stakingInfoKeypair.publicKey;

  let createStakingAccountInstruction: TransactionInstruction;
  let stakingTokenAuthority: PublicKey;
  let stakingTokenAccount: PublicKey;

  const expectedStakingInfo = stakingUtils.getDefaultStakingInfo();
  const expectedUserInfo = stakingUtils.getDefaultUserInfo();

  const user = Keypair.generate();
  let tokenAccountAuthority: Keypair;
  let tokenAccount: PublicKey;
  let userInfoPubkey: PublicKey;

  const initialBalance = 200_000;
  const stakeAmount = 100_000;

  const totalDays = 10;
  let startDay: number;
  let startTime: BN;
  let endTime: BN;
  let minStakeSize: BN;

  let initializeAccounts: stakingUtils.InitializeAccounts;
  let stakeAccounts: stakingUtils.StakeAccounts;
  let claimAccounts: stakingUtils.ClaimAccounts;

  before(async () => {
    payer = await utils.keypairWithSol();
    chillMint = await utils.createMint(primaryWallet.publicKey, 9);

    [tokenAccountAuthority, tokenAccount] =
      await stakingUtils.createUserWithTokenAccount(
        chillMint,
        primaryWallet,
        initialBalance
      );

    const currentTime = await utils.getCurrentTime();
    startTime = new BN(currentTime + 5);
    endTime = startTime.addn(totalDays * stakingUtils.SEC_IN_DAY);
    minStakeSize = new BN(500);

    createStakingAccountInstruction =
      await stakingUtils.createStakingAccountInstruction(
        stakingInfoPubkey,
        payer,
        totalDays,
        program
      );
  });

  it("Try to initialize after start day", async () => {
    stakingTokenAuthority = await stakingUtils.getStakingAuthority(
      stakingInfoPubkey,
      program.programId
    );

    stakingTokenAccount = await utils.getAssociatedTokenAddress(
      stakingTokenAuthority,
      chillMint
    );

    initializeAccounts = {
      primaryWallet: primaryWallet.publicKey,
      payer: payer.publicKey,
      stakingInfo: stakingInfoPubkey,
      stakingTokenAuthority,
      stakingTokenAccount,
      mint: chillMint,
      rent: SYSVAR_RENT_PUBKEY,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    };

    const currentTime = await utils.getCurrentTime();
    const wrongStartTime = new BN(currentTime - 5);

    await assert.rejects(
      async () => {
        await program.methods
          .initialize({ startTime: wrongStartTime, endTime, minStakeSize })
          .accounts(initializeAccounts)
          .preInstructions([createStakingAccountInstruction])
          .signers([primaryWallet, payer, stakingInfoKeypair])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "StakingMustStartInFuture");
        return true;
      }
    );
  });

  it("Try to initialize with endDay <= startDay", async () => {
    await assert.rejects(async () => {
      await program.methods
        .initialize({ startTime: endTime, endTime: startTime, minStakeSize })
        .accounts(initializeAccounts)
        .preInstructions([createStakingAccountInstruction])
        .signers([primaryWallet, payer, stakingInfoKeypair])
        .rpc();
    });
  });

  it("Initialize", async () => {
    await program.methods
      .initialize({ startTime, endTime, minStakeSize })
      .accounts(initializeAccounts)
      .preInstructions([createStakingAccountInstruction])
      .signers([primaryWallet, payer, stakingInfoKeypair])
      .rpc();

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedStakingInfo.primaryWallet = primaryWallet.publicKey;
    expectedStakingInfo.mint = chillMint;
    expectedStakingInfo.startDay = stakingInfo.startDay;
    expectedStakingInfo.endDay = stakingInfo.startDay.addn(totalDays);
    expectedStakingInfo.minStakeSize = minStakeSize;

    startDay = stakingInfo.startDay.toNumber();
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Try to initialize twice", async () => {
    await assert.rejects(async () => {
      await program.methods
        .initialize({ startTime, endTime, minStakeSize })
        .accounts(initializeAccounts)
        .signers([primaryWallet, payer])
        .rpc();
    });
  });

  it("Add reward tokens", async () => {
    const rewardTokensAmount = 100_000_000;
    await stakingUtils.addRewardTokens(
      rewardTokensAmount,
      primaryWallet,
      chillMint,
      stakingInfoPubkey,
      program
    );

    expectedStakingInfo.rewardTokensAmount = new BN(rewardTokensAmount);

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Try to stake before start", async () => {
    userInfoPubkey = await stakingUtils.getUserInfoPubkey(
      user.publicKey,
      stakingInfoPubkey,
      program.programId
    );

    stakeAccounts = {
      user: user.publicKey,
      payer: payer.publicKey,
      tokenAccountAuthority: tokenAccountAuthority.publicKey,
      userInfo: userInfoPubkey,
      fromTokenAccount: tokenAccount,
      stakingInfo: stakingInfoPubkey,
      stakingTokenAuthority,
      stakingTokenAccount,
      systemProgram: SystemProgram.programId,
      tokenProgram: TOKEN_PROGRAM_ID,
    };

    await assert.rejects(
      async () => {
        await program.methods
          .stake(new BN(stakeAmount))
          .accounts(stakeAccounts)
          .signers([user, payer, tokenAccountAuthority])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "StakingIsNotStarted");
        return true;
      }
    );
  });

  it("Try to boost before stake", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .boost()
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "AccountNotInitialized");
        return true;
      }
    );
  });

  it("Wait for start", async () => {
    await stakingUtils.waitUntil(program, startDay);
  });

  it("Try to add reward tokens after start", async () => {
    await assert.rejects(
      async () => {
        await stakingUtils.addRewardTokens(
          1,
          primaryWallet,
          chillMint,
          stakingInfoPubkey,
          program
        );
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "StakingIsAlreadyStarted");
        return true;
      }
    );
  });

  it("Try to stake zero tokens", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .stake(new BN(0))
          .accounts(stakeAccounts)
          .signers([user, payer, tokenAccountAuthority])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "StakeZeroTokens");
        return true;
      }
    );
  });

  it("Try to small amount of tokens", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .stake(minStakeSize.subn(1))
          .accounts(stakeAccounts)
          .signers([user, payer, tokenAccountAuthority])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "SmallStakeSize");
        return true;
      }
    );
  });

  it("Stake once", async () => {
    await program.methods
      .stake(new BN(stakeAmount))
      .accounts(stakeAccounts)
      .signers([user, payer, tokenAccountAuthority])
      .rpc();

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedStakingInfo.totalStakedAmount = new BN(stakeAmount);
    expectedStakingInfo.lastDailyReward = new BN(5_000_000);
    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.totalStakesNumber = new BN(1);
    expectedStakingInfo.lastUpdateDay = new BN(startDay);
    expectedStakingInfo.lastDayWithStake = new BN(startDay);

    expectedUserInfo.user = user.publicKey;
    expectedUserInfo.stakingInfo = stakingInfoPubkey;
    expectedUserInfo.bump = userInfo.bump;
    expectedUserInfo.startDay = new BN(startDay);
    expectedUserInfo.stakedAmount = new BN(stakeAmount);
    expectedUserInfo.dailyStakingReward = new BN(5_000_000);
    expectedUserInfo.totalStakedAmount = new BN(stakeAmount);

    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
  });

  it("Try to stake zero tokens with active stake", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .stake(new BN(0))
          .accounts(stakeAccounts)
          .signers([user, payer, tokenAccountAuthority])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "AddZeroTokensToPendingAmount");
        return true;
      }
    );
  });

  it("Stake twice", async () => {
    await stakingUtils.waitUntil(program, startDay + 1);

    await program.methods
      .stake(new BN(stakeAmount))
      .accounts(stakeAccounts)
      .signers([user, payer, tokenAccountAuthority])
      .rpc();

    expectedUserInfo.pendingAmount = new BN(stakeAmount);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 1);

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
  });

  it("Boost", async () => {
    await stakingUtils.waitUntil(program, startDay + 2);
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
    assert.equal(currentDay.toNumber(), startDay + 2);

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedUserInfo.totalBoostNumber = new BN(1);
    expectedStakingInfo.totalBoostNumber = new BN(1);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 2);

    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
  });

  it("Try to boost twice in a day", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .boost()
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "AlreadyBoosted");
        return true;
      }
    );
  });

  it("Check boosted days array", async () => {
    const boostedDays: Array<boolean> = await program.methods
      .viewBoostedDaysList()
      .accounts({
        userInfo: userInfoPubkey,
      })
      .view();

    assert.deepEqual(boostedDays, [
      false,
      false,
      true,
      false,
      false,
      false,
      false,
    ]);
  });

  it("Claim pending amount", async () => {
    await stakingUtils.waitUntil(program, startDay + 4);

    let userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    let tokenBalance = await utils.tokenBalance(tokenAccount);

    const tokenExpectedBalance =
      tokenBalance + userInfo.pendingAmount.toNumber();

    claimAccounts = {
      user: user.publicKey,
      userInfo: userInfoPubkey,
      recipientTokenAccount: tokenAccount,
      stakingInfo: stakingInfoPubkey,
      stakingTokenAuthority,
      stakingTokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    };

    await program.methods
      .claim(userInfo.pendingAmount)
      .accounts(claimAccounts)
      .signers([user])
      .rpc();

    userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedUserInfo.pendingAmount = new BN(0);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 4);

    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    tokenBalance = await utils.tokenBalance(tokenAccount);
    assert.equal(tokenBalance, tokenExpectedBalance);
  });

  it("Try to claim something else", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .claim(new BN(1))
          .accounts(claimAccounts)
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "InsufficientFunds");
        return true;
      }
    );
  });

  it("Check the reward before the stake expires", async () => {
    let currentDay = await stakingUtils.getCurrentDay(program);
    let reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      userInfoPubkey,
      stakingInfoPubkey
    );

    while (currentDay.toNumber() < startDay + 7) {
      assert.equal(reward.toNumber(), 0);
      reward = await stakingUtils.getUserRewardFromSimulation(
        program,
        userInfoPubkey,
        stakingInfoPubkey
      );

      await stakingUtils.waitForDay(program);
      currentDay = await stakingUtils.getCurrentDay(program);
    }
  });

  it("Check the reward after the stake expires", async () => {
    await stakingUtils.waitUntil(program, startDay + 7);

    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      userInfoPubkey,
      stakingInfoPubkey
    );

    assert.equal(reward.toNumber(), 7 * 5_000_000 + 5_000_000);
  });

  it("Try to claim zero tokens", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .claim(new BN(0))
          .accounts(claimAccounts)
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "WithdrawZeroTokens");
        return true;
      }
    );
  });

  it("Transfer part of reward to pending amount", async () => {
    await program.methods
      .transferRewardToPendingAmount(new BN(20_000_000))
      .accounts({
        user: user.publicKey,
        userInfo: userInfoPubkey,
        stakingInfo: stakingInfoPubkey,
      })
      .signers([user])
      .rpc();

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedStakingInfo.activeStakesNumber = new BN(0);
    expectedStakingInfo.totalRewardedAmount = new BN(40_000_000);
    expectedStakingInfo.totalUnspentAmount = new BN(30_000_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 7);

    expectedUserInfo.startDay = null;
    expectedUserInfo.stakedAmount = new BN(0);
    expectedUserInfo.rewardedAmount = new BN(20_000_000);
    expectedUserInfo.pendingAmount = new BN(20_100_000);
    expectedUserInfo.totalRewardedAmount = new BN(40_000_000);

    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Claim all rewards", async () => {
    let userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const reward = userInfo.rewardedAmount;

    let tokenBalance = await utils.tokenBalance(tokenAccount);
    const expectedBalance = tokenBalance + reward.toNumber();

    await program.methods
      .claim(reward)
      .accounts(claimAccounts)
      .signers([user])
      .rpc();

    userInfo = await program.account.userInfo.fetch(userInfoPubkey);

    expectedUserInfo.rewardedAmount = new BN(0);
    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);

    tokenBalance = await utils.tokenBalance(tokenAccount);
    assert.equal(tokenBalance, expectedBalance);
  });

  it("Claim some pending tokens", async () => {
    let tokenBalance = await utils.tokenBalance(tokenAccount);
    const expectedBalance = tokenBalance + 100_000;

    await program.methods
      .claim(new BN(100_000))
      .accounts(claimAccounts)
      .signers([user])
      .rpc();

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);

    expectedUserInfo.pendingAmount = new BN(20_000_000);
    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);

    tokenBalance = await utils.tokenBalance(tokenAccount);
    assert.equal(tokenBalance, expectedBalance);
  });

  it("Try to transfer rewarded tokens to pending amount", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .transferRewardToPendingAmount(new BN(1))
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "InsufficientFunds");
        return true;
      }
    );
  });

  it("Try to boost without active stake", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .boost()
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "NoActiveStake");
        return true;
      }
    );
  });

  it("Try to cancel without active stake", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .cancel()
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "NoActiveStake");
        return true;
      }
    );
  });

  it("Stake with only pending amount", async () => {
    await stakingUtils.waitUntil(program, startDay + 8);

    await program.methods
      .stake(new BN(0))
      .accounts(stakeAccounts)
      .signers([user, payer, tokenAccountAuthority])
      .rpc();

    const currentDay = await stakingUtils.getCurrentDay(program);
    assert.equal(currentDay.toNumber(), startDay + 8);

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedUserInfo.startDay = new BN(startDay + 8);
    expectedUserInfo.pendingAmount = new BN(0);
    expectedUserInfo.stakedAmount = new BN(20_000_000);
    expectedUserInfo.dailyStakingReward = new BN(15_000_000);
    expectedUserInfo.totalStakedAmount = new BN(20_100_000);

    expectedStakingInfo.activeStakesNumber = new BN(1);
    expectedStakingInfo.totalDaysWithNoReward = new BN(1);
    expectedStakingInfo.totalUnspentAmount = new BN(40_000_000);
    expectedStakingInfo.lastUpdateDay = new BN(startDay + 8);
    expectedStakingInfo.lastDayWithStake = new BN(startDay + 8);
    expectedStakingInfo.lastDailyReward = new BN(15_000_000);
    expectedStakingInfo.totalStakedAmount = new BN(20_100_000);
    expectedStakingInfo.totalStakesNumber = new BN(2);

    // (unspentBoostedAmount + 10_000_000 * days_without_stake) / 2 days = 20_000_000
    expectedStakingInfo.dailyUnspentReward = new BN(20_000_000);

    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Try to claim with zero balance", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .claim(new BN(1))
          .accounts(claimAccounts)
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "InsufficientFunds");
        return true;
      }
    );
  });

  it("Wait for 10th day", async () => {
    await stakingUtils.waitUntil(program, startDay + 10);
  });

  it("Try to cancel stake", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .cancel()
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "NoActiveStake");
        return true;
      }
    );
  });

  it("Try to transfer reward to pending after staking end day", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .transferRewardToPendingAmount(new BN(1))
          .accounts({
            user: user.publicKey,
            userInfo: userInfoPubkey,
            stakingInfo: stakingInfoPubkey,
          })
          .signers([user])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "StakingIsFinished");
        return true;
      }
    );
  });

  it("Claim reward amount", async () => {
    const reward = await stakingUtils.getUserRewardFromSimulation(
      program,
      userInfoPubkey,
      stakingInfoPubkey
    );

    await program.methods
      .claim(reward)
      .accounts(claimAccounts)
      .signers([user])
      .rpc();

    const userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    expectedUserInfo.startDay = null;
    expectedUserInfo.stakedAmount = new BN(0);
    expectedUserInfo.rewardedAmount = new BN(0);
    expectedUserInfo.pendingAmount = new BN(20_000_000);
    expectedUserInfo.totalRewardedAmount = new BN(70_000_000);

    expectedStakingInfo.activeStakesNumber = new BN(0);
    expectedStakingInfo.totalRewardedAmount = new BN(70_000_000);
    expectedStakingInfo.totalUnspentAmount = new BN(70_000_000);

    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Try to stake when staking is finished", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .stake(new BN(1))
          .accounts(stakeAccounts)
          .signers([user, payer, tokenAccountAuthority])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "StakingIsFinished");
        return true;
      }
    );
  });

  it("Claim all pending", async () => {
    let userInfo = await program.account.userInfo.fetch(userInfoPubkey);

    await program.methods
      .claim(userInfo.pendingAmount)
      .accounts(claimAccounts)
      .signers([user])
      .rpc();

    expectedUserInfo.pendingAmount = new BN(0);

    userInfo = await program.account.userInfo.fetch(userInfoPubkey);
    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    stakingUtils.assertUserInfoEqual(userInfo, expectedUserInfo);
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);
  });

  it("Redeem remainings", async () => {
    let stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    const remainings = stakingInfo.rewardTokensAmount.sub(
      stakingInfo.totalRewardedAmount
    );

    let tokenBalance = await utils.tokenBalance(tokenAccount);
    const expectedBalace = tokenBalance + remainings.toNumber();

    await program.methods
      .redeemRemainingRewardTokens(remainings)
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        stakingInfo: stakingInfoPubkey,
        stakingTokenAuthority,
        stakingTokenAccount,
        recipientTokenAccount: tokenAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([primaryWallet])
      .rpc();

    stakingInfo = await program.account.stakingInfo.fetch(stakingInfoPubkey);

    expectedStakingInfo.rewardTokensAmount = stakingInfo.totalRewardedAmount;
    stakingUtils.assertStakingInfoEqual(stakingInfo, expectedStakingInfo);

    tokenBalance = await utils.tokenBalance(tokenAccount);
    assert.equal(tokenBalance, expectedBalace);
  });

  it("Try to redeem again", async () => {
    await assert.rejects(
      async () => {
        await program.methods
          .redeemRemainingRewardTokens(new BN(1))
          .accounts({
            primaryWallet: primaryWallet.publicKey,
            stakingInfo: stakingInfoPubkey,
            stakingTokenAuthority,
            stakingTokenAccount,
            recipientTokenAccount: tokenAccount,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .signers([primaryWallet])
          .rpc();
      },
      (err: AnchorError) => {
        assert.equal(err.error.errorCode.code, "InsufficientFunds");
        return true;
      }
    );
  });

  it("Close userInfo", async () => {
    await program.methods
      .closeUserInfo()
      .accounts({
        user: user.publicKey,
        userInfo: userInfoPubkey,
        recipient: user.publicKey,
      })
      .signers([user])
      .rpc();

    await assert.rejects(async () => {
      await program.account.userInfo.fetch(userInfoPubkey);
    });
  });

  it("Close stakingInfo", async () => {
    await program.methods
      .closeStakingInfo()
      .accounts({
        primaryWallet: primaryWallet.publicKey,
        stakingInfo: stakingInfoPubkey,
        recipient: primaryWallet.publicKey,
      })
      .signers([primaryWallet])
      .rpc();

    await assert.rejects(async () => {
      await program.account.stakingInfo.fetch(stakingInfoPubkey);
    });
  });
});
