import * as assert from "assert";
import { Accounts, BN, Program } from "@project-serum/anchor";
import { TypeDef } from "@project-serum/anchor/dist/cjs/program/namespace/types";
import {
  ASSOCIATED_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
} from "@project-serum/anchor/dist/cjs/utils/token";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { ChillStaking } from "../../target/types/chill_staking";
import {
  createTokenAccount,
  getAssociatedTokenAddress,
  getCurrentTime,
  mintTokens,
} from "../utils";

export type StakingInfo = TypeDef<
  ChillStaking["accounts"][1],
  ChillStaking["accounts"][number]
>;

export type UserInfo = TypeDef<
  ChillStaking["accounts"][2],
  ChillStaking["accounts"][number]
>;

export type InitializeAccounts = Accounts<
  ChillStaking["instructions"][5]["accounts"][number]
>;

export type StakeAccounts = Accounts<
  ChillStaking["instructions"][10]["accounts"][number]
>;

export type ClaimAccounts = Accounts<
  ChillStaking["instructions"][12]["accounts"][number]
>;

export function getDefaultStakingInfo(): StakingInfo {
  return {
    primaryWallet: PublicKey.default,
    mint: PublicKey.default,
    startDay: new BN(0),
    endDay: new BN(0),
    lastDailyReward: new BN(0),
    lastUpdateDay: new BN(0),
    unspentBoostedAmount: new BN(0),
    dailyUnspentReward: new BN(0),
    rewardedUnspentAmount: new BN(0),
    daysWithoutStake: new BN(0),
    activeStakesNumber: new BN(0),
    rewardTokensAmount: new BN(0),
    totalBoostAmount: new BN(0),
    totalStakedAmount: new BN(0),
    totalStakesNumber: new BN(0),
    totalRewardedAmount: new BN(0),
  };
}

export function getDefaultUserInfo(): UserInfo {
  return {
    user: PublicKey.default,
    stakingInfo: PublicKey.default,
    bump: 0,
    startDay: new BN(0),
    stakedAmount: new BN(0),
    pendingAmount: new BN(0),
    dailyStakingReward: new BN(0),
    rewardedAmount: new BN(0),
    totalStakedAmount: new BN(0),
    totalRewardedAmount: new BN(0),
    totalBoostAmount: new BN(0),
  };
}

export function assertStakingInfoEqual(
  stakingInfo: StakingInfo,
  expectedStakingInfo: StakingInfo
) {
  const tmpStakingInfo = {
    primaryWallet: stakingInfo.primaryWallet,
    mint: stakingInfo.mint,
    startDay: stakingInfo.startDay,
    endDay: stakingInfo.endDay,
    lastDailyReward: stakingInfo.lastDailyReward,
    lastUpdateDay: stakingInfo.lastUpdateDay,
    unspentBoostedAmount: stakingInfo.unspentBoostedAmount,
    dailyUnspentReward: stakingInfo.dailyUnspentReward,
    rewardedUnspentAmount: stakingInfo.rewardedUnspentAmount,
    daysWithoutStake: stakingInfo.daysWithoutStake,
    activeStakesNumber: stakingInfo.activeStakesNumber,
    rewardTokensAmount: stakingInfo.rewardTokensAmount,
    totalBoostAmount: stakingInfo.totalBoostAmount,
    totalStakedAmount: stakingInfo.totalStakedAmount,
    totalStakesNumber: stakingInfo.totalStakesNumber,
    totalRewardedAmount: stakingInfo.totalRewardedAmount,
  };

  const tmpExptectedStakingInfo = {
    primaryWallet: expectedStakingInfo.primaryWallet,
    mint: expectedStakingInfo.mint,
    startDay: expectedStakingInfo.startDay,
    endDay: expectedStakingInfo.endDay,
    lastDailyReward: expectedStakingInfo.lastDailyReward,
    lastUpdateDay: expectedStakingInfo.lastUpdateDay,
    unspentBoostedAmount: expectedStakingInfo.unspentBoostedAmount,
    dailyUnspentReward: expectedStakingInfo.dailyUnspentReward,
    rewardedUnspentAmount: expectedStakingInfo.rewardedUnspentAmount,
    daysWithoutStake: expectedStakingInfo.daysWithoutStake,
    activeStakesNumber: expectedStakingInfo.activeStakesNumber,
    rewardTokensAmount: expectedStakingInfo.rewardTokensAmount,
    totalBoostAmount: expectedStakingInfo.totalBoostAmount,
    totalStakedAmount: expectedStakingInfo.totalStakedAmount,
    totalStakesNumber: expectedStakingInfo.totalStakesNumber,
    totalRewardedAmount: expectedStakingInfo.totalRewardedAmount,
  };

  assert.equal(
    JSON.stringify(tmpStakingInfo),
    JSON.stringify(tmpExptectedStakingInfo)
  );
}

export function assertUserInfoEqual(
  userInfo: UserInfo,
  expectedUserInfo: UserInfo
) {
  const tmpUserInfo = {
    user: userInfo.user,
    stakingInfo: userInfo.stakingInfo,
    bump: userInfo.bump,
    startDay: userInfo.startDay,
    stakedAmount: userInfo.stakedAmount,
    pendingAmount: userInfo.pendingAmount,
    dailyStakingReward: userInfo.dailyStakingReward,
    rewardedAmount: userInfo.rewardedAmount,
    totalStakedAmount: userInfo.totalStakedAmount,
    totalRewardedAmount: userInfo.totalRewardedAmount,
    totalBoostAmount: userInfo.totalBoostAmount,
  };

  const tmpExpectedUserInfo = {
    user: expectedUserInfo.user,
    stakingInfo: expectedUserInfo.stakingInfo,
    bump: expectedUserInfo.bump,
    startDay: expectedUserInfo.startDay,
    stakedAmount: expectedUserInfo.stakedAmount,
    pendingAmount: expectedUserInfo.pendingAmount,
    dailyStakingReward: expectedUserInfo.dailyStakingReward,
    rewardedAmount: expectedUserInfo.rewardedAmount,
    totalStakedAmount: expectedUserInfo.totalStakedAmount,
    totalRewardedAmount: expectedUserInfo.totalRewardedAmount,
    totalBoostAmount: expectedUserInfo.totalBoostAmount,
  };

  assert.equal(
    JSON.stringify(tmpUserInfo),
    JSON.stringify(tmpExpectedUserInfo)
  );
}

