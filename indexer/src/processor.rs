use std::sync::Arc;

use async_trait::async_trait;
use carbon_core::{
    deserialize::ArrangeAccounts,
    error::CarbonResult,
    instruction::{DecodedInstruction, InstructionMetadata, NestedInstructions},
    metrics::MetricsCollection,
    processor::Processor,
};
use sqlx::PgPool;

use crate::decoder::{
    accounts::ClaimArrange,
    instructions::{AddParticipantData, InitializeData, VestingInstruction},
};

pub struct VestingProcessor {
    pub pool: PgPool,
}

#[async_trait]
impl Processor for VestingProcessor {
    type InputType = (
        InstructionMetadata,
        DecodedInstruction<VestingInstruction>,
        NestedInstructions,
        solana_instruction::Instruction,
    );

    async fn process(
        &mut self,
        (metadata, instruction, nested, _raw_ix): Self::InputType,
        _metrics: Arc<MetricsCollection>,
    ) -> CarbonResult<()> {
        let sig = metadata.transaction_metadata.signature.to_string();
        let slot = metadata.transaction_metadata.slot as i64;

        match &instruction.data {
            VestingInstruction::Initialize(data) => {
                self.handle_initialize(data, &instruction.accounts, &sig, slot)
                    .await
            }
            VestingInstruction::AddParticipant(data) => {
                self.handle_add_participant(data, &instruction.accounts, &sig, slot)
                    .await
            }
            VestingInstruction::Claim => {
                self.handle_claim(&instruction.accounts, &nested, &sig, slot)
                    .await
            }
        }

        Ok(())
    }
}

impl VestingProcessor {
    async fn handle_initialize(
        &self,
        data: &InitializeData,
        accounts: &[solana_instruction::AccountMeta],
        sig: &str,
        slot: i64,
    ) {
        let Some(accs) = InitializeData::arrange_accounts(accounts) else {
            log::warn!("Initialize: failed to arrange accounts, tx={sig}");
            return;
        };

        let result = sqlx::query(
            "INSERT INTO schedules (
                schedule_address, mint, authority, seed,
                start_timestamp, cliff_duration, step_duration, total_duration,
                bump, vault, tx_signature, slot
            ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT DO NOTHING",
        )
        .bind(accs.schedule.to_string())
        .bind(accs.mint.to_string())
        .bind(accs.authority.to_string())
        .bind(data.seed as i64)
        .bind(data.start_timestamp)
        .bind(data.cliff_duration)
        .bind(data.step_duration)
        .bind(data.total_duration)
        .bind(data.bump as i16)
        .bind(accs.vault.to_string())
        .bind(sig)
        .bind(slot)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => log::info!("Initialize: schedule={}, tx={sig}", accs.schedule),
            Err(e) => log::error!("Initialize insert failed: {e}, tx={sig}"),
        }
    }

    async fn handle_add_participant(
        &self,
        data: &AddParticipantData,
        accounts: &[solana_instruction::AccountMeta],
        sig: &str,
        slot: i64,
    ) {
        let Some(accs) = AddParticipantData::arrange_accounts(accounts) else {
            log::warn!("AddParticipant: failed to arrange accounts, tx={sig}");
            return;
        };

        let result = sqlx::query(
            "INSERT INTO participants (
                participant_pda, schedule_address, participant_wallet,
                allocated_amount, tx_signature, slot
            ) VALUES ($1,$2,$3,$4,$5,$6)
            ON CONFLICT DO NOTHING",
        )
        .bind(accs.vested_participant.to_string())
        .bind(accs.schedule.to_string())
        .bind(accs.participant_wallet.to_string())
        .bind(data.token_allocation_amount as i64)
        .bind(sig)
        .bind(slot)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => log::info!(
                "AddParticipant: pda={}, wallet={}, tx={sig}",
                accs.vested_participant,
                accs.participant_wallet
            ),
            Err(e) => log::error!("AddParticipant insert failed: {e}, tx={sig}"),
        }
    }

    async fn handle_claim(
        &self,
        accounts: &[solana_instruction::AccountMeta],
        nested: &NestedInstructions,
        sig: &str,
        slot: i64,
    ) {
        let Some(accs) = ClaimArrange::arrange_accounts(accounts) else {
            log::warn!("Claim: failed to arrange accounts, tx={sig}");
            return;
        };

        // Extract the claimed amount from the inner SPL Token Transfer CPI.
        // The transfer instruction data is: 1-byte discriminator (3 = Transfer) + 8-byte u64 amount.
        let claimed_amount = nested
            .iter()
            .find_map(|inner| {
                let ix = &inner.instruction;
                if ix.data.len() == 9 && ix.data[0] == 3 {
                    Some(u64::from_le_bytes(
                        ix.data[1..9].try_into().unwrap_or_default(),
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(0);

        let result = sqlx::query(
            "INSERT INTO claims (
                participant_pda, schedule_address, participant_wallet,
                claimed_amount, tx_signature, slot
            ) VALUES ($1,$2,$3,$4,$5,$6)
            ON CONFLICT (tx_signature) DO NOTHING",
        )
        .bind(accs.vested_participant.to_string())
        .bind(accs.schedule.to_string())
        .bind(accs.participant_wallet.to_string())
        .bind(claimed_amount as i64)
        .bind(sig)
        .bind(slot)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => log::info!(
                "Claim: pda={}, amount={claimed_amount}, tx={sig}",
                accs.vested_participant
            ),
            Err(e) => log::error!("Claim insert failed: {e}, tx={sig}"),
        }
    }
}
