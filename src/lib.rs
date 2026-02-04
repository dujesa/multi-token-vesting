use pinocchio::{ProgramResult, account_info::AccountInfo, entrypoint, program_error::ProgramError, pubkey::Pubkey};

pub mod instructions;
pub use instructions::*;

pub mod errors;
pub use errors::*;

pub mod state;
pub use state::*;

entrypoint!(process_instruction);

//FwnGeaANDtRZHA1xXzjyTjr5mmEZtXBSKuA3umcRPiWG.
pub const ID: Pubkey = [
    0xde, 0x0c, 0x2a, 0xd8, 0xf6, 0xeb, 0x0d, 0x5a, 0x94, 0x92, 0x02, 0x79, 0x06, 0xfa, 0xcc, 0x62,
    0x60, 0xbb, 0x41, 0xca, 0xcd, 0xdd, 0x62, 0x68, 0x67, 0xb5, 0xe6, 0x8a, 0xfc, 0x26, 0xe0, 0x35,
];

fn process_instruction(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8]
) -> ProgramResult {
    match instruction_data.split_first() {
        Some((Initialize::DISCRIMINATOR, data)) => Initialize::try_from((data, accounts))?.process(),
        Some((AddParticipant::DISCRIMINATOR, data)) => AddParticipant::try_from((data, accounts))?.process(),
        Some((Claim::DISCRIMINATOR, _)) => Claim::try_from(accounts)?.process(),
        _ => Err(ProgramError::InvalidInstructionData)
    }
}