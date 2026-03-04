use crate::Discriminator;
use core::mem::size_of;
use std::ops::{Div, Mul};
use pinocchio::{
    account_info::{AccountInfo, Ref, RefMut},
    program_error::ProgramError,
    pubkey::Pubkey,
    sysvars::{clock::Clock, Sysvar},
};

// it is good practice to save the bump on the account state when using PDAs, this way we can verify the seeds and bump when loading the account in a more performant way
#[repr(C, packed)]
pub struct Schedule {
    // the discriminator is usually stored in the first byte of the account data
    pub discriminator: u8, //1
    pub mint: Pubkey,      //32
    pub authority: Pubkey, //32
    pub seed: [u8; 8],       //8
    pub start: i64,          //8
    pub cliff_duration: i64, //8
    pub step_duration: i64,  //8
    pub total_duration: i64, //8
    pub bump: u8,
}

impl Discriminator for Schedule {
    const DISCRIMINATOR: u8 = 0;
    const LEN: usize = 2 * size_of::<u8>() + 2 * size_of::<Pubkey>() + 5 * size_of::<i64>();
}

impl Schedule {
    #[inline(always)]
    pub fn load(account_info: &AccountInfo) -> Result<Ref<Self>, ProgramError> {
        if account_info.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if account_info.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(Ref::map(account_info.try_borrow_data()?, |bytes| unsafe {
            &*(bytes.as_ptr() as *mut Schedule)
        }))
    }
    #[inline(always)]
    pub fn load_mut(account_info: &AccountInfo) -> Result<RefMut<Self>, ProgramError> {
        if account_info.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if account_info.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }
        Ok(RefMut::map(
            account_info.try_borrow_mut_data()?,
            |bytes| unsafe { &mut *(bytes.as_ptr() as *mut Schedule) },
        ))
    }
    #[inline(always)]
    pub fn discriminator(&self) -> u8 {
        self.discriminator
    }
    #[inline(always)]
    pub fn mint(&self) -> &Pubkey {
        &self.mint
    }
    #[inline(always)]
    pub fn authority(&self) -> &Pubkey {
        &self.authority
    }
    #[inline(always)]
    pub fn seed(&self) -> u64 {
        u64::from_le_bytes(self.seed)
    }
    #[inline(always)]
    pub fn start(&self) -> i64 {
        self.start
    }
    #[inline(always)]
    pub fn cliff_duration(&self) -> i64 {
        self.cliff_duration
    }
    #[inline(always)]
    pub fn step_duration(&self) -> i64 {
        self.step_duration
    }
    #[inline(always)]
    pub fn total_duration(&self) -> i64 {
        self.total_duration
    }
    #[inline(always)]
    pub fn bump(&self) -> u8 {
        self.bump
    }
    #[inline(always)]
    pub fn is_cliff_completed(&self) -> bool {
        Clock::get().unwrap().unix_timestamp > self.cliff_duration + self.start
    }
    #[inline(always)]
    pub fn steps_passed_percentage(&self, bps_denominator: u64) -> i64 {       
        // Never use float in on-chain logic, use BPS with integers instead
        if !self.is_cliff_completed() {
            return 0;
        }
        
        let now = Clock::get().unwrap().unix_timestamp;
        let end = self.start() + self.total_duration();
        if now >= end {
            return 1.mul(bps_denominator) as i64;       
        }
        
        // Cliff = 1 period, remaining vesting periods after cliff
        let vesting_duration = self.total_duration() - self.cliff_duration();
        let steps_after_cliff = vesting_duration / self.step_duration();
        let total_periods = 1 + steps_after_cliff; // cliff + steps
        
        let elapsed_after_cliff = now - self.start() - self.cliff_duration();
        let periods_after_cliff = elapsed_after_cliff / self.step_duration();

        (1 + periods_after_cliff)
            .mul(bps_denominator as i64)
            .div(total_periods) // 1 for cliff + periods passed
    }
    
    #[inline(always)]
    pub fn set_discriminator(&mut self, discriminator: u8) {
        self.discriminator = discriminator;
    }
    #[inline(always)]
    pub fn set_mint(&mut self, mint: Pubkey) {
        self.mint = mint;
    }
    #[inline(always)]
    pub fn set_authority(&mut self, authority: Pubkey) {
        self.authority = authority;
    }
    #[inline(always)]
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_le_bytes();
    }
    #[inline(always)]
    pub fn set_start(&mut self, start: i64) {
        self.start = start;
    }
    #[inline(always)]
    pub fn set_cliff_duration(&mut self, cliff_duration: i64) {
        self.cliff_duration = cliff_duration;
    }
    #[inline(always)]
    pub fn set_step_duration(&mut self, step_duration: i64) {
        self.step_duration = step_duration;
    }
    #[inline(always)]
    pub fn set_total_duration(&mut self, total_duration: i64) {
        self.total_duration = total_duration;
    }
    #[inline(always)]
    pub fn set_bump(&mut self, bump: u8) {
        self.bump = bump;
    }
    #[inline(always)]
    pub fn set_inner(
        &mut self,
        mint: Pubkey,
        authority: Pubkey,
        seed: u64,
        start: i64,
        cliff_duration: i64,
        step_duration: i64,
        total_duration: i64,
        bump: u8,
    ) -> Result<(), ProgramError> {
        self.set_discriminator(Schedule::DISCRIMINATOR);
        self.set_mint(mint);
        self.set_authority(authority);
        self.set_seed(seed);
        self.set_start(start);
        self.set_cliff_duration(cliff_duration);
        self.set_step_duration(step_duration);
        self.set_total_duration(total_duration);
        self.set_bump(bump);

        Ok(())
    }
}
