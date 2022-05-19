use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Wrong vector size")]
    WrongVectorSize,

    #[msg("Out of bounds")]
    OutOfBounds,

    #[msg("Max vector size has been reached")]
    MaxSizeReached,

    #[msg("Already boosted today")]
    AlreadyBoosted,

    #[msg("User doesn't have active stake")]
    NoActiveStake,

    #[msg("Staking is finished")]
    StakingIsFinished,

    #[msg("Staking is not finished yet")]
    StakingIsNotFinished,

    #[msg("Adding zero tokens to pending amount")]
    AddZeroTokensToPendingAmount,

    #[msg("Stake zero tokens")]
    StakeZeroTokens,

    #[msg("Withdraw zero tokens")]
    WithdrawZeroTokens,
}
