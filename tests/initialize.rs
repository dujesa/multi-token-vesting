use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, spl_token};
use multi_token_vesting::{Discriminator, Schedule};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    sysvar::clock::Clock,
    transaction::Transaction,
};

// System program ID: 11111111111111111111111111111111
const SYSTEM_PROGRAM_ID: Pubkey = Pubkey::new_from_array([0u8; 32]);

const PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    0xde, 0x0c, 0x2a, 0xd8, 0xf6, 0xeb, 0x0d, 0x5a, 0x94, 0x92, 0x02, 0x79, 0x06, 0xfa, 0xcc, 0x62,
    0x60, 0xbb, 0x41, 0xca, 0xcd, 0xdd, 0x62, 0x68, 0x67, 0xb5, 0xe6, 0x8a, 0xfc, 0x26, 0xe0, 0x35,
]);

fn setup_svm() -> LiteSVM {
    let mut svm = LiteSVM::new()
        .with_sigverify(false)
        .with_builtins();
    svm.add_program_from_file(
        PROGRAM_ID,
        "target/deploy/multi_token_vesting.so",
    ).expect("Failed to load program");
    svm
}

fn get_schedule_pda(seed: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[b"schedule", &seed.to_le_bytes()], &PROGRAM_ID)
}

fn get_vault_ata(schedule: &Pubkey, mint: &Pubkey) -> Pubkey {
    spl_associated_token_account::get_associated_token_address(schedule, mint)
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
    let mut data = vec![0u8]; // discriminator for Initialize
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

#[test]
fn test_initialize_success() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // Create mint
    let mint = CreateMint::new(&mut svm, &authority)
        .decimals(9)
        .send()
        .unwrap();

    // Derive PDAs
    let seed: u64 = 1;
    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_vault_ata(&schedule, &mint);

    // Set clock to known time
    let current_time: i64 = 1000;
    svm.set_sysvar(&Clock {
        unix_timestamp: current_time,
        ..Default::default()
    });

    // Build and send transaction
    let start_timestamp: u64 = 2000; // future
    let cliff_duration: u64 = 100;
    let step_duration: u64 = 50;
    let total_duration: u64 = 300; // (300-100) % 50 == 0

    let ix = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        start_timestamp,
        cliff_duration,
        step_duration,
        total_duration,
        seed,
        bump,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Initialize should succeed: {:?}", result.err());

    // Verify schedule account exists and has correct size
    let schedule_account = svm.get_account(&schedule).unwrap();
    assert_eq!(schedule_account.owner, PROGRAM_ID);
    assert_eq!(schedule_account.data.len(), 137); // Schedule::LEN
}

#[test]
fn test_initialize_seed_zero_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &authority)
        .decimals(9)
        .send()
        .unwrap();

    let seed: u64 = 0; // Invalid seed
    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_vault_ata(&schedule, &mint);

    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    let ix = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        2000, // start
        100,  // cliff
        50,   // step
        300,  // total
        seed,
        bump,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "seed=0 should fail");
}

#[test]
fn test_initialize_start_in_past_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &authority)
        .decimals(9)
        .send()
        .unwrap();

    let seed: u64 = 2;
    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_vault_ata(&schedule, &mint);

    // Current time = 2000
    svm.set_sysvar(&Clock {
        unix_timestamp: 2000,
        ..Default::default()
    });

    let ix = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        1000, // start in PAST (< 2000)
        100,
        50,
        300,
        seed,
        bump,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "start_timestamp in past should fail");
}

#[test]
fn test_initialize_uneven_steps_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &authority)
        .decimals(9)
        .send()
        .unwrap();

    let seed: u64 = 3;
    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_vault_ata(&schedule, &mint);

    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    // (total - cliff) % step != 0
    // (300 - 100) % 70 = 200 % 70 = 60 != 0
    let ix = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        2000,
        100,  // cliff
        70,   // step (doesn't divide evenly)
        300,  // total
        seed,
        bump,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "uneven step division should fail");
}

#[test]
fn test_initialize_duplicate_seed_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    let mint = CreateMint::new(&mut svm, &authority)
        .decimals(9)
        .send()
        .unwrap();

    let seed: u64 = 4;
    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_vault_ata(&schedule, &mint);

    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    // First initialize - should succeed
    let ix1 = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        2000, 100, 50, 300, seed, bump,
    );

    let tx1 = Transaction::new_signed_with_payer(
        &[ix1],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result1 = svm.send_transaction(tx1);
    assert!(result1.is_ok(), "First init should succeed");

    // Second initialize with same seed - should fail
    let ix2 = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        2000, 100, 50, 300, seed, bump,
    );

    let tx2 = Transaction::new_signed_with_payer(
        &[ix2],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result2 = svm.send_transaction(tx2);
    assert!(result2.is_err(), "Duplicate seed should fail");
}
