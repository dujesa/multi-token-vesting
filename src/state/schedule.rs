use core::mem::size_of;
use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey, sysvars::{Sysvar, clock::Clock}};
use crate::Discriminator;

#[repr(C)]
pub struct Schedule {
    pub mint: Pubkey,           //32
    pub authority: Pubkey,      //32
    pub vault: Pubkey,          //32
    pub seed: [u8; 8],          //8
    pub start: u64,             //8
    pub cliff_duration: u64,    //8
    pub step_duration: u64,     //8
    pub total_duration: u64,    //8
    pub discriminator: u8,      //1
}

impl Discriminator for Schedule {
    const DISCRIMINATOR: u8 = 0;
    const LEN: usize = 3 * size_of::<Pubkey>() + 5 * size_of::<u64>() + size_of::<u8>();
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
        Ok(RefMut::map(account_info.try_borrow_mut_data()?, |bytes| unsafe {
            &mut *(bytes.as_ptr() as *mut Schedule)
        }))
    }
    #[inline(always)]
    pub fn mint(&self) -> &Pubkey { &self.mint }
    #[inline(always)]
    pub fn authority(&self) -> &Pubkey { &self.authority }
    #[inline(always)]
    pub fn vault(&self) -> &Pubkey { &self.vault }
    #[inline(always)]
    pub fn seed(&self) -> u64 { u64::from_le_bytes(self.seed) }
    #[inline(always)]
    pub fn start(&self) -> u64 { self.start }
    #[inline(always)]
    pub fn cliff_duration(&self) -> u64 { self.cliff_duration }
    #[inline(always)]
    pub fn step_duration(&self) -> u64 { self.step_duration }
    #[inline(always)]
    pub fn total_duration(&self) -> u64 { self.total_duration }
    #[inline(always)]
    pub fn discriminator(&self) -> u8 { self.discriminator }
    #[inline(always)]
    pub fn is_cliff_completed(&self) -> bool {
        Clock::get().unwrap().unix_timestamp as u64 > self.cliff_duration + self.start
    }
    #[inline(always)]
    pub fn steps_passed_percentage(&self) -> f32 {
        if !self.is_cliff_completed() {
            return 0.0; 
        }

        let now = Clock::get().unwrap().unix_timestamp as u64;
        let end = self.start() + self.total_duration();
        if now >= end {
            return 1.0;
        }

        let elapsed = (now - self.start()) as f32;
        let periods_passed = elapsed / self.step_duration() as f32;
        let total_periods = self.total_duration() as f32 / self.step_duration() as f32;

        return periods_passed / total_periods;
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
    pub fn set_vault(&mut self, vault: Pubkey) {
        self.vault = vault;
    }
    #[inline(always)]
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = seed.to_le_bytes();
    }
    #[inline(always)]
    pub fn set_start(&mut self, start: u64) {
        self.start = start;
    }
    #[inline(always)]
    pub fn set_cliff_duration(&mut self, cliff_duration: u64) {
        self.cliff_duration = cliff_duration;
    }
    #[inline(always)]
    pub fn set_step_duration(&mut self, step_duration: u64) {
        self.step_duration = step_duration;
    }
    #[inline(always)]
    pub fn set_total_duration(&mut self, total_duration: u64) {
        self.total_duration = total_duration;
    }
    #[inline(always)]
    pub fn set_discriminator(&mut self, discriminator: u8) {
        self.discriminator = discriminator;
    }
    #[inline(always)]
    pub fn set_inner(
        &mut self,
        mint: Pubkey,       
        authority: Pubkey,  
        vault: Pubkey,      
        seed: u64,      
        start: u64,         
        cliff_duration: u64,
        step_duration: u64, 
        total_duration: u64,
    ) -> Result<(), ProgramError> {
        self.set_mint(mint);
        self.set_authority(authority);
        self.set_vault(vault);
        self.set_seed(seed);
        self.set_start(start);
        self.set_cliff_duration(cliff_duration);
        self.set_step_duration(step_duration);
        self.set_total_duration(total_duration);
        self.set_discriminator(Schedule::DISCRIMINATOR);

        Ok(())
    }
}