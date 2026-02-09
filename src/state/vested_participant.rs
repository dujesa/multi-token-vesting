use core::mem::size_of;
use pinocchio::{account_info::{AccountInfo, Ref, RefMut}, program_error::ProgramError, pubkey::Pubkey};
use crate::Discriminator;

#[repr(C, packed)]
pub struct VestedParticipant {
    pub discriminator: u8,       //1
    pub schedule: Pubkey,       //32
    pub participant: Pubkey,    //32
    pub allocated_amount: u64,  //8
    pub claimed_amount: u64,    //8
}

impl Discriminator for VestedParticipant {
    const LEN: usize = size_of::<u8>() + 2 * size_of::<Pubkey>() + 2 * size_of::<u64>();
    const DISCRIMINATOR: u8 = 1;
}

impl VestedParticipant {
    #[inline(always)]
    pub fn load(account_info: &AccountInfo) -> Result<Ref<Self>, ProgramError> {
        if account_info.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData)
        }
        if account_info.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner)
        }
        Ok(Ref::map(account_info.try_borrow_data()?, |bytes| unsafe {
            &*(bytes.as_ptr() as *mut VestedParticipant)
        }))
    }
    #[inline(always)]
    pub fn load_mut(account_info: &AccountInfo) -> Result<RefMut<Self>, ProgramError> {
        if account_info.data_len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData)
        }
        if account_info.owner() != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner)
        }
        Ok(RefMut::map(account_info.try_borrow_mut_data()?, |bytes| unsafe {
            &mut *(bytes.as_ptr() as *mut VestedParticipant)
        }))
    }
    #[inline(always)]
    pub fn schedule(&self) -> &Pubkey { &self.schedule }
    #[inline(always)]
    pub fn participant(&self) -> &Pubkey { &self.participant }
    #[inline(always)]
    pub fn allocated_amount(&self) -> u64 { self.allocated_amount }
    #[inline(always)]
    pub fn claimed_amount(&self) -> u64 { self.claimed_amount }
    #[inline(always)]
    pub fn discriminator(&self) -> u8 { self.discriminator }
    #[inline(always)]
    pub fn is_claim_finalized(&self) -> bool { self.claimed_amount == self.allocated_amount }
    #[inline(always)]
    pub fn set_schedule(&mut self, schedule: Pubkey) {
        self.schedule = schedule;
    }
    #[inline(always)]
    pub fn set_wallet(&mut self, wallet: Pubkey) {
        self.participant = wallet;
    }
    #[inline(always)]
    pub fn set_allocated_amount(&mut self, allocated_amount: u64) {
        self.allocated_amount = allocated_amount;
    }
    #[inline(always)]
    pub fn set_claimed_amount(&mut self, claimed_amount: u64) {
        self.claimed_amount = claimed_amount;
    }
    #[inline(always)]
    pub fn set_disctiminator(&mut self, discriminator: u8) {
        self.discriminator = discriminator;
    }
    #[inline(always)]
    pub fn set_inner(
        &mut self, 
        schedule_mint: Pubkey,
        wallet: Pubkey,
        allocated_amount: u64,
        claimed_amount: u64,

    ) -> Result<(), ProgramError> {
        self.set_schedule(schedule_mint);
        self.set_wallet(wallet);
        self.set_allocated_amount(allocated_amount);
        self.set_claimed_amount(claimed_amount);
        self.set_disctiminator(VestedParticipant::DISCRIMINATOR);

        Ok(())
    }
}