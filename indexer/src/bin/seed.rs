use std::time::{SystemTime, UNIX_EPOCH};

use solana_client::rpc_client::RpcClient;
use solana_commitment_config::CommitmentConfig;
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::{read_keypair_file, Keypair},
    signer::Signer,
    transaction::Transaction,
};
use solana_system_interface::instruction as system_instruction;

const PROGRAM_ID: Pubkey = solana_sdk::pubkey!("FwnGeaANDtRZHA1xXzjyTjr5mmEZtXBSKuA3umcRPiWG");
const SYSTEM_PROGRAM_ID: Pubkey = solana_sdk::pubkey!("11111111111111111111111111111111");

fn get_schedule_pda(seed: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"schedule", &seed.to_le_bytes()], &PROGRAM_ID)
}

fn get_participant_pda(wallet: &Pubkey, schedule: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"participant", wallet.as_ref(), schedule.as_ref()],
        &PROGRAM_ID,
    )
}

fn get_ata(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(owner, mint)
}

fn build_initialize_ix(
    authority: &Pubkey,
    schedule: &Pubkey,
    mint: &Pubkey,
    vault: &Pubkey,
    start_timestamp: u64,
    cliff_duration: u64,
    step_duration: u64,
    total_duration: u64,
    seed: u64,
    bump: u8,
) -> Instruction {
    let mut data = vec![0u8];
    data.extend_from_slice(&start_timestamp.to_le_bytes());
    data.extend_from_slice(&cliff_duration.to_le_bytes());
    data.extend_from_slice(&step_duration.to_le_bytes());
    data.extend_from_slice(&total_duration.to_le_bytes());
    data.extend_from_slice(&seed.to_le_bytes());
    data.push(bump);

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*schedule, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data,
    }
}

fn build_add_participant_ix(
    authority: &Pubkey,
    authority_ata: &Pubkey,
    vault: &Pubkey,
    participant_wallet: &Pubkey,
    vested_participant_pda: &Pubkey,
    schedule: &Pubkey,
    mint: &Pubkey,
    allocation: u64,
) -> Instruction {
    let mut data = vec![1u8];
    data.extend_from_slice(&allocation.to_le_bytes());

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*authority, true),
            AccountMeta::new(*authority_ata, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(*participant_wallet, false),
            AccountMeta::new(*vested_participant_pda, false),
            AccountMeta::new_readonly(*schedule, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
        ],
        data,
    }
}

fn build_claim_ix(
    participant_wallet: &Pubkey,
    vested_participant: &Pubkey,
    participant_ata: &Pubkey,
    vault: &Pubkey,
    schedule: &Pubkey,
    mint: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new(*participant_wallet, true),
            AccountMeta::new(*vested_participant, false),
            AccountMeta::new(*participant_ata, false),
            AccountMeta::new(*vault, false),
            AccountMeta::new_readonly(*schedule, false),
            AccountMeta::new_readonly(*mint, false),
            AccountMeta::new_readonly(SYSTEM_PROGRAM_ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: vec![2u8],
    }
}

fn send_and_confirm(
    client: &RpcClient,
    ixs: &[Instruction],
    payer: &Keypair,
    signers: &[&Keypair],
) -> solana_sdk::signature::Signature {
    let blockhash = client.get_latest_blockhash().expect("get blockhash");
    let tx = Transaction::new_signed_with_payer(ixs, Some(&payer.pubkey()), signers, blockhash);
    client
        .send_and_confirm_transaction(&tx)
        .expect("send_and_confirm_transaction")
}

