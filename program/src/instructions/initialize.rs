use crate::{
    AssociatedTokenAccount, Discriminator, MintAccount, PinocchioError, ProgramAccount, Schedule,
    SignerAccount,
};
use core::mem::size_of;
use pinocchio::{
    account_info::AccountInfo,
    instruction::Seed,
    program_error::ProgramError,
    sysvars::{clock::Clock, Sysvar},
    ProgramResult,
};

pub struct InitializeAccounts<'a> {
    pub authority: &'a AccountInfo, //signer
    pub schedule: &'a AccountInfo,
    pub mint: &'a AccountInfo,  //mint
    pub vault: &'a AccountInfo, //ata
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}
impl<'a> TryFrom<&'a [AccountInfo]> for InitializeAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, schedule, mint, vault, system_program, token_program, associated_token_account_program] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(authority)?;
        MintAccount::check(mint)?;

        // do i need to check token and system programs account keys?
        // yes, we should check the the program keys that we cpi into otherwise an attacker could pass in malicious program accounts and cause havoc
        ProgramAccount::check_program(system_program, &pinocchio_system::ID)?;
        ProgramAccount::check_program(token_program, &pinocchio_token::ID)?;

        AssociatedTokenAccount::init_if_needed(
            vault,
            mint,
            authority,
            schedule,
            system_program,
            token_program,
        )?;

        Ok(Self {
            authority,
            schedule,
            mint,
            vault,
            system_program,
            token_program,
        })
    }
}
#[repr(C, packed)]
pub struct InitializeInstructionData {
    pub start_timestamp: i64,
    pub cliff_duration: i64,
    pub step_duration: i64,
    pub total_duration: i64,
    pub seed: u64,
    // be careful with passing in bumps through instruction data, here thanks to verify_seeds we are safe but in general its better to avoid passing bumps through instruction data and just calculate them on the fly especially since we are already doing find_program_addres so we don't waste any extra CUs
    pub bump: u8,
}
impl<'a> TryFrom<&'a [u8]> for InitializeInstructionData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != size_of::<InitializeInstructionData>() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let start_timestamp = i64::from_le_bytes(data[0..8].try_into().unwrap());
        let cliff_duration = i64::from_le_bytes(data[8..16].try_into().unwrap());
        let step_duration = i64::from_le_bytes(data[16..24].try_into().unwrap());
        let total_duration = i64::from_le_bytes(data[24..32].try_into().unwrap());
        let seed = u64::from_le_bytes(data[32..40].try_into().unwrap());
        let bump = u8::from_le_bytes(data[40..41].try_into().unwrap());

        let unix_timestamp = Clock::get()?.unix_timestamp;

        if start_timestamp < unix_timestamp {
            return Err(PinocchioError::StartTimeInvalid.into());
        }

        if total_duration == 0
            || step_duration == 0
            || (total_duration - cliff_duration) % step_duration != 0
        {
            return Err(PinocchioError::DurationInvalid.into());
        }

        Ok(Self {
            start_timestamp,
            cliff_duration,
            step_duration,
            total_duration,
            seed,
            bump,
        })
    }
}
pub struct Initialize<'a> {
    pub accounts: InitializeAccounts<'a>,
    pub instruction_data: InitializeInstructionData,
}
impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for Initialize<'a> {
    type Error = ProgramError;
    fn try_from(
        (instruction_data, accounts): (&'a [u8], &'a [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        let accounts = InitializeAccounts::try_from(accounts)?;
        let instruction_data = InitializeInstructionData::try_from(instruction_data)?;

        let seed_binding = instruction_data.seed.to_le_bytes();
        let seeds = [Seed::from(b"schedule"), Seed::from(&seed_binding)];

        ProgramAccount::verify_seeds(&seeds, accounts.schedule)?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}
impl<'a> Initialize<'a> {
    pub const DISCRIMINATOR: &'a u8 = &0;
    pub fn process(&mut self) -> ProgramResult {
        let seed_binding = self.instruction_data.seed.to_le_bytes();
        let bump_binding = [self.instruction_data.bump];
        let seeds = [
            Seed::from(b"schedule"),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding),
        ];

        ProgramAccount::init::<Schedule>(
            self.accounts.authority,
            self.accounts.schedule,
            &seeds,
            Schedule::LEN,
        )?;

        let mut schedule_state = Schedule::load_mut(self.accounts.schedule)?;
        schedule_state.set_inner(
            *self.accounts.mint.key(),
            *self.accounts.authority.key(),
            self.instruction_data.seed,
            self.instruction_data.start_timestamp,
            self.instruction_data.cliff_duration,
            self.instruction_data.step_duration,
            self.instruction_data.total_duration,
            self.instruction_data.bump,
        )?;

        Ok(())
    }
}
