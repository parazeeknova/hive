//! Agent platform storage: durable connector tasks, bridged resources and prompts,
//! MCP OAuth, discovery ranking signals, commit-addressed code intelligence and
//! repository memory. Every table is owner-scoped.
use sea_orm_migration::prelude::*;
#[derive(DeriveMigrationName)]
pub struct Migration;

const UP: &str = r"
ALTER TABLE mcp_connections
  ADD COLUMN IF NOT EXISTS resources jsonb NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS resource_templates jsonb NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS prompts jsonb NOT NULL DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS features text[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS snapshot_hash text NOT NULL DEFAULT '';

ALTER TABLE mcp_jobs
  ADD COLUMN IF NOT EXISTS method text NOT NULL DEFAULT 'tools/call',
  ADD COLUMN IF NOT EXISTS status text NOT NULL DEFAULT 'queued',
  ADD COLUMN IF NOT EXISTS status_message text,
  ADD COLUMN IF NOT EXISTS input_requests jsonb,
  ADD COLUMN IF NOT EXISTS input_responses jsonb,
  ADD COLUMN IF NOT EXISTS error jsonb,
  ADD COLUMN IF NOT EXISTS cancel_requested boolean NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS detached boolean NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS input_modes text[] NOT NULL DEFAULT '{}',
  ADD COLUMN IF NOT EXISTS cache_key text,
  ADD COLUMN IF NOT EXISTS claim_expires_at bigint,
  ADD COLUMN IF NOT EXISTS created_at bigint NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS updated_at bigint NOT NULL DEFAULT 0;
UPDATE mcp_jobs SET
  status = CASE WHEN result IS NOT NULL THEN 'completed' WHEN lease IS NOT NULL THEN 'working' ELSE 'queued' END,
  claim_expires_at = expires_at,
  created_at = expires_at - 35,
  updated_at = expires_at - 35
  WHERE claim_expires_at IS NULL;
ALTER TABLE mcp_jobs ADD CONSTRAINT mcp_jobs_status CHECK (status IN ('queued','working','input_required','completed','failed','cancelled'));
CREATE INDEX IF NOT EXISTS mcp_jobs_claimable ON mcp_jobs(connection_id, claim_expires_at) WHERE lease IS NULL AND status = 'queued';
CREATE INDEX IF NOT EXISTS mcp_jobs_caller ON mcp_jobs(caller, created_at DESC);

ALTER TABLE mcp_cache
  ADD COLUMN IF NOT EXISTS created_at bigint NOT NULL DEFAULT 0,
  ADD COLUMN IF NOT EXISTS hits bigint NOT NULL DEFAULT 0;

CREATE TABLE oauth_clients (
  client_id text PRIMARY KEY,
  kind text NOT NULL CHECK (kind IN ('dynamic','metadata_document')),
  client_name text NOT NULL,
  client_uri text,
  redirect_uris jsonb NOT NULL,
  application_type text NOT NULL DEFAULT 'native',
  created_at bigint NOT NULL,
  last_used_at bigint NOT NULL,
  metadata_expires_at bigint
);
CREATE TABLE oauth_grants (
  id uuid PRIMARY KEY,
  client_id text NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  owner text NOT NULL,
  scopes text[] NOT NULL,
  created_at bigint NOT NULL,
  last_used_at bigint NOT NULL,
  revoked_at bigint
);
CREATE INDEX oauth_grants_owner ON oauth_grants(owner, created_at DESC);
CREATE TABLE oauth_codes (
  code_hash bytea PRIMARY KEY,
  client_id text NOT NULL REFERENCES oauth_clients(client_id) ON DELETE CASCADE,
  owner text NOT NULL,
  redirect_uri text NOT NULL,
  code_challenge text NOT NULL,
  scopes text[] NOT NULL,
  resource text NOT NULL,
  expires_at bigint NOT NULL,
  used_at bigint,
  grant_id uuid REFERENCES oauth_grants(id) ON DELETE SET NULL
);
CREATE INDEX oauth_codes_expiry ON oauth_codes(expires_at);
CREATE TABLE oauth_tokens (
  credential_id uuid PRIMARY KEY REFERENCES agent_credentials(id) ON DELETE CASCADE,
  grant_id uuid NOT NULL REFERENCES oauth_grants(id) ON DELETE CASCADE
);
CREATE INDEX oauth_tokens_grant ON oauth_tokens(grant_id);
CREATE TABLE oauth_refresh_tokens (
  token_hash bytea PRIMARY KEY,
  grant_id uuid NOT NULL REFERENCES oauth_grants(id) ON DELETE CASCADE,
  expires_at bigint NOT NULL,
  used_at bigint
);
CREATE INDEX oauth_refresh_grant ON oauth_refresh_tokens(grant_id);
CREATE INDEX oauth_refresh_expiry ON oauth_refresh_tokens(expires_at);

CREATE TABLE tool_usage (
  owner text NOT NULL,
  tool text NOT NULL,
  calls bigint NOT NULL DEFAULT 0,
  failures bigint NOT NULL DEFAULT 0,
  last_used_at bigint NOT NULL,
  PRIMARY KEY(owner, tool)
);
CREATE TABLE tool_embeddings (
  text_hash text NOT NULL,
  model text NOT NULL,
  vector real[] NOT NULL,
  created_at bigint NOT NULL,
  PRIMARY KEY(text_hash, model)
);

CREATE TABLE code_blobs (
  owner text NOT NULL,
  sha text NOT NULL CHECK (sha ~ '^[0-9a-f]{40}$'),
  bytes integer NOT NULL,
  language text NOT NULL,
  content text NOT NULL,
  search tsvector NOT NULL,
  created_at bigint NOT NULL,
  PRIMARY KEY(owner, sha)
);
CREATE INDEX code_blobs_search ON code_blobs USING gin(search);
CREATE TABLE code_symbols (
  owner text NOT NULL,
  sha text NOT NULL,
  name text NOT NULL,
  lname text NOT NULL,
  kind text NOT NULL,
  line integer NOT NULL,
  signature text NOT NULL,
  PRIMARY KEY(owner, sha, line, name),
  FOREIGN KEY(owner, sha) REFERENCES code_blobs(owner, sha) ON DELETE CASCADE
);
CREATE INDEX code_symbols_name ON code_symbols(owner, lname);
CREATE TABLE code_snapshots (
  owner text NOT NULL,
  repo text NOT NULL,
  commit_sha text NOT NULL CHECK (commit_sha ~ '^[0-9a-f]{40}$'),
  ref_name text,
  status text NOT NULL CHECK (status IN ('pending','ready')),
  files integer NOT NULL DEFAULT 0,
  bytes bigint NOT NULL DEFAULT 0,
  summary jsonb NOT NULL DEFAULT '{}'::jsonb,
  created_at bigint NOT NULL,
  updated_at bigint NOT NULL,
  PRIMARY KEY(owner, repo, commit_sha)
);
CREATE INDEX code_snapshots_recent ON code_snapshots(owner, repo, updated_at DESC);
CREATE TABLE code_files (
  owner text NOT NULL,
  repo text NOT NULL,
  commit_sha text NOT NULL,
  path text NOT NULL,
  sha text NOT NULL,
  bytes integer NOT NULL,
  PRIMARY KEY(owner, repo, commit_sha, path),
  FOREIGN KEY(owner, repo, commit_sha) REFERENCES code_snapshots(owner, repo, commit_sha) ON DELETE CASCADE
);
CREATE INDEX code_files_blob ON code_files(owner, sha);

CREATE TABLE agent_memories (
  id uuid PRIMARY KEY,
  owner text NOT NULL,
  repo text NOT NULL,
  kind text NOT NULL,
  title text NOT NULL,
  body text NOT NULL,
  tags text[] NOT NULL DEFAULT '{}',
  pinned boolean NOT NULL DEFAULT false,
  source jsonb NOT NULL DEFAULT '{}'::jsonb,
  search tsvector NOT NULL,
  created_at bigint NOT NULL,
  updated_at bigint NOT NULL,
  expires_at bigint,
  superseded_by uuid REFERENCES agent_memories(id) ON DELETE SET NULL
);
CREATE INDEX agent_memories_repo ON agent_memories(owner, repo, updated_at DESC);
CREATE INDEX agent_memories_search ON agent_memories USING gin(search);
CREATE UNIQUE INDEX agent_memories_title ON agent_memories(owner, repo, kind, lower(title)) WHERE superseded_by IS NULL;
";

const DOWN: &str = r"
DROP TABLE agent_memories;
DROP TABLE code_files;
DROP TABLE code_snapshots;
DROP TABLE code_symbols;
DROP TABLE code_blobs;
DROP TABLE tool_embeddings;
DROP TABLE tool_usage;
DROP TABLE oauth_refresh_tokens;
DROP TABLE oauth_tokens;
DROP TABLE oauth_codes;
DROP TABLE oauth_grants;
DROP TABLE oauth_clients;
ALTER TABLE mcp_cache DROP COLUMN hits, DROP COLUMN created_at;
DROP INDEX mcp_jobs_caller;
DROP INDEX mcp_jobs_claimable;
ALTER TABLE mcp_jobs DROP CONSTRAINT mcp_jobs_status;
ALTER TABLE mcp_jobs DROP COLUMN updated_at, DROP COLUMN created_at, DROP COLUMN claim_expires_at,
  DROP COLUMN cache_key, DROP COLUMN input_modes, DROP COLUMN detached, DROP COLUMN cancel_requested, DROP COLUMN error, DROP COLUMN input_responses,
  DROP COLUMN input_requests, DROP COLUMN status_message, DROP COLUMN status, DROP COLUMN method;
ALTER TABLE mcp_connections DROP COLUMN snapshot_hash, DROP COLUMN features, DROP COLUMN prompts,
  DROP COLUMN resource_templates, DROP COLUMN resources;
";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(UP).await?;
        Ok(())
    }
    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared(DOWN).await?;
        Ok(())
    }
}
