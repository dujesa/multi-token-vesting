use pinocchio::program_error::ProgramError;
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub enum PinocchioError {
    #[error("Invalid signer")]
    InvalidSigner,
    #[error("Invalid address")]
    InvalidAddress,
    #[error("Invalid seed")]
    InvalidSeed,
    #[error("Start time invalid")]
    StartTimeInvalid,
    #[error("Duration invalid")]
    DurationInvalid,
    #[error("Cannot claim before cliff")]
    CannotClaimBeforeCliff,
    #[error("Cannot add participant after cliff")]
    CannotAddParticipantAfterCliff,
    #[error("Cannot double claim")]
    CannotDoubleClaim,
    #[error("Invalid claim amount")]
    ClaimAmountInvalid,
    #[error("Claim amount overflowes allocated amount")]
    ClaimAmountOverflow,
}
impl From<PinocchioError> for ProgramError {
    fn from(value: PinocchioError) -> Self {
        ProgramError::Custom(value as u32)
    }
}