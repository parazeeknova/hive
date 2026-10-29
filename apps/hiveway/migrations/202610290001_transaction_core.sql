CREATE TABLE wallet_challenges (
    id UUID PRIMARY KEY,
    wallet TEXT NOT NULL,
    peer_ip TEXT NOT NULL,
    message TEXT NOT NULL,
    action JSONB NOT NULL,
    expires_at BIGINT NOT NULL,
    consumed_at BIGINT,
    created_at BIGINT NOT NULL
);
CREATE INDEX wallet_challenges_ip_rate ON wallet_challenges (peer_ip, created_at);
CREATE INDEX wallet_challenges_rate ON wallet_challenges (wallet, created_at);

CREATE TABLE agent_credentials (
    id UUID PRIMARY KEY,
    wallet TEXT NOT NULL,
    name TEXT NOT NULL,
    token_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(token_hash) = 32),
    scopes TEXT[] NOT NULL,
    per_call_limit BIGINT NOT NULL CHECK (per_call_limit > 0),
    total_limit BIGINT NOT NULL CHECK (total_limit > 0),
    spent BIGINT NOT NULL DEFAULT 0 CHECK (spent >= 0),
    reserved BIGINT NOT NULL DEFAULT 0 CHECK (reserved >= 0),
    expires_at BIGINT NOT NULL,
    revoked_at BIGINT,
    created_at BIGINT NOT NULL,
    CHECK (spent + reserved <= total_limit)
);
CREATE INDEX agent_credentials_owner ON agent_credentials (wallet);

CREATE TABLE capabilities (
    id UUID PRIMARY KEY,
    creator_wallet TEXT NOT NULL,
    name TEXT NOT NULL CHECK (length(name) BETWEEN 1 AND 80),
    description TEXT NOT NULL CHECK (length(description) BETWEEN 1 AND 1000),
    version TEXT NOT NULL,
    runtime TEXT NOT NULL CHECK (runtime = 'repository_stack'),
    price BIGINT NOT NULL CHECK (price BETWEEN 1 AND 1000000000),
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at BIGINT NOT NULL
);

CREATE TABLE invocations (
    id UUID PRIMARY KEY,
    credential_id UUID NOT NULL REFERENCES agent_credentials(id),
    capability_id UUID NOT NULL REFERENCES capabilities(id),
    idempotency_key UUID NOT NULL,
    request_hash BYTEA NOT NULL,
    input JSONB NOT NULL,
    requirements JSONB NOT NULL,
    resource JSONB NOT NULL,
    state TEXT NOT NULL CHECK (state IN ('quoted','verifying','executing','settling','settlement_unknown','settled','failed','expired')),
    price BIGINT NOT NULL CHECK (price > 0),
    expires_at BIGINT NOT NULL,
    payment_nonce TEXT UNIQUE,
    payment_payload JSONB,
    result JSONB,
    settlement JSONB,
    settlement_block TEXT,
    error_code TEXT,
    created_at BIGINT NOT NULL,
    updated_at BIGINT NOT NULL,
    UNIQUE (credential_id, idempotency_key),
    CHECK (state <> 'settled' OR (result IS NOT NULL AND settlement IS NOT NULL))
);
CREATE INDEX invocations_expiry ON invocations (credential_id, expires_at) WHERE state = 'quoted';
CREATE INDEX invocations_recovery ON invocations (state) WHERE state IN ('verifying','executing','settling','settlement_unknown');

CREATE TABLE creator_sales (
    invocation_id UUID PRIMARY KEY REFERENCES invocations(id),
    creator_wallet TEXT NOT NULL,
    payer_wallet TEXT NOT NULL,
    amount BIGINT NOT NULL CHECK (amount > 0),
    network TEXT NOT NULL,
    transaction_hash TEXT NOT NULL UNIQUE,
    created_at BIGINT NOT NULL
);
