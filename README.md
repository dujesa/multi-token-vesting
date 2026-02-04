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

## Building

```bash
cargo build-sbf
```

## Testing

```bash
cargo test
```