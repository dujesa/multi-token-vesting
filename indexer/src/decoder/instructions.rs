use solana_pubkey::Pubkey;

/// All instructions the vesting program can process.
#[derive(Debug, Clone, PartialEq)]
pub enum VestingInstruction {
    Initialize(InitializeData),
    AddParticipant(AddParticipantData),
    Claim,
}

/// 41 bytes after discriminator.
#[derive(Debug, Clone, PartialEq)]
pub struct InitializeData {
    pub start_timestamp: i64,
    pub cliff_duration: i64,
    pub step_duration: i64,
    pub total_duration: i64,
    pub seed: u64,
    pub bump: u8,
}

/// 8 bytes after discriminator.
#[derive(Debug, Clone, PartialEq)]
pub struct AddParticipantData {
    pub token_allocation_amount: u64,
}

// ---------- Account arrangement structs ----------
#[allow(dead_code)]

pub struct InitializeAccounts {
    pub authority: Pubkey,
    pub schedule: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub system_program: Pubkey,
    pub token_program: Pubkey,
    pub ata_program: Pubkey,
}

#[allow(dead_code)]
pub struct AddParticipantAccounts {
    pub authority: Pubkey,
    pub authority_ata: Pubkey,
    pub vault: Pubkey,
    pub participant_wallet: Pubkey,
    pub vested_participant: Pubkey,
    pub schedule: Pubkey,
    pub mint: Pubkey,
    pub system_program: Pubkey,
    pub token_program: Pubkey,
}

#[allow(dead_code)]
pub struct ClaimAccounts {
    pub participant_wallet: Pubkey,
    pub vested_participant: Pubkey,
    pub participant_ata: Pubkey,
    pub vault: Pubkey,
    pub schedule: Pubkey,
    pub mint: Pubkey,
    pub system_program: Pubkey,
    pub token_program: Pubkey,
    pub ata_program: Pubkey,
}
