CREATE TABLE IF NOT EXISTS schedules (
    schedule_address TEXT PRIMARY KEY,
    mint TEXT NOT NULL,
    authority TEXT NOT NULL,
    seed BIGINT NOT NULL,
    start_timestamp BIGINT NOT NULL,
    cliff_duration BIGINT NOT NULL,
    step_duration BIGINT NOT NULL,
    total_duration BIGINT NOT NULL,
    bump SMALLINT NOT NULL,
    vault TEXT NOT NULL,
    tx_signature TEXT NOT NULL,
    slot BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS participants (
    participant_pda TEXT PRIMARY KEY,
    schedule_address TEXT NOT NULL,
    participant_wallet TEXT NOT NULL,
    allocated_amount BIGINT NOT NULL,
    tx_signature TEXT NOT NULL,
    slot BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS claims (
    id SERIAL PRIMARY KEY,
    participant_pda TEXT NOT NULL,
    schedule_address TEXT NOT NULL,
    participant_wallet TEXT NOT NULL,
    claimed_amount BIGINT NOT NULL,
    tx_signature TEXT NOT NULL UNIQUE,
    slot BIGINT NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_participants_schedule ON participants(schedule_address);
CREATE INDEX IF NOT EXISTS idx_participants_wallet ON participants(participant_wallet);
CREATE INDEX IF NOT EXISTS idx_claims_participant ON claims(participant_pda);
CREATE INDEX IF NOT EXISTS idx_claims_schedule ON claims(schedule_address);