fn main() {
    dotenvy::dotenv().ok();
    env_logger::init();

    let rpc_url = std::env::var("RPC_URL").unwrap_or_else(|_| "http://127.0.0.1:8899".into());
    let keypair_path = std::env::var("KEYPAIR_PATH")
        .unwrap_or_else(|_| shellexpand::tilde("~/.config/solana/id.json").into_owned());

    let client = RpcClient::new_with_commitment(&rpc_url, CommitmentConfig::confirmed());
    let payer = read_keypair_file(&keypair_path).expect("read payer keypair");

    println!("RPC:    {rpc_url}");
    println!("Payer:  {}", payer.pubkey());

    let balance = client.get_balance(&payer.pubkey()).expect("get balance");
    println!("Balance: {} SOL", balance as f64 / LAMPORTS_PER_SOL as f64);

    // ── 1. Create mint ──────────────────────────────────────────────────
    let mint = Keypair::new();
    let rent = client
        .get_minimum_balance_for_rent_exemption(spl_token::state::Mint::LEN)
        .expect("get rent");

    let create_mint_ixs = vec![
        system_instruction::create_account(
            &payer.pubkey(),
            &mint.pubkey(),
            rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::ID,
        ),
        spl_token::instruction::initialize_mint2(
            &spl_token::ID,
            &mint.pubkey(),
            &payer.pubkey(),
            None,
            9,
        )
        .expect("init_mint ix"),
    ];

    let sig = send_and_confirm(&client, &create_mint_ixs, &payer, &[&payer, &mint]);
    println!("\nMint created:  {}", mint.pubkey());
    println!("  tx: {sig}");

    // ── 2. Create payer ATA & mint tokens ───────────────────────────────
    let payer_ata = get_ata(&payer.pubkey(), &mint.pubkey());
    let create_ata_ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &payer.pubkey(),
        &mint.pubkey(),
        &spl_token::ID,
    );
    let mint_to_ix = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint.pubkey(),
        &payer_ata,
        &payer.pubkey(),
        &[],
        1_000_000,
    )
    .expect("mint_to ix");

    let sig = send_and_confirm(&client, &[create_ata_ix, mint_to_ix], &payer, &[&payer]);
    println!("Payer ATA:     {payer_ata}");
    println!("  minted 1_000_000 tokens, tx: {sig}");

    // ── 3. Initialize schedule ──────────────────────────────────────────
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let seed: u64 = now; // unique per run
    let start_timestamp = now + 5; // 5s in the future
    let cliff_duration: u64 = 5;
    let step_duration: u64 = 5;
    let total_duration: u64 = 30;

    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_ata(&schedule, &mint.pubkey());

    let init_ix = build_initialize_ix(
        &payer.pubkey(),
        &schedule,
        &mint.pubkey(),
        &vault,
        start_timestamp,
        cliff_duration,
        step_duration,
        total_duration,
        seed,
        bump,
    );

    let init_slot = client.get_slot().unwrap_or(0);
    let sig = send_and_confirm(&client, &[init_ix], &payer, &[&payer]);
    println!("\nSchedule PDA:  {schedule}");
    println!("  seed={seed}, start={start_timestamp}, cliff={cliff_duration}, step={step_duration}, total={total_duration}");
    println!("  tx: {sig}  (slot ~{init_slot})");

    // ── 4. Add participant ──────────────────────────────────────────────
    let participant = Keypair::new();
    let allocation: u64 = 500_000;
    let (vested_participant_pda, _) = get_participant_pda(&participant.pubkey(), &schedule);

    let add_ix = build_add_participant_ix(
        &payer.pubkey(),
        &payer_ata,
        &vault,
        &participant.pubkey(),
        &vested_participant_pda,
        &schedule,
        &mint.pubkey(),
        allocation,
    );

    let sig = send_and_confirm(&client, &[add_ix], &payer, &[&payer]);
    println!("\nParticipant:   {}", participant.pubkey());
    println!("  allocation={allocation}, tx: {sig}");

    // ── 5. Wait for cliff ───────────────────────────────────────────────
    println!("\nWaiting 15s for cliff to pass...");
    std::thread::sleep(std::time::Duration::from_secs(15));

    // ── 6. Fund participant & claim ─────────────────────────────────────
    // Transfer SOL so participant can pay for tx fees
    let fund_ix =
        system_instruction::transfer(&payer.pubkey(), &participant.pubkey(), LAMPORTS_PER_SOL / 10);
    send_and_confirm(&client, &[fund_ix], &payer, &[&payer]);

    let participant_ata = get_ata(&participant.pubkey(), &mint.pubkey());
    let claim_ix = build_claim_ix(
        &participant.pubkey(),
        &vested_participant_pda,
        &participant_ata,
        &vault,
        &schedule,
        &mint.pubkey(),
    );

    let sig = send_and_confirm(&client, &[claim_ix], &participant, &[&participant]);
    println!("Claim tx:      {sig}");

    // ── Summary ─────────────────────────────────────────────────────────
    let start_slot = init_slot.saturating_sub(1);
    println!("\n═══ Seed complete ═══");
    println!("START_SLOT={start_slot}");
    println!("Set this in your .env, then run the indexer:");
    println!("  RUST_LOG=info cargo run -p vesting-indexer");
}
