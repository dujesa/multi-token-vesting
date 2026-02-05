use core::mem::size_of;
use pinocchio::{ProgramResult, account_info::AccountInfo, instruction::{Seed, Signer}, program_error::ProgramError, pubkey::find_program_address, sysvars::{Sysvar, rent::Rent}};
use pinocchio_associated_token_account::instructions::Create;
use pinocchio_system::instructions::CreateAccount;

use crate::{Discriminator, PinocchioError};

pub struct ProgramAccount;
impl ProgramAccount {
    pub fn check<T: Discriminator>(account: &AccountInfo) -> Result<(), ProgramError> 
    where 
        T: 'static
    {
        if !account.is_owned_by(&crate::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        // we can check the discriminator byte to make sure the account is of the expected type instead of checking the length
        if account.data_len().ne(&T::LEN) {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }

    pub fn init<'a, T: Sized>(
        payer: &AccountInfo,
        account: &AccountInfo,
        seeds: &[Seed<'a>],
        space: usize
    ) -> ProgramResult {
        let lamports = Rent::get()?.minimum_balance(space);
        let signer = [Signer::from(seeds)];
        CreateAccount {
            from: payer,
            to: account,
            lamports,
            space: space as u64,
            owner: &crate::ID,
        }
        .invoke_signed(&signer)?;
        Ok(())
    }

    pub fn verify_seeds(
        seeds: &[Seed],
        account: &AccountInfo,
    ) -> Result<(), ProgramError> {
        let seed_bytes: Vec<&[u8]> = seeds.iter().map(|s| s.as_ref()).collect();
        let (expected_public_key, _) = find_program_address(&seed_bytes, &crate::ID);

        if account.key().ne(&expected_public_key) {
            return Err(ProgramError::InvalidAccountData)
        }

        Ok(())
    }

    pub fn get_bump(
        seeds: &[Seed],
    ) -> Result<u8, ProgramError> {
        let seed_bytes: Vec<&[u8]> = seeds.iter().map(|s| s.as_ref()).collect();
        let (_, bump) = find_program_address(&seed_bytes, &crate::ID);

        Ok(bump)
    }

    pub fn close(
        account: &AccountInfo,
        destination: &AccountInfo
    ) -> ProgramResult {
        {
            let mut data = account.try_borrow_mut_data()?;
            data[0] = 0xff;
        }
        *destination.try_borrow_mut_lamports()? += *account.try_borrow_mut_lamports()?;
        account.resize(1)?;
        account.close()
    }
}

pub struct SignerAccount;
impl SignerAccount {
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_signer() {
            return Err(PinocchioError::InvalidSigner.into());
        }

        Ok(())
    }
}

pub struct MintAccount;
impl MintAccount {
    pub fn check(account: &AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }

        if account.data_len().ne(&pinocchio_token::state::Mint::LEN) {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}

pub struct TokenAccount;
impl TokenAccount {
    pub fn check(account: AccountInfo) -> Result<(), ProgramError> {
        if !account.is_owned_by(&pinocchio_token::ID) {
            return Err(ProgramError::InvalidAccountOwner);
        }
        if account.data_len().ne(&pinocchio_token::state::TokenAccount::LEN) {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

pub struct AssociatedTokenAccount;
impl AssociatedTokenAccount {
    pub fn check(
        account: &AccountInfo,
        authority: &AccountInfo,
        mint: &AccountInfo,
        token_program: &AccountInfo,
    ) -> Result<(), ProgramError> {
        TokenAccount::check(*account)?;
        if find_program_address(
            &[authority.key(), token_program.key(), mint.key()],
            &pinocchio_associated_token_account::ID
        ).0.ne(account.key()) {
            return Err(PinocchioError::InvalidAddress.into())
        }

        Ok(())
    }
    pub fn init(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        Create {
            funding_account: payer,
            account,
            wallet: owner,
            mint,
            system_program,
            token_program
        }
        .invoke()
        .map_err(|_| ProgramError::InvalidAccountData)
    }
    pub fn init_if_needed(
        account: &AccountInfo,
        mint: &AccountInfo,
        payer: &AccountInfo,
        owner: &AccountInfo,
        system_program: &AccountInfo,
        token_program: &AccountInfo,
    ) -> ProgramResult {
        match Self::check(account, payer, mint, token_program) {
            Ok(_) => Ok(()),
            Err(_) => Self::init(account, mint, payer, owner, system_program, token_program)
        }
    }
}