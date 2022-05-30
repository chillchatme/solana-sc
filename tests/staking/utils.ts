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

export const SEC_IN_DAY = 3;

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
    activeStakesNumber: new BN(0),
    dailyUnspentReward: new BN(0),
    endDay: new BN(0),
    lastDailyReward: new BN(0),
    lastDayWithStake: new BN(0),
    lastUpdateDay: new BN(0),
    minStakeSize: new BN(0),
    mint: PublicKey.default,
    primaryWallet: PublicKey.default,
    rewardTokensAmount: new BN(0),
    rewardedUnspentAmount: new BN(0),
    startDay: new BN(0),
    totalBoostNumber: new BN(0),
    totalCancelNumber: new BN(0),
    totalDaysWithNoReward: new BN(0),
    totalRewardedAmount: new BN(0),
    totalStakedAmount: new BN(0),
    totalStakesNumber: new BN(0),
    totalUnspentAmount: new BN(0),
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
    totalBoostNumber: new BN(0),
  };
}

export function assertStakingInfoEqual(
  stakingInfo: StakingInfo,
  expectedStakingInfo: StakingInfo
) {
  const tmpStakingInfo = {
    activeStakesNumber: stakingInfo.activeStakesNumber,
    dailyUnspentReward: stakingInfo.dailyUnspentReward,
    endDay: stakingInfo.endDay,
    lastDailyReward: stakingInfo.lastDailyReward,
    lastDayWithStake: stakingInfo.lastDayWithStake,
    lastUpdateDay: stakingInfo.lastUpdateDay,
    mint: stakingInfo.mint,
    primaryWallet: stakingInfo.primaryWallet,
    rewardTokensAmount: stakingInfo.rewardTokensAmount,
    rewardedUnspentAmount: stakingInfo.rewardedUnspentAmount,
    startDay: stakingInfo.startDay,
    totalBoostNumber: stakingInfo.totalBoostNumber,
    totalDaysWithNoReward: stakingInfo.totalDaysWithNoReward,
    totalCancelNumber: stakingInfo.totalCancelNumber,
    totalRewardedAmount: stakingInfo.totalRewardedAmount,
    totalStakedAmount: stakingInfo.totalStakedAmount,
    totalStakesNumber: stakingInfo.totalStakesNumber,
    totalUnspentAmount: stakingInfo.totalUnspentAmount,
  };

  const tmpExptectedStakingInfo = {
    activeStakesNumber: expectedStakingInfo.activeStakesNumber,
    dailyUnspentReward: expectedStakingInfo.dailyUnspentReward,
    endDay: expectedStakingInfo.endDay,
    lastDailyReward: expectedStakingInfo.lastDailyReward,
    lastDayWithStake: expectedStakingInfo.lastDayWithStake,
    lastUpdateDay: expectedStakingInfo.lastUpdateDay,
    mint: expectedStakingInfo.mint,
    primaryWallet: expectedStakingInfo.primaryWallet,
    rewardTokensAmount: expectedStakingInfo.rewardTokensAmount,
    rewardedUnspentAmount: expectedStakingInfo.rewardedUnspentAmount,
    startDay: expectedStakingInfo.startDay,
    totalBoostNumber: expectedStakingInfo.totalBoostNumber,
    totalDaysWithNoReward: expectedStakingInfo.totalDaysWithNoReward,
    totalCancelNumber: expectedStakingInfo.totalCancelNumber,
    totalRewardedAmount: expectedStakingInfo.totalRewardedAmount,
    totalStakedAmount: expectedStakingInfo.totalStakedAmount,
    totalStakesNumber: expectedStakingInfo.totalStakesNumber,
    totalUnspentAmount: expectedStakingInfo.totalUnspentAmount,
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
    totalBoostNumber: userInfo.totalBoostNumber,
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
    totalBoostNumber: expectedUserInfo.totalBoostNumber,
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
  const endTime = startTime.addn(totalDays * SEC_IN_DAY);
  const minStakeSize = new BN(0);

  await program.methods
    .initialize({ startTime, endTime, minStakeSize })
    .accounts({
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
      tokenAccountAuthority: tokenAuthority.publicKey,
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
  return await program.methods.viewCurrentDayNumber().view({
    skipPreflight: true,
  });
}

export async function getDailyRewardFromSimulation(
  program: Program<ChillStaking>,
  stakingInfo: PublicKey
): Promise<BN> {
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
  while (currentDay.toNumber() < day) {
    await pause(ms);
    currentDay = await getCurrentDay(program);
  }
}

export async function waitForSomeDays(
  program: Program<ChillStaking>,
  days: number,
  ms?: number
): Promise<void> {
  const currentDay = await getCurrentDay(program);
  const expectedDay = currentDay.addn(days);
  await waitUntil(program, expectedDay.toNumber(), ms);
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
