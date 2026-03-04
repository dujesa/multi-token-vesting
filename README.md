# Multi-Token Vesting

A Solana program for token vesting with cliff and linear unlock schedules. Built with [Pinocchio](https://github.com/febo/pinocchio) for minimal compute usage.

## Features

- **Cliff vesting**: Tokens locked until cliff period ends
- **Step-based unlocking**: Linear vesting in configurable time steps
- **Multi-schedule support**: Create multiple vesting schedules with unique seeds
- **Per-participant tracking**: Individual allocation and claim tracking

## Instructions

### Initialize

Creates a new vesting schedule.

### AddParticipant

Adds a participant to a vesting schedule and transfers their allocation to the vault.

**Constraints:**
- Must be called before cliff ends
- Only schedule authority can add participants
- Authority must have sufficient token balance

### Claim

Participant claims their vested tokens.

**Constraints:**
- Cliff must be completed
- Only the participant can claim their tokens
- Cannot claim more than vested amount
- Cannot claim after fully vested (double-claim prevention)

## PDAs

| PDA | Seeds |
|-----|-------|
| Schedule | `["schedule", seed.to_le_bytes()]` |
| VestedParticipant | `["participant", participant_wallet, schedule]` |

## Prerequisites

- Rust (with `cargo`)
- Solana CLI (`solana`, `solana-test-validator`)
- PostgreSQL

## Building

```bash
cargo build-sbf --manifest-path program/Cargo.toml
```

## Testing

```bash
cargo test
```

## Indexer

The indexer watches on-chain vesting transactions and stores them in Postgres. It uses [Carbon](https://github.com/sevenlabs-hq/carbon) for block crawling and live subscription.

### Setup

1. Create the database:

```bash
createdb vesting_indexer
```

2. Copy the env file and configure it:

```bash
cp indexer/.env.example indexer/.env
```

Set at minimum:
```
RPC_URL=http://127.0.0.1:8899
WS_URL=ws://127.0.0.1:8900
DATABASE_URL=postgres://<your-user>@localhost:5432/vesting_indexer
START_SLOT=0
KEYPAIR_PATH=~/.config/solana/id.json
```

3. Start a local validator with the program loaded and block subscriptions enabled:

```bash
solana-test-validator \
  --rpc-pubsub-enable-block-subscription \
  --bpf-program FwnGeaANDtRZHA1xXzjyTjr5mmEZtXBSKuA3umcRPiWG target/deploy/multi_token_vesting.so \
  --reset
```

### Seeding test data

The seed tool sends Initialize, AddParticipant, and Claim transactions to the running validator:

```bash
RUST_LOG=info cargo run -p vesting-indexer --bin seed
```

Copy the `START_SLOT` from the output into your `indexer/.env`.

### Running the indexer

```bash
RUST_LOG=info cargo run -p vesting-indexer
```

The indexer will backfill from `START_SLOT` and then subscribe to new blocks. Verify data with:

```bash
psql -d vesting_indexer -c "SELECT * FROM schedules;"
psql -d vesting_indexer -c "SELECT * FROM participants;"
psql -d vesting_indexer -c "SELECT * FROM claims;"
```

To test live subscription, run the seed tool again while the indexer is running — new transactions will appear in the database in real time.