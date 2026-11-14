//! Validated, public operational configuration. Provider credentials stay in env.
use anyhow::{Context, bail};
use serde::Deserialize;
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfig {
    pub listen_address: std::net::SocketAddr,
    pub public_url: String,
    pub auth_origin: String,
    pub facilitator_url: String,
    pub rpc_url: String,
    pub dodo_api_url: String,
    pub http: HttpLimits,
    pub infrastructure: Infrastructure,
    pub worker: Worker,
    pub cache: Cache,
    pub payment: Payment,
    pub sandbox: SandboxLimits,
    pub identity: IdentityLimits,
    pub connectors: ConnectorLimits,
    pub mcp: McpProtocol,
    pub discovery: Discovery,
    pub oauth: OAuth,
    pub code_index: CodeIndex,
    pub memory: Memory,
    pub workflows: Workflows,
    pub providers: Providers,
}
/// Bring-your-own-key providers that Hiveway calls on an account's behalf.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Providers {
    /// Concurrent upstream calls per process across all accounts.
    pub concurrency: usize,
    /// Budget for validating a key (initialize plus tools/list).
    pub validation_timeout_seconds: u64,
    /// Reuse an upstream MCP session for this long before re-initializing.
    pub session_ttl_seconds: u64,
    pub sessions_cached: usize,
    pub key_bytes: usize,
    /// Connection-level retries before a request is sent; never after.
    pub connect_retries: u32,
}
impl Providers {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=4096).contains(&self.concurrency)
            || !(1..=120).contains(&self.validation_timeout_seconds)
            || !(10..=86_400).contains(&self.session_ttl_seconds)
            || !(1..=100_000).contains(&self.sessions_cached)
            || !(16..=8192).contains(&self.key_bytes)
            || self.connect_retries > 5
        {
            bail!("invalid provider limits");
        }
        Ok(())
    }
}
const fn default_queue_depth() -> usize {
    512
}
const fn default_queue_wait_ms() -> u64 {
    3000
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpLimits {
    pub concurrency: usize,
    /// Requests that may wait for an active slot during a burst, and for how long.
    /// Defaults keep older override files valid.
    #[serde(default = "default_queue_depth")]
    pub queue_depth: usize,
    #[serde(default = "default_queue_wait_ms")]
    pub queue_wait_ms: u64,
    /// Anonymous requests per client address and minute.
    pub requests_per_minute: u32,
    /// Requests per bearer credential and minute, shared across replicas through Redis.
    pub credential_requests_per_minute: u32,
    /// Flood ceiling for one address that presents bearer credentials, so CI runners
    /// behind one NAT do not throttle each other while random tokens cannot bypass limits.
    pub authenticated_ip_requests_per_minute: u32,
    /// Redis rate-limit budget; slower answers fall back to the process-local window.
    pub rate_limit_redis_timeout_ms: u64,
    /// Requests parked in a long-poll or task wait. They hold no database
    /// connection, so they are admitted separately from `concurrency`.
    pub idle_waiters: usize,
    pub tracked_peers: usize,
    pub body_bytes: usize,
    pub body_timeout_seconds: u64,
    pub request_timeout_seconds: u64,
    pub shutdown_drain_timeout_seconds: u64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Infrastructure {
    pub postgres_connections: u32,
    pub connection_timeout_seconds: u64,
    pub outbound_timeout_seconds: u64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Worker {
    pub interval_seconds: u64,
    pub challenge_retention_seconds: i64,
    pub batch_size: i64,
    pub webhook_max_attempts: u32,
    pub execution_timeout_seconds: u64,
    pub stale_after_seconds: i64,
    pub builder_interval_seconds: u64,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cache {
    pub connected_tool_ttl_seconds: u64,
    pub connected_tool_max_bytes: usize,
    /// Freshness hint for `tools/list`, `prompts/list` and `resources/list` results.
    pub list_ttl_ms: u64,
    /// Freshness hint for `server/discover`, which does not vary per caller.
    pub discover_ttl_ms: u64,
}
/// Wallet challenges, browser sessions and account API keys.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdentityLimits {
    pub challenge_ttl_seconds: i64,
    pub challenges_per_peer_minute: i64,
    pub session_ttl_seconds: i64,
    pub session_per_call_limit: i64,
    pub session_total_limit: i64,
    pub api_keys_per_account: u64,
    pub api_key_default_lifetime_seconds: i64,
    pub api_key_lifetimes_seconds: Vec<i64>,
}
/// Outbound connector queues. Every bound protects the shared Postgres queue.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConnectorLimits {
    pub connections_per_account: i64,
    pub tools_per_connection: usize,
    pub resources_per_connection: usize,
    pub prompts_per_connection: usize,
    pub snapshot_bytes: usize,
    pub argument_bytes: usize,
    pub result_bytes: usize,
    pub jobs_per_connection: i64,
    pub shared_jobs_per_caller: i64,
    /// A connection without a poll inside this window is offline.
    pub online_window_seconds: i64,
    /// A queued job that no connector claims within this window fails.
    pub claim_timeout_seconds: i64,
    /// Upper bound for one connected operation, including user input and task polling.
    pub job_ttl_seconds: i64,
    /// How long a synchronous `tools/call` waits before it returns a task handle.
    pub call_wait_seconds: u64,
    /// Longest connector long-poll.
    pub poll_wait_seconds: u64,
    /// Safety re-check interval while waiting for a notification.
    pub fallback_poll_ms: u64,
    pub task_poll_interval_ms: u64,
    pub input_request_bytes: usize,
    pub input_requests_per_job: usize,
}
/// MCP transport behaviour beyond request handling.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpProtocol {
    pub subscription_seconds: u64,
    pub subscriptions_per_credential: usize,
    pub subscriptions_per_process: usize,
    pub keepalive_seconds: u64,
}
/// `search_tools` ranking. Weights are additive score components.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Discovery {
    pub default_results: u64,
    pub max_results: u64,
    pub response_bytes: usize,
    pub catalog_candidates: u64,
    pub catalog_listing: u64,
    pub max_query_terms: usize,
    pub stopwords: Vec<String>,
    pub lexical_weight: f64,
    pub semantic_weight: f64,
    pub semantic_floor: f64,
    pub usage_weight: f64,
    pub stack_weight: f64,
    pub embedding_batch: usize,
    pub embedding_timeout_ms: u64,
}
/// MCP authorization server for remote clients.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OAuth {
    pub access_token_ttl_seconds: i64,
    pub refresh_token_ttl_seconds: i64,
    pub code_ttl_seconds: i64,
    pub client_metadata_bytes: usize,
    pub client_metadata_cache_seconds: i64,
    pub redirect_uris_per_client: usize,
    pub unused_client_retention_seconds: i64,
}
/// Commit-addressed code intelligence storage.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodeIndex {
    pub file_bytes: usize,
    pub files_per_snapshot: i64,
    pub manifest_files_per_call: usize,
    pub blobs_per_upload: usize,
    pub owner_storage_bytes: i64,
    pub snapshots_per_repo: i64,
    pub search_results: u64,
    pub snippets_per_file: usize,
    pub read_lines: usize,
    pub symbols_per_file: usize,
}
/// Multi-tool workflows and automatic publishing of repeated tool chains.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workflows {
    /// Steps per workflow definition.
    pub max_steps: usize,
    /// Items one `for_each` step may fan out to.
    pub max_items: usize,
    /// Tool calls one run may make, across steps and items.
    pub max_tool_calls: usize,
    /// Steps (or items) that run at the same time within one run.
    pub step_concurrency: usize,
    /// Nested workflow calls.
    pub max_depth: usize,
    /// How long one step waits for its tool, and the deadline for a whole run.
    pub step_wait_seconds: u64,
    pub run_seconds: u64,
    /// How long `run_workflow` waits before returning a run handle.
    pub call_wait_seconds: u64,
    pub runs_per_owner: usize,
    pub runs_per_process: usize,
    pub run_retention_seconds: i64,
    /// Bytes of one step's result kept for later steps and the stored run.
    pub step_result_bytes: usize,
    pub definition_bytes: usize,
    pub workflows_per_owner: usize,
    /// Publish a private workflow after a tool chain repeats this often (0 disables).
    pub auto_publish_after: u32,
    /// Window in which consecutive calls can form a chain.
    pub chain_window_seconds: u64,
    /// Accounts whose recent calls are tracked per process.
    pub tracked_owners: usize,
}
impl Workflows {
    fn validate(&self, connectors: &ConnectorLimits) -> anyhow::Result<()> {
        if !(1..=32).contains(&self.max_steps)
            || !(1..=200).contains(&self.max_items)
            || !(1..=500).contains(&self.max_tool_calls)
            || !(1..=16).contains(&self.step_concurrency)
            || self.max_depth > 4
            || !(1..=1800).contains(&self.step_wait_seconds)
            || self.run_seconds < self.step_wait_seconds
            || self.run_seconds > 7200
            || self.call_wait_seconds > self.run_seconds
            || !(1..=64).contains(&self.runs_per_owner)
            || !(1..=4096).contains(&self.runs_per_process)
            || self.run_retention_seconds < 60
            || !(1024..=connectors.result_bytes).contains(&self.step_result_bytes)
            || !(1024..=262_144).contains(&self.definition_bytes)
            || !(1..=1000).contains(&self.workflows_per_owner)
            || self.auto_publish_after > 100
            || !(10..=86_400).contains(&self.chain_window_seconds)
            || !(1..=1_000_000).contains(&self.tracked_owners)
        {
            bail!("invalid workflow limits");
        }
        Ok(())
    }
}
/// Repository memory and decisions.
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Memory {
    pub entries_per_repo: i64,
    pub title_bytes: usize,
    pub body_bytes: usize,
    pub tags_per_entry: usize,
    pub recall_results: u64,
    #[serde(default = "memory_response_bytes")]
    pub response_bytes: usize,
}
const fn memory_response_bytes() -> usize {
    16_384
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SandboxLimits {
    pub concurrency: usize,
    pub artifact_bytes: usize,
    pub input_bytes: usize,
    pub output_bytes: usize,
    pub schema_bytes: usize,
    pub memory_bytes: usize,
    pub fuel: u64,
    pub artifacts_per_wallet: u64,
    pub json_depth: usize,
    pub json_nodes: usize,
    pub collection_items: usize,
}
impl SandboxLimits {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=16).contains(&self.concurrency)
            || !(8..=1_048_576).contains(&self.artifact_bytes)
            || !(1024..=262_144).contains(&self.input_bytes)
            || !(1024..=262_144).contains(&self.output_bytes)
            || !(256..=16_384).contains(&self.schema_bytes)
            || !(65_536..=67_108_864).contains(&self.memory_bytes)
            || !(100..=10_000_000).contains(&self.fuel)
            || !(1..=1000).contains(&self.artifacts_per_wallet)
            || !(1..=32).contains(&self.json_depth)
            || !(64..=16_384).contains(&self.json_nodes)
            || !(1..=1024).contains(&self.collection_items)
        {
            bail!("invalid sandbox resource limits");
        }
        Ok(())
    }
}
impl IdentityLimits {
    fn validate(&self) -> anyhow::Result<()> {
        if !(60..=3600).contains(&self.challenge_ttl_seconds)
            || !(1..=1000).contains(&self.challenges_per_peer_minute)
            || !(300..=2_592_000).contains(&self.session_ttl_seconds)
            || !(1..=1_000_000_000).contains(&self.session_per_call_limit)
            || self.session_total_limit < self.session_per_call_limit
            || self.session_total_limit > 1_000_000_000
            || !(1..=10_000).contains(&self.api_keys_per_account)
            || self.api_key_lifetimes_seconds.is_empty()
            || self.api_key_lifetimes_seconds.len() > 16
            || self
                .api_key_lifetimes_seconds
                .iter()
                .any(|s| !(300..=31_536_000).contains(s))
            || !self
                .api_key_lifetimes_seconds
                .contains(&self.api_key_default_lifetime_seconds)
        {
            bail!("invalid identity limits");
        }
        Ok(())
    }
}
impl ConnectorLimits {
    fn validate(&self, http: &HttpLimits) -> anyhow::Result<()> {
        // Results travel back through one HTTP request, so they must fit its body.
        let body = http.body_bytes;
        if !(1..=256).contains(&self.connections_per_account)
            || !(1..=1024).contains(&self.tools_per_connection)
            || self.resources_per_connection > 4096
            || self.prompts_per_connection > 1024
            || !(1024..=1_048_576).contains(&self.snapshot_bytes)
            || !(1024..=body).contains(&self.argument_bytes)
            || !(1024..body).contains(&self.result_bytes)
            || self.snapshot_bytes >= body
            || !(1..=1024).contains(&self.jobs_per_connection)
            || !(1..=self.jobs_per_connection).contains(&self.shared_jobs_per_caller)
            || !(10..=3600).contains(&self.online_window_seconds)
            || !(5..=600).contains(&self.claim_timeout_seconds)
            || !(30..=86_400).contains(&self.job_ttl_seconds)
            || self.job_ttl_seconds <= self.claim_timeout_seconds
            || !(1..=120).contains(&self.call_wait_seconds)
            || !(1..=60).contains(&self.poll_wait_seconds)
            || i64::try_from(self.poll_wait_seconds).unwrap_or(i64::MAX)
                >= self.online_window_seconds
            || !(50..=10_000).contains(&self.fallback_poll_ms)
            || !(100..=60_000).contains(&self.task_poll_interval_ms)
            || !(256..=65_536).contains(&self.input_request_bytes)
            || !(1..=16).contains(&self.input_requests_per_job)
        {
            bail!("invalid connector limits");
        }
        if self.call_wait_seconds >= http.request_timeout_seconds
            || self.poll_wait_seconds >= http.request_timeout_seconds
        {
            bail!("connector waits must finish inside the HTTP request timeout");
        }
        Ok(())
    }
}
impl McpProtocol {
    fn validate(&self) -> anyhow::Result<()> {
        if !(10..=3600).contains(&self.subscription_seconds)
            || !(1..=64).contains(&self.subscriptions_per_credential)
            || !(1..=100_000).contains(&self.subscriptions_per_process)
            || !(1..=120).contains(&self.keepalive_seconds)
            || self.keepalive_seconds >= self.subscription_seconds
        {
            bail!("invalid MCP protocol limits");
        }
        Ok(())
    }
}
impl Discovery {
    fn validate(&self) -> anyhow::Result<()> {
        let weight = |w: f64| w.is_finite() && (0.0..=100.0).contains(&w);
        if !(1..=self.max_results).contains(&self.default_results)
            || !(1..=64).contains(&self.max_results)
            || !(1024..=262_144).contains(&self.response_bytes)
            || !(1..=500).contains(&self.catalog_candidates)
            || !(1..=1000).contains(&self.catalog_listing)
            || !(1..=64).contains(&self.max_query_terms)
            || self.stopwords.len() > 512
            || self.stopwords.iter().any(|w| w.is_empty() || w.len() > 32)
            || ![
                self.lexical_weight,
                self.semantic_weight,
                self.usage_weight,
                self.stack_weight,
            ]
            .into_iter()
            .all(weight)
            || !(self.semantic_floor.is_finite() && (0.0..1.0).contains(&self.semantic_floor))
            || !(1..=256).contains(&self.embedding_batch)
            || !(100..=30_000).contains(&self.embedding_timeout_ms)
        {
            bail!("invalid discovery ranking configuration");
        }
        Ok(())
    }
}
impl OAuth {
    fn validate(&self) -> anyhow::Result<()> {
        if !(60..=86_400).contains(&self.access_token_ttl_seconds)
            || !(3600..=31_536_000).contains(&self.refresh_token_ttl_seconds)
            || self.refresh_token_ttl_seconds <= self.access_token_ttl_seconds
            || !(30..=600).contains(&self.code_ttl_seconds)
            || !(512..=65_536).contains(&self.client_metadata_bytes)
            || !(60..=86_400).contains(&self.client_metadata_cache_seconds)
            || !(1..=32).contains(&self.redirect_uris_per_client)
            || !(3600..=31_536_000).contains(&self.unused_client_retention_seconds)
        {
            bail!("invalid OAuth limits");
        }
        Ok(())
    }
}
impl CodeIndex {
    fn validate(&self, http: &HttpLimits) -> anyhow::Result<()> {
        if !(1024..=http.body_bytes / 2).contains(&self.file_bytes)
            || !(1..=1_000_000).contains(&self.files_per_snapshot)
            || !(1..=10_000).contains(&self.manifest_files_per_call)
            || !(1..=1024).contains(&self.blobs_per_upload)
            || !(1_048_576..=1_099_511_627_776).contains(&self.owner_storage_bytes)
            || !(1..=1000).contains(&self.snapshots_per_repo)
            || !(1..=200).contains(&self.search_results)
            || !(1..=20).contains(&self.snippets_per_file)
            || !(10..=5000).contains(&self.read_lines)
            || !(1..=10_000).contains(&self.symbols_per_file)
        {
            bail!("invalid code index limits");
        }
        Ok(())
    }
}
impl Memory {
    fn validate(&self) -> anyhow::Result<()> {
        if !(1..=100_000).contains(&self.entries_per_repo)
            || !(16..=1024).contains(&self.title_bytes)
            || !(64..=65_536).contains(&self.body_bytes)
            || self.tags_per_entry > 64
            || !(1..=200).contains(&self.recall_results)
            || !(1024..=262_144).contains(&self.response_bytes)
        {
            bail!("invalid memory limits");
        }
        Ok(())
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Payment {
    pub decimals: u8,
    pub settlement_finality: SettlementFinality,
    pub reorg_margin_seconds: u64,
    pub chain_id: u64,
    pub asset: String,
    pub name: String,
    pub version: String,
}
#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SettlementFinality {
    Finalized,
    Latest,
}
impl SettlementFinality {
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Finalized => "finalized",
            Self::Latest => "latest",
        }
    }
    /// Unfinalized settlement is restricted to isolated loopback development RPCs.
    /// # Errors
    /// Rejects a latest-block policy against a non-loopback RPC endpoint.
    pub fn validate_rpc(self, rpc_url: &str) -> anyhow::Result<()> {
        let url = reqwest::Url::parse(rpc_url)?;
        if self == Self::Latest
            && !matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
        {
            bail!("latest settlement finality requires a loopback development RPC");
        }
        Ok(())
    }
}
impl Payment {
    #[must_use]
    pub fn network(&self) -> String {
        format!("eip155:{}", self.chain_id)
    }
}
impl RuntimeConfig {
    /// Load the configured file or the bundled public defaults.
    /// # Errors
    /// Rejects missing files, malformed values, and limits outside safety bounds.
    pub fn load() -> anyhow::Result<Self> {
        let text = std::env::var("HIVE_CONFIG_FILE")
            .map_or_else(
                |_| Ok(include_str!("../config.toml").to_owned()),
                std::fs::read_to_string,
            )
            .context("reading Hiveway operational configuration")?;
        let config: Self =
            toml::from_str(&text).context("parsing Hiveway operational configuration")?;
        config.validate()?;
        Ok(config)
    }
    /// Check operational values before accepting traffic.
    /// # Errors
    /// Rejects zero, excessive, or inconsistent limits and malformed payment assets.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.sandbox.validate()?;
        let h = &self.http;
        let i = &self.infrastructure;
        let w = &self.worker;
        let c = &self.cache;
        // These are security ceilings, not runtime tuning defaults.
        if !(1..=1024).contains(&h.concurrency)
            || !(1..=100_000).contains(&h.requests_per_minute)
            || !(1..=65_536).contains(&h.tracked_peers)
            || !(1024..=1_048_576).contains(&h.body_bytes)
            || !(1..=30).contains(&h.body_timeout_seconds)
            || !(1..=600).contains(&h.request_timeout_seconds)
            || !(1..=600).contains(&h.shutdown_drain_timeout_seconds)
            || !(1..=100).contains(&i.postgres_connections)
            || !(1..=60).contains(&i.connection_timeout_seconds)
            || !(1..=60).contains(&i.outbound_timeout_seconds)
            || w.challenge_retention_seconds <= 60
            || !(1..=60).contains(&w.interval_seconds)
            || !(1..=100).contains(&w.batch_size)
            || !(1..=1000).contains(&w.webhook_max_attempts)
            || !(1..=120).contains(&w.execution_timeout_seconds)
            || w.stale_after_seconds
                <= i64::try_from(w.execution_timeout_seconds + i.outbound_timeout_seconds * 2)
                    .unwrap_or(i64::MAX)
            || h.request_timeout_seconds
                < w.execution_timeout_seconds + i.outbound_timeout_seconds * 2
            || c.connected_tool_ttl_seconds > 3600
            || !(1024..=524_288).contains(&c.connected_tool_max_bytes)
            || c.list_ttl_ms > 3_600_000
            || c.discover_ttl_ms > 86_400_000
            || !(1..=100_000).contains(&h.credential_requests_per_minute)
            || h.authenticated_ip_requests_per_minute < h.requests_per_minute
            || h.authenticated_ip_requests_per_minute > 1_000_000
            || !(1..=1000).contains(&h.rate_limit_redis_timeout_ms)
            || !(1..=100_000).contains(&h.idle_waiters)
            || h.queue_depth > 100_000
            || h.queue_wait_ms > 30_000
            || !(1..=60).contains(&w.builder_interval_seconds)
        {
            bail!("invalid or inconsistent Hiveway resource limits");
        }
        self.identity.validate()?;
        self.connectors.validate(h)?;
        self.mcp.validate()?;
        self.discovery.validate()?;
        self.oauth.validate()?;
        self.code_index.validate(h)?;
        self.memory.validate()?;
        self.workflows.validate(&self.connectors)?;
        self.providers.validate()?;
        if self.payment.decimals != 6
            || !(1..=86_400).contains(&self.payment.reorg_margin_seconds)
            || self.payment.chain_id == 0
            || self.payment.name.is_empty()
            || self.payment.name.len() > 80
            || self.payment.version.is_empty()
            || self.payment.version.len() > 20
        {
            bail!("invalid payment network configuration");
        }
        self.payment
            .settlement_finality
            .validate_rpc(&self.rpc_url)?;
        crate::features::identity::wallet(&self.payment.asset)?;
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn unfinalized_settlement_is_local_only() -> anyhow::Result<()> {
        super::SettlementFinality::Latest.validate_rpc("http://127.0.0.1:8545")?;
        assert!(
            super::SettlementFinality::Latest
                .validate_rpc("https://sepolia.base.org")
                .is_err()
        );
        super::SettlementFinality::Finalized.validate_rpc("https://sepolia.base.org")?;
        Ok(())
    }
    #[test]
    fn unsafe_operational_overrides_fail_closed() -> anyhow::Result<()> {
        let valid: super::RuntimeConfig = toml::from_str(include_str!("../config.toml"))?;
        valid.validate()?;
        let mut config = valid.clone();
        config.http.concurrency = 0;
        assert!(config.validate().is_err());
        let mut config = valid.clone();
        config.worker.stale_after_seconds = 1;
        assert!(config.validate().is_err());
        let mut config = valid;
        config.payment.asset = "http://evil.example".into();
        assert!(config.validate().is_err());
        Ok(())
    }
}
