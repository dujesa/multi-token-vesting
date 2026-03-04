use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo, spl_token};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
    sysvar::clock::Clock,
    transaction::Transaction,
};

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

fn get_participant_pda(participant: &Pubkey, schedule: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[b"participant", participant.as_ref(), schedule.as_ref()],
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
    let mut data = vec![0u8]; // Initialize discriminator
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
    let mut data = vec![1u8]; // AddParticipant discriminator
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

/// Helper to initialize a schedule and return (schedule_pda, vault_ata, mint)
fn setup_schedule(svm: &mut LiteSVM, authority: &Keypair, seed: u64) -> (Pubkey, Pubkey, Pubkey) {
    let mint = CreateMint::new(svm, authority)
        .decimals(9)
        .send()
        .unwrap();

    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_ata(&schedule, &mint);

    let ix = build_initialize_ix(
        &authority.pubkey(),
        &schedule,
        &mint,
        &vault,
        2000,  // start
        100,   // cliff
        50,    // step
        300,   // total
        seed,
        bump,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[authority],
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).expect("Initialize should succeed");

    (schedule, vault, mint)
}

#[test]
fn test_add_participant_success() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // Set clock before cliff
    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    // Setup schedule
    let seed: u64 = 100;
    let (schedule, vault, mint) = setup_schedule(&mut svm, &authority, seed);

    // Create authority's ATA
    let authority_ata = get_ata(&authority.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint)
        .owner(&authority.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &authority, &mint, &authority_ata, 1_000_000_000)
        .send()
        .unwrap();

    // Derive participant PDA from wallet + schedule
    let participant_wallet = Keypair::new();
    let (vested_participant_pda, _) = get_participant_pda(&participant_wallet.pubkey(), &schedule);

    // Add participant
    let allocation: u64 = 500_000_000;
    let ix = build_add_participant_ix(
        &authority.pubkey(),
        &authority_ata,
        &vault,
        &participant_wallet.pubkey(),
        &vested_participant_pda,
        &schedule,
        &mint,
        allocation,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "AddParticipant should succeed: {:?}", result.err());

    // Verify vested participant PDA exists
    let vested_participant_account = svm.get_account(&vested_participant_pda).unwrap();
    assert_eq!(vested_participant_account.owner, PROGRAM_ID);
}

#[test]
fn test_add_wrong_authority_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let wrong_authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&wrong_authority.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    // Setup schedule with `authority`
    let seed: u64 = 200;
    let (schedule, vault, mint) = setup_schedule(&mut svm, &authority, seed);

    // Create wrong_authority's ATA and fund it
    let wrong_authority_ata = get_ata(&wrong_authority.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &wrong_authority, &mint)
        .owner(&wrong_authority.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &authority, &mint, &wrong_authority_ata, 1_000_000_000)
        .send()
        .unwrap();

    // Derive participant PDA
    let participant_wallet = Keypair::new();
    let (vested_participant_pda, _) = get_participant_pda(&participant_wallet.pubkey(), &schedule);

    // Try to add participant with wrong authority
    let ix = build_add_participant_ix(
        &wrong_authority.pubkey(),  // WRONG - not schedule authority
        &wrong_authority_ata,
        &vault,
        &participant_wallet.pubkey(),
        &vested_participant_pda,
        &schedule,
        &mint,
        500_000_000,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&wrong_authority.pubkey()),
        &[&wrong_authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Wrong authority should fail");
}

#[test]
fn test_add_after_cliff_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    // Set time BEFORE cliff for schedule creation
    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    let seed: u64 = 300;
    let (schedule, vault, mint) = setup_schedule(&mut svm, &authority, seed);

    // Create authority ATA and fund it
    let authority_ata = get_ata(&authority.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint)
        .owner(&authority.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &authority, &mint, &authority_ata, 1_000_000_000)
        .send()
        .unwrap();

    // Now set time AFTER cliff (schedule starts at 2000, cliff is 100, so cliff ends at 2100)
    svm.set_sysvar(&Clock {
        unix_timestamp: 2200,  // After cliff completed
        ..Default::default()
    });

    let participant_wallet = Keypair::new();
    let (vested_participant_pda, _) = get_participant_pda(&participant_wallet.pubkey(), &schedule);

    let ix = build_add_participant_ix(
        &authority.pubkey(),
        &authority_ata,
        &vault,
        &participant_wallet.pubkey(),
        &vested_participant_pda,
        &schedule,
        &mint,
        500_000_000,
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Adding participant after cliff should fail");
}

#[test]
fn test_add_zero_allocation_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock {
        unix_timestamp: 1000,
        ..Default::default()
    });

    let seed: u64 = 400;
    let (schedule, vault, mint) = setup_schedule(&mut svm, &authority, seed);

    let authority_ata = get_ata(&authority.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &authority, &mint)
        .owner(&authority.pubkey())
        .send()
        .unwrap();

    MintTo::new(&mut svm, &authority, &mint, &authority_ata, 1_000_000_000)
        .send()
        .unwrap();

    let participant_wallet = Keypair::new();
    let (vested_participant_pda, _) = get_participant_pda(&participant_wallet.pubkey(), &schedule);

    // Try with zero allocation
    let ix = build_add_participant_ix(
        &authority.pubkey(),
        &authority_ata,
        &vault,
        &participant_wallet.pubkey(),
        &vested_participant_pda,
        &schedule,
        &mint,
        0,  // ZERO allocation
    );

    let tx = Transaction::new_signed_with_payer(
        &[ix],
        Some(&authority.pubkey()),
        &[&authority],
        svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Zero allocation should fail");
}