export async function initializeStaking(
  primaryWallet: Keypair,
  payer: Keypair,
  totalDays: number,
  chillMint: PublicKey,
  program: Program<ChillStaking>
): Promise<PublicKey> {
  const stakingInfoKeypair = Keypair.generate();
  const stakingInfoPubkey = stakingInfoKeypair.publicKey;

  const stakingTokenAuthority = await getStakingAuthority(
    stakingInfoPubkey,
    program.programId
  );

  const stakingTokenAccount = await getAssociatedTokenAddress(
    stakingTokenAuthority,
    chillMint
  );

  const currentTime = await getCurrentTime();
  const startTime = new BN(currentTime + 5);
  const endTime = startTime.addn(totalDays * 3);

  await program.methods
    .initialize({ startTime, endTime })
    .accounts({
      primaryWallet: primaryWallet.publicKey,
      payer: payer.publicKey,
      stakingInfo: stakingInfoPubkey,
      stakingTokenAuthority,
      stakingTokenAccount,
      chillMint,
      rent: SYSVAR_RENT_PUBKEY,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .signers([primaryWallet, payer, stakingInfoKeypair])
    .rpc();

  return stakingInfoPubkey;
}

export async function addRewardTokens(
  amount: number,
  primaryWallet: Keypair,
  chillMint: PublicKey,
  stakingInfo: PublicKey,
  program: Program<ChillStaking>
) {
  const tokenAuthority = Keypair.generate();
  const tokenAccount = await createTokenAccount(
    tokenAuthority.publicKey,
    chillMint
  );

  const stakingTokenAuthority = await getStakingAuthority(
    stakingInfo,
    program.programId
  );

  const stakingTokenAccount = await getAssociatedTokenAddress(
    stakingTokenAuthority,
    chillMint
  );

  await mintTokens(primaryWallet, chillMint, tokenAccount, amount);

  await program.methods
    .addRewardTokens(new BN(amount))
    .accounts({
      primaryWallet: primaryWallet.publicKey,
      tokenAuthority: tokenAuthority.publicKey,
      tokenAccount,
      stakingInfo,
      stakingTokenAuthority,
      stakingTokenAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .signers([primaryWallet, tokenAuthority])
    .rpc();
}

export async function getStakingAuthority(
  stakingInfo: PublicKey,
  programId: PublicKey
): Promise<PublicKey> {
  return (
    await PublicKey.findProgramAddress([stakingInfo.toBytes()], programId)
  )[0];
}

export async function getUserInfoPubkey(
  user: PublicKey,
  stakingInfo: PublicKey,
  programId: PublicKey
): Promise<PublicKey> {
  return (
    await PublicKey.findProgramAddress(
      [stakingInfo.toBytes(), user.toBytes()],
      programId
    )
  )[0];
}

export async function createUserWithTokenAccount(
  chillMint: PublicKey,
  mintAuthority: Keypair,
  initialBalance: number
): Promise<[Keypair, PublicKey]> {
  const user = Keypair.generate();
  const tokenAccount = await createTokenAccount(user.publicKey, chillMint);
  await mintTokens(mintAuthority, chillMint, tokenAccount, initialBalance);
  return [user, tokenAccount];
}

export async function pause(ms: number): Promise<void> {
  await new Promise((f) => setTimeout(f, ms));
}

export async function getCurrentDay(
  program: Program<ChillStaking>
): Promise<BN> {
  return await program.methods.viewCurrentDayNumber().view();
}

export async function getDailyRewardFromSimulation(
  program: Program<ChillStaking>,
  stakingInfo: PublicKey
): Promise<BN> {
  await program.methods
    .viewDailyStakingReward()
    .accounts({ stakingInfo })
    .rpc({ skipPreflight: true });

  return await program.methods
    .viewDailyStakingReward()
    .accounts({ stakingInfo })
    .view();
}

export async function getUserRewardFromSimulation(
  program: Program<ChillStaking>,
  userInfo: PublicKey,
  stakingInfo: PublicKey
): Promise<BN> {
  await program.methods
    .viewUserRewardAmount()
    .accounts({
      userInfo,
      stakingInfo,
    })
    .rpc({ skipPreflight: true });

  const info = await program.methods
    .viewUserRewardAmount()
    .accounts({
      userInfo,
      stakingInfo,
    })
    .view();

  return info;
}

export async function waitUntil(
  program: Program<ChillStaking>,
  day: number,
  ms?: number
): Promise<void> {
  if (ms == null) {
    ms = 100;
  }

  let currentDay = await getCurrentDay(program);
  while (currentDay < day) {
    await pause(ms);
    currentDay = await getCurrentDay(program);
  }
}

export async function waitForSomeDays(
  program: Program<ChillStaking>,
  days: number,
  ms?: number
): Promise<void> {
  const currentDay = await program.methods.viewCurrentDayNumber().view();
  const expectedDay = currentDay + days;
  await waitUntil(program, expectedDay, ms);
}

export async function waitForDay(
  program: Program<ChillStaking>,
  ms?: number
): Promise<void> {
  await waitForSomeDays(program, 1, ms);
}

export async function waitForWeek(
  program: Program<ChillStaking>,
  ms?: number
): Promise<void> {
  await waitForSomeDays(program, 7, ms);
}
