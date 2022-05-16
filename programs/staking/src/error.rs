use anchor_lang::prelude::*;

#[error_code]
pub enum ErrorCode {
    #[msg("Wrong vector size")]
    WrongVectorSize,

    #[msg("Out of bounds")]
    OutOfBounds,

    #[msg("Max vector size has been reached")]
    MaxSizeReached,

    #[msg("Staking is finished")]
    StakingIsFinished,

    #[msg("Staking is not finished yet")]
    StakingIsNotFinished,
}
