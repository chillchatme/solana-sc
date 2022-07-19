import * as assert from "assert";
import * as anchor from "@project-serum/anchor";
import * as utils from "../utils";
import * as stakingUtils from "./utils";
import { Program } from "@project-serum/anchor";
import { ChillStaking } from "../../target/types/chill_staking";
import { Keypair, PublicKey } from "@solana/web3.js";

describe("Staking simulation | No stakes", () => {
  anchor.setProvider(anchor.AnchorProvider.env());
  const program = anchor.workspace.ChillStaking as Program<ChillStaking>;
  const primaryWallet = Keypair.generate();

  let payer: Keypair;
  let chillMint: PublicKey;
  let stakingInfoPubkey: PublicKey;
  let startDay: number;

  const totalDays = 5;
  const rewardTokensAmount = 100_000_000;

  before(async () => {
    payer = await utils.keypairWithSol();
    chillMint = await utils.createMint(primaryWallet.publicKey, 9);

    stakingInfoPubkey = await stakingUtils.initializeStaking(
      primaryWallet,
      payer,
      totalDays,
      chillMint,
      program
    );

    const stakingInfo = await program.account.stakingInfo.fetch(
      stakingInfoPubkey
    );

    startDay = stakingInfo.startDay.toNumber();
    await stakingUtils.addRewardTokens(
      rewardTokensAmount,
      primaryWallet,
      chillMint,
      stakingInfoPubkey,
      program
    );
  });

  for (let i = 0; i < totalDays; i++) {
    it("Day " + i.toString(), async () => {
      await stakingUtils.waitUntil(program, startDay + i);

      const dailyReward = await stakingUtils.getDailyRewardFromSimulation(
        program,
        stakingInfoPubkey
      );

      assert.equal(
        dailyReward.toNumber(),
        Math.floor(100_000_000 / (2 * (totalDays - i)))
      );
    });
  }

  it("Day " + totalDays.toString(), async () => {
    await stakingUtils.waitUntil(program, startDay + totalDays);
    const dailyReward = await stakingUtils.getDailyRewardFromSimulation(
      program,
      stakingInfoPubkey
    );

    assert.equal(dailyReward.toNumber(), 0);
  });
});
