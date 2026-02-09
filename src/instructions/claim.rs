use std::ops::{Div, Mul};

use pinocchio::{ProgramResult, account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError};
use pinocchio_token::{instructions::Transfer, state::TokenAccount};

use crate::{AssociatedTokenAccount, MintAccount, PinocchioError, ProgramAccount, Schedule, SignerAccount, VestedParticipant};

pub struct ClaimAccounts<'a> {
    pub participant_wallet: &'a AccountInfo, //signer 
    pub vested_participant: &'a AccountInfo, //state acc
    pub participant_ata: &'a AccountInfo, //claimers ata
    pub vault: &'a AccountInfo, //vault for sending from
    pub schedule: &'a AccountInfo,  
    pub mint: &'a AccountInfo,
    pub system_program: &'a AccountInfo,
    pub token_program: &'a AccountInfo,
    pub associated_token_account_program: &'a AccountInfo,
}
impl<'a> TryFrom<&'a [AccountInfo]> for ClaimAccounts<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let [
            participant_wallet,
            vested_participant,
            participant_ata,
            vault,
            schedule,
            mint,
            system_program,
            token_program,
            associated_token_account_program
        ] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys)
        };

        SignerAccount::check(participant_wallet)?;
        ProgramAccount::check::<VestedParticipant>(vested_participant)?;
        ProgramAccount::check::<Schedule>(schedule)?;
        MintAccount::check(mint)?;

        Ok(Self { participant_wallet, vested_participant, participant_ata, vault, schedule, mint, system_program, token_program, associated_token_account_program })
    }
}
pub struct Claim<'a> {
    pub accounts: ClaimAccounts<'a>,
}
impl<'a> TryFrom<&'a [AccountInfo]> for Claim<'a> {
    type Error = ProgramError;
    fn try_from(accounts: &'a [AccountInfo]) -> Result<Self, Self::Error> {
        let accounts = ClaimAccounts::try_from(accounts)?;

        {
            let schedule = Schedule::load(accounts.schedule)?;
            if !schedule.is_cliff_completed() {
                return Err(PinocchioError::CannotClaimBeforeCliff.into());
            }

            let vested_participant = VestedParticipant::load(accounts.vested_participant)?;
            if accounts.mint.key() != schedule.mint() || accounts.schedule.key() != vested_participant.schedule() {
                return Err(ProgramError::InvalidAccountData);
            }
            
            if vested_participant.is_claim_finalized() {
                return Err(PinocchioError::CannotDoubleClaim.into());
            }
            if *vested_participant.participant() != *accounts.participant_wallet.key() {
                return Err(PinocchioError::InvalidSigner.into());
            }
            if *vested_participant.schedule() != *accounts.schedule.key() {
                return Err(PinocchioError::InvalidSigner.into());
            }
        }

        AssociatedTokenAccount::check(
            accounts.vault, 
            accounts.schedule, 
            accounts.mint, 
            accounts.token_program
        )?;

        AssociatedTokenAccount::init_if_needed(
            accounts.participant_ata,
            accounts.mint,
            accounts.participant_wallet,
            accounts.participant_wallet,
            accounts.system_program,
            accounts.token_program,
        )?;

        ProgramAccount::verify_seeds(
            &[
                Seed::from(b"participant"), 
                Seed::from(accounts.participant_wallet.key()),
                Seed::from(accounts.schedule.key()),
            ], 
            accounts.vested_participant, 
        )?;

        Ok(Self { accounts })
    }
}
impl<'a> Claim<'a> {
    pub const DISCRIMINATOR: &'a u8 = &2;
    pub fn process(&mut self) -> ProgramResult {
        let (claim_amount, seed) = {
            const BPS_DENOMINATOR: u64 = 10_000;     

            let schedule = Schedule::load(self.accounts.schedule)?;
            let vested_participant = VestedParticipant::load(self.accounts.vested_participant)?;
            
            let possible_claim_amount = vested_participant.allocated_amount()
                    .mul(schedule.steps_passed_percentage(BPS_DENOMINATOR) as u64)
                    .div(BPS_DENOMINATOR);
            
            let claim_amount = possible_claim_amount - vested_participant.claimed_amount();
            if claim_amount == 0 {
                return Err(PinocchioError::ClaimAmountInvalid.into());
            }

            (claim_amount, schedule.seed())
        };

        {
            let vault = TokenAccount::from_account_info(self.accounts.vault)?;
            if vault.amount() < claim_amount {
                return Err(ProgramError::InsufficientFunds);
            }
        }

        let seed_binding = seed.to_le_bytes();
        let bump = ProgramAccount::get_bump(&[
            Seed::from(b"schedule"),
            Seed::from(&seed_binding),
            ])?;
        let bump_binding = [bump];
        let seeds = [
            Seed::from(b"schedule"),
            Seed::from(&seed_binding),
            Seed::from(&bump_binding)
        ];
        let signer = [Signer::from(&seeds)];

        Transfer {
            from: self.accounts.vault,
            amount: claim_amount,
            to: self.accounts.participant_ata,
            authority: self.accounts.schedule,
        }.invoke_signed(&signer)?;

        let mut vested_participant = VestedParticipant::load_mut(self.accounts.vested_participant)?;
        
        let total_claimed_amount = vested_participant.claimed_amount() + claim_amount;
        if total_claimed_amount > vested_participant.allocated_amount() {
            return Err(PinocchioError::ClaimAmountOverflow.into());
        }

        vested_participant.set_claimed_amount(total_claimed_amount);

        Ok(())
    }
}