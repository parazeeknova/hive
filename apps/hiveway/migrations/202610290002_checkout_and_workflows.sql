ALTER TABLE capabilities DROP CONSTRAINT capabilities_runtime_check;
ALTER TABLE capabilities ADD CONSTRAINT capabilities_runtime_check CHECK (runtime IN ('repository_stack','dependency_inventory','workflow'));
ALTER TABLE capabilities ADD COLUMN definition JSONB NOT NULL DEFAULT '{}'::jsonb;
ALTER TABLE invocations DROP CONSTRAINT invocations_state_check;
ALTER TABLE invocations ADD CONSTRAINT invocations_state_check CHECK (state IN ('quoted','verifying','executing','settling','settlement_unknown','settled','failed','expired','checkout_creating','checkout_pending','paid','execution_failed'));
ALTER TABLE invocations ADD COLUMN payment_method TEXT CHECK (payment_method IN ('x402','dodo'));
ALTER TABLE invocations ADD COLUMN checkout_id TEXT UNIQUE;
ALTER TABLE invocations ADD COLUMN checkout_url TEXT;
ALTER TABLE invocations ADD COLUMN provider_payment_id TEXT UNIQUE;
ALTER TABLE invocations ADD COLUMN worker_started_at BIGINT;
CREATE TABLE payment_webhook_inbox (
    event_id TEXT PRIMARY KEY,
    event_type TEXT NOT NULL,
    payment_id TEXT,
    received_at BIGINT NOT NULL,
    processed_at BIGINT,
    attempts INTEGER NOT NULL DEFAULT 0,
    last_error TEXT
);
