pub mod accounts;
pub mod instructions;

use carbon_core::instruction::{DecodedInstruction, InstructionDecoder};
use instructions::{AddParticipantData, InitializeData, VestingInstruction};
use solana_pubkey::Pubkey;

pub const PROGRAM_ID: Pubkey =
    solana_pubkey::pubkey!("FwnGeaANDtRZHA1xXzjyTjr5mmEZtXBSKuA3umcRPiWG");

pub struct VestingDecoder;

impl InstructionDecoder<'_> for VestingDecoder {
    type InstructionType = VestingInstruction;

    fn decode_instruction(
        &self,
        instruction: &solana_instruction::Instruction,
    ) -> Option<DecodedInstruction<Self::InstructionType>> {
        if instruction.program_id != PROGRAM_ID {
            return None;
        }

        let data = instruction.data.as_slice();
        if data.is_empty() {
            return None;
        }

        let discriminator = data[0];
        let body = &data[1..];

        let decoded = match discriminator {
            // Initialize: 41 bytes — i64, i64, i64, i64, u64, u8
            0 => {
                if body.len() < 41 {
                    return None;
                }
                VestingInstruction::Initialize(InitializeData {
                    start_timestamp: i64::from_le_bytes(body[0..8].try_into().ok()?),
                    cliff_duration: i64::from_le_bytes(body[8..16].try_into().ok()?),
                    step_duration: i64::from_le_bytes(body[16..24].try_into().ok()?),
                    total_duration: i64::from_le_bytes(body[24..32].try_into().ok()?),
                    seed: u64::from_le_bytes(body[32..40].try_into().ok()?),
                    bump: body[40],
                })
            }
            // AddParticipant: 8 bytes — u64
            1 => {
                if body.len() < 8 {
                    return None;
                }
                VestingInstruction::AddParticipant(AddParticipantData {
                    token_allocation_amount: u64::from_le_bytes(body[0..8].try_into().ok()?),
                })
            }
            // Claim: no data
            2 => VestingInstruction::Claim,
            _ => return None,
        };

        Some(DecodedInstruction {
            program_id: instruction.program_id,
            data: decoded,
            accounts: instruction.accounts.clone(),
        })
    }
}
