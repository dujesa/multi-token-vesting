use carbon_core::{account_utils::next_account, deserialize::ArrangeAccounts};
use solana_instruction::AccountMeta;

use super::instructions::{
    AddParticipantAccounts, AddParticipantData, ClaimAccounts, InitializeAccounts, InitializeData,
};

impl ArrangeAccounts for InitializeData {
    type ArrangedAccounts = InitializeAccounts;

    fn arrange_accounts(accounts: &[AccountMeta]) -> Option<Self::ArrangedAccounts> {
        let mut iter = accounts.iter();
        Some(InitializeAccounts {
            authority: next_account(&mut iter)?,
            schedule: next_account(&mut iter)?,
            mint: next_account(&mut iter)?,
            vault: next_account(&mut iter)?,
            system_program: next_account(&mut iter)?,
            token_program: next_account(&mut iter)?,
            ata_program: next_account(&mut iter)?,
        })
    }
}

impl ArrangeAccounts for AddParticipantData {
    type ArrangedAccounts = AddParticipantAccounts;

    fn arrange_accounts(accounts: &[AccountMeta]) -> Option<Self::ArrangedAccounts> {
        let mut iter = accounts.iter();
        Some(AddParticipantAccounts {
            authority: next_account(&mut iter)?,
            authority_ata: next_account(&mut iter)?,
            vault: next_account(&mut iter)?,
            participant_wallet: next_account(&mut iter)?,
            vested_participant: next_account(&mut iter)?,
            schedule: next_account(&mut iter)?,
            mint: next_account(&mut iter)?,
            system_program: next_account(&mut iter)?,
            token_program: next_account(&mut iter)?,
        })
    }
}

/// Claim has no instruction data struct to impl on, so we use a unit struct.
pub struct ClaimArrange;

impl ArrangeAccounts for ClaimArrange {
    type ArrangedAccounts = ClaimAccounts;

    fn arrange_accounts(accounts: &[AccountMeta]) -> Option<Self::ArrangedAccounts> {
        let mut iter = accounts.iter();
        Some(ClaimAccounts {
            participant_wallet: next_account(&mut iter)?,
            vested_participant: next_account(&mut iter)?,
            participant_ata: next_account(&mut iter)?,
            vault: next_account(&mut iter)?,
            schedule: next_account(&mut iter)?,
            mint: next_account(&mut iter)?,
            system_program: next_account(&mut iter)?,
            token_program: next_account(&mut iter)?,
            ata_program: next_account(&mut iter)?,
        })
    }
}
