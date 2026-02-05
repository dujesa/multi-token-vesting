use core::mem::size_of;
use pinocchio::{
    account_info::AccountInfo, instruction::Seed, program_error::ProgramError, ProgramResult,
};
use pinocchio_token::{instructions::Transfer, state::TokenAccount};

use crate::{
    AssociatedTokenAccount, Discriminator, MintAccount, PinocchioError, ProgramAccount, Schedule,
    SignerAccount, VestedParticipant,
};

pub struct AddParticipantAccounts<'a> {
    pub authority: &'a AccountInfo,     //signer
    pub authority_ata: &'a AccountInfo, //signers ata
    pub vault: &'a AccountInfo,         //vault for allocations
    pub participant_wallet: &'a AccountInfo,
    pub vested_participant: &'a AccountInfo,
    pub schedule: &'a AccountInfo,
    pub mint: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
}
impl<'a> TryFrom<&'a [AccountInfo]> for AddParticipantAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [authority, authority_ata, vault, participant_wallet, vested_participant, schedule, mint, system_program, token_program] =
            accounts
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        SignerAccount::check(authority)?;
        ProgramAccount::check::<Schedule>(schedule)?;
        MintAccount::check(mint)?;

        Ok(Self {
            authority,
            authority_ata,
            vault,
            vested_participant,
            participant_wallet,
            schedule,
            mint,
            system_program,
            token_program,
        })
    }
}
#[repr(C, packed)]
pub struct AddParticipantInstructionData {
    pub token_allocation_amount: u64,
}
impl<'a> TryFrom<&'a [u8]> for AddParticipantInstructionData {
    type Error = ProgramError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        if data.len() != size_of::<AddParticipantInstructionData>() {
            return Err(ProgramError::InvalidInstructionData);
        }

        let token_allocation_amount = u64::from_le_bytes(data[0..8].try_into().unwrap());
        if token_allocation_amount == 0 {
            return Err(ProgramError::InvalidInstructionData);
        }

        Ok(Self {
            token_allocation_amount,
        })
    }
}
pub struct AddParticipant<'a> {
    pub accounts: AddParticipantAccounts<'a>,
    pub instruction_data: AddParticipantInstructionData,
}
impl<'a> TryFrom<(&'a [u8], &'a [AccountInfo])> for AddParticipant<'a> {
    type Error = ProgramError;
    fn try_from(
        (instruction_data, accounts): (&'a [u8], &'a [AccountInfo]),
    ) -> Result<Self, Self::Error> {
        let accounts = AddParticipantAccounts::try_from(accounts)?;
        let instruction_data = AddParticipantInstructionData::try_from(instruction_data)?;

        let schedule = Schedule::load(accounts.schedule)?;

        if schedule.is_cliff_completed() {
            return Err(PinocchioError::CannotAddParticipantAfterCliff.into());
        }

        if schedule.authority() != accounts.authority.key() {
            return Err(ProgramError::IllegalOwner);
        }

        if accounts.mint.key() != schedule.mint() {
            return Err(ProgramError::InvalidAccountData);
        }

        ProgramAccount::verify_seeds(
            &[
                Seed::from(b"participant"),
                Seed::from(accounts.participant_wallet.key()),
                Seed::from(accounts.schedule.key()),
            ],
            accounts.vested_participant,
        )?;

        AssociatedTokenAccount::check(
            accounts.authority_ata,
            accounts.authority,
            accounts.mint,
            accounts.token_program,
        )?;

        let authority_ata = TokenAccount::from_account_info(accounts.authority_ata)?;
        if authority_ata.amount() < instruction_data.token_allocation_amount {
            return Err(ProgramError::InsufficientFunds);
        }

        AssociatedTokenAccount::check(
            accounts.vault,
            accounts.schedule,
            accounts.mint,
            accounts.token_program,
        )?;

        Ok(Self {
            accounts,
            instruction_data,
        })
    }
}
impl<'a> AddParticipant<'a> {
    pub const DISCRIMINATOR: &'a u8 = &1;
    pub fn process(&mut self) -> ProgramResult {
        let bump_binding = [
            // could we get the bump from verify_seeds to avoid wasting CUs on find_program_address again, find_program_address is not very performant so we should avoid calling it multiple times
            ProgramAccount::get_bump(&[
                Seed::from(b"participant"),
                Seed::from(self.accounts.participant_wallet.key()),
                Seed::from(self.accounts.schedule.key()),
            ])?,
        ];
        let seeds = [
            Seed::from(b"participant"),
            Seed::from(self.accounts.participant_wallet.key()),
            Seed::from(self.accounts.schedule.key()),
            Seed::from(&bump_binding),
        ];
        ProgramAccount::init::<VestedParticipant>(
            self.accounts.authority,
            self.accounts.vested_participant,
            &seeds,
            VestedParticipant::LEN,
        )?;

        let mut vested_participant_state =
            VestedParticipant::load_mut(self.accounts.vested_participant)?;
        vested_participant_state.set_inner(
            *self.accounts.schedule.key(),
            *self.accounts.participant_wallet.key(),
            self.instruction_data.token_allocation_amount,
            0,
        )?;

        Transfer {
            from: self.accounts.authority_ata,
            amount: self.instruction_data.token_allocation_amount,
            to: self.accounts.vault,
            authority: self.accounts.authority,
        }
        .invoke()?;

        Ok(())
    }
}
