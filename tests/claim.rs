use litesvm::LiteSVM;
use litesvm_token::{CreateAssociatedTokenAccount, CreateMint, MintTo, spl_token};
use solana_sdk::{
    account::ReadableAccount,
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

fn get_token_balance(svm: &LiteSVM, ata: &Pubkey) -> u64 {
    let account = svm.get_account(ata).expect("ATA not found");
    let data = account.data();
    // Token account balance is at bytes 64-72
    u64::from_le_bytes(data[64..72].try_into().unwrap())
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

/// Setup schedule + participant, returns (schedule, vault, mint, vested_participant_pda)
fn setup_vesting(
    svm: &mut LiteSVM,
    authority: &Keypair,
    participant: &Keypair,
    seed: u64,
    allocation: u64,
) -> (Pubkey, Pubkey, Pubkey, Pubkey) {
    let mint = CreateMint::new(svm, authority).decimals(9).send().unwrap();
    let (schedule, bump) = get_schedule_pda(seed);
    let vault = get_ata(&schedule, &mint);

    // Initialize: start=1000, cliff=100, step=50, total=300
    let ix = build_initialize_ix(
        &authority.pubkey(), &schedule, &mint, &vault,
        1000, 100, 50, 300, seed, bump,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&authority.pubkey()), &[authority], svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("Initialize failed");

    // Fund authority ATA
    let authority_ata = get_ata(&authority.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(svm, authority, &mint)
        .owner(&authority.pubkey()).send().unwrap();
    MintTo::new(svm, authority, &mint, &authority_ata, allocation).send().unwrap();

    // Add participant
    let (vested_participant_pda, _) = get_participant_pda(&participant.pubkey(), &schedule);
    let ix = build_add_participant_ix(
        &authority.pubkey(), &authority_ata, &vault,
        &participant.pubkey(), &vested_participant_pda, &schedule, &mint, allocation,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&authority.pubkey()), &[authority], svm.latest_blockhash(),
    );
    svm.send_transaction(tx).expect("AddParticipant failed");

    (schedule, vault, mint, vested_participant_pda)
}

#[test]
fn test_claim_success() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 1000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Warp AFTER cliff: start=1000, cliff=100 -> cliff_end=1100
    svm.set_sysvar(&Clock { unix_timestamp: 1200, ..Default::default() });

    // Pre-create participant ATA (required since ATA program not in LiteSVM)
    let participant_ata = get_ata(&participant.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &participant, &mint)
        .owner(&participant.pubkey())
        .send()
        .unwrap();

    let ix = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Claim should succeed: {:?}", result.err());
}

#[test]
fn test_claim_before_cliff_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 2000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Time is BEFORE cliff: start=1000, cliff=100 -> cliff ends at 1100
    // Set to 1050: still within cliff period
    svm.set_sysvar(&Clock { unix_timestamp: 1050, ..Default::default() });

    let participant_ata = get_ata(&participant.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &participant, &mint)
        .owner(&participant.pubkey())
        .send()
        .unwrap();

    let ix = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Claim before cliff should fail");
}

#[test]
fn test_claim_wrong_signer_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    let attacker = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&attacker.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 3000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Warp past cliff
    svm.set_sysvar(&Clock { unix_timestamp: 1200, ..Default::default() });

    // Attacker tries to claim participant's tokens
    let attacker_ata = get_ata(&attacker.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &attacker, &mint)
        .owner(&attacker.pubkey())
        .send()
        .unwrap();

    let ix = build_claim_ix(
        &attacker.pubkey(),  // WRONG signer
        &vested_participant_pda,
        &attacker_ata,
        &vault,
        &schedule,
        &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&attacker.pubkey()), &[&attacker], svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_err(), "Wrong signer should fail");
}

#[test]
fn test_claim_double_claim_fails() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 4000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Warp to AFTER full vesting: start=1000, total=300 -> ends at 1300
    svm.set_sysvar(&Clock { unix_timestamp: 1400, ..Default::default() });

    let participant_ata = get_ata(&participant.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &participant, &mint)
        .owner(&participant.pubkey())
        .send()
        .unwrap();

    // First claim - should succeed (100% vested)
    let ix = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );
    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "First claim should succeed: {:?}", result.err());

    // Second claim - should fail (already fully claimed)
    let ix2 = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx2 = Transaction::new_signed_with_payer(
        &[ix2], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );
    let result2 = svm.send_transaction(tx2);
    assert!(result2.is_err(), "Double claim should fail");
}

#[test]
fn test_claim_right_after_cliff() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 5000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Clock: 1101 (just past cliff, before step 2)
    // start=1000, cliff=100 -> cliff ends at 1100
    // Periods passed: 1 (cliff only)
    // Expected: 1/5 = 20% -> 200_000_000 tokens
    svm.set_sysvar(&Clock { unix_timestamp: 1101, ..Default::default() });

    let participant_ata = get_ata(&participant.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &participant, &mint)
        .owner(&participant.pubkey())
        .send()
        .unwrap();

    let ix = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Claim should succeed: {:?}", result.err());

    let balance = get_token_balance(&svm, &participant_ata);
    assert_eq!(balance, 200_000_000, "Should receive 20% (1/5) of allocation");
}

#[test]
fn test_claim_mid_vesting() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 6000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Clock: 1200 (step 3 complete)
    // start=1000, cliff=100, step=50
    // cliff ends at 1100, step 2 at 1150, step 3 at 1200
    // Periods passed: 3 (cliff + 2 steps)
    // Expected: 3/5 = 60% -> 600_000_000 tokens
    svm.set_sysvar(&Clock { unix_timestamp: 1200, ..Default::default() });

    let participant_ata = get_ata(&participant.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &participant, &mint)
        .owner(&participant.pubkey())
        .send()
        .unwrap();

    let ix = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Claim should succeed: {:?}", result.err());

    let balance = get_token_balance(&svm, &participant_ata);
    assert_eq!(balance, 600_000_000, "Should receive 60% (3/5) of allocation");
}

#[test]
fn test_claim_after_vesting_complete() {
    let mut svm = setup_svm();

    let authority = Keypair::new();
    let participant = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    svm.airdrop(&participant.pubkey(), 10_000_000_000).unwrap();

    svm.set_sysvar(&Clock { unix_timestamp: 500, ..Default::default() });

    let seed: u64 = 7000;
    let allocation: u64 = 1_000_000_000;
    let (schedule, vault, mint, vested_participant_pda) =
        setup_vesting(&mut svm, &authority, &participant, seed, allocation);

    // Clock: 1400 (past end)
    // start=1000, total=300 -> ends at 1300
    // Expected: 100% -> 1_000_000_000 tokens
    svm.set_sysvar(&Clock { unix_timestamp: 1400, ..Default::default() });

    let participant_ata = get_ata(&participant.pubkey(), &mint);
    CreateAssociatedTokenAccount::new(&mut svm, &participant, &mint)
        .owner(&participant.pubkey())
        .send()
        .unwrap();

    let ix = build_claim_ix(
        &participant.pubkey(), &vested_participant_pda, &participant_ata,
        &vault, &schedule, &mint,
    );
    let tx = Transaction::new_signed_with_payer(
        &[ix], Some(&participant.pubkey()), &[&participant], svm.latest_blockhash(),
    );

    let result = svm.send_transaction(tx);
    assert!(result.is_ok(), "Claim should succeed: {:?}", result.err());

    let balance = get_token_balance(&svm, &participant_ata);
    assert_eq!(balance, 1_000_000_000, "Should receive 100% of allocation");
}
