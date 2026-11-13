//! Admission control. Only the edge-verified `CF-Connecting-IP` identifies a
//! client address; generic proxy headers never do.
//!
//! Rate limits are fixed one-minute windows stored in Redis so every replica
//! shares them. Bearer traffic is limited per credential (plus a generous
//! per-address ceiling, so random tokens cannot mint fresh buckets), and
//! anonymous traffic per address. If Redis is slow or down, the same windows
//! fall back to bounded process-local state instead of failing open.
use crate::state::AppState;
use axum::{
    body::{Body, to_bytes},
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use fred::prelude::{ClientLike, KeysInterface};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    hash::Hash,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
type Windows = HashMap<String, (Instant, u32)>;
#[derive(Clone)]
pub(super) struct Admission {
    pub state: AppState,
    pub requests: Arc<Semaphore>,
    /// Requests briefly waiting for an active slot during a burst.
    pub queue: Arc<Semaphore>,
    pub idle: Arc<Semaphore>,
    pub windows: Arc<Mutex<Windows>>,
}
/// A request's admission permit. Handlers that park in a long-poll call
/// [`AdmissionSlot::idle`] so waiting does not consume active request capacity.
#[derive(Clone, Default)]
pub struct AdmissionSlot {
    active: Arc<Mutex<Option<OwnedSemaphorePermit>>>,
    parked: Arc<Mutex<Option<OwnedSemaphorePermit>>>,
    idle: Option<Arc<Semaphore>>,
}
impl AdmissionSlot {
    /// Move from the active pool to the idle-waiter pool. Without idle capacity
    /// the request simply keeps its active permit.
    pub fn idle(&self) {
        let Some(idle) = &self.idle else { return };
        let Ok(mut active) = self.active.lock() else {
            return;
        };
        if active.is_none() {
            return;
        }
        if let Ok(permit) = idle.clone().try_acquire_owned()
            && let Ok(mut parked) = self.parked.lock()
        {
            *parked = Some(permit);
            *active = None;
        }
    }
    fn release(&self) {
        if let Ok(mut active) = self.active.lock() {
            active.take();
        }
        if let Ok(mut parked) = self.parked.lock() {
            parked.take();
        }
    }
}
/// Tracks handler lifetimes, including tasks whose HTTP response has timed out.
pub struct HandlerDrain {
    requests: Arc<Semaphore>,
    concurrency: usize,
    idle: Arc<Semaphore>,
    idle_capacity: usize,
}
impl HandlerDrain {
    pub(super) fn new(
        requests: Arc<Semaphore>,
        concurrency: usize,
        idle: Arc<Semaphore>,
        idle_capacity: usize,
    ) -> Self {
        Self {
            requests,
            concurrency,
            idle,
            idle_capacity,
        }
    }
    /// Wait for all handlers and hold admission closed through infrastructure cleanup.
    /// # Errors
    /// Returns an error if draining exceeds the deadline or admission is closed.
    pub async fn wait(
        self,
        timeout: Duration,
    ) -> anyhow::Result<(OwnedSemaphorePermit, OwnedSemaphorePermit)> {
        let count = u32::try_from(self.concurrency)?;
        let idle = u32::try_from(self.idle_capacity)?;
        Ok(tokio::time::timeout(timeout, async {
            let active = self.requests.acquire_many_owned(count).await?;
            let parked = self.idle.acquire_many_owned(idle).await?;
            Ok::<_, tokio::sync::AcquireError>((active, parked))
        })
        .await??)
    }
}
fn peer_key(ip: IpAddr) -> IpAddr {
    match ip {
        IpAddr::V4(_) => ip,
        IpAddr::V6(ip) => ip.to_ipv4_mapped().map_or_else(
            || {
                let mut bytes = ip.octets();
                bytes[8..].fill(0);
                IpAddr::V6(bytes.into())
            },
            IpAddr::V4,
        ),
    }
}
/// Resolve the accountable client address.
///
/// All public traffic arrives through Cloudflare, so the edge-verified
/// `CF-Connecting-IP` identifies the real client. Direct-to-origin traffic
/// (health checks, loopback, operator tooling) falls back to the TCP peer.
fn client_ip(request: &Request, peer: std::net::SocketAddr) -> IpAddr {
    request
        .headers()
        .get("cf-connecting-ip")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(peer.ip())
}
/// Process-local fixed window. Bounded: unknown keys are refused once full.
fn admit<K: Hash + Eq>(
    windows: &mut HashMap<K, (Instant, u32)>,
    key: K,
    now: Instant,
    limit: u32,
    tracked: usize,
) -> bool {
    windows.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
    if windows.len() >= tracked && !windows.contains_key(&key) {
        return false;
    }
    let (_, count) = windows.entry(key).or_insert((now, 0));
    if *count >= limit {
        return false;
    }
    *count += 1;
    true
}
/// Well-formed Hive bearer credentials, hashed so raw tokens never reach Redis.
fn credential_bucket(request: &Request) -> Option<String> {
    let value = request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?;
    let token = value.strip_prefix("Bearer ")?;
    if token.len() != 68
        || !token.starts_with("hwa_")
        || !token[4..].bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    Some(format!(
        "c:{}",
        hex::encode(&Sha256::digest(token.as_bytes())[..16])
    ))
}
/// Unauthenticated OAuth discovery and token endpoints must work for browser-based
/// MCP clients on any origin. They never read cookies or ambient credentials.
pub(super) fn public_oauth_path(path: &str) -> bool {
    path.starts_with("/.well-known/oauth-protected-resource")
        || path == "/.well-known/oauth-authorization-server"
        || matches!(path, "/oauth/token" | "/oauth/register" | "/oauth/revoke")
}
struct Limiter<'a> {
    admission: &'a Admission,
}
impl Limiter<'_> {
    async fn allow(&self, bucket: &str, limit: u32) -> bool {
        let state = &self.admission.state;
        let limits = &state.settings.runtime.http;
        let window = crate::features::common::model::now() / 60;
        if state.redis.is_connected() {
            let key = format!("hive:rl:{bucket}:{window}");
            let counted = tokio::time::timeout(
                Duration::from_millis(limits.rate_limit_redis_timeout_ms),
                async {
                    let count: i64 = state.redis.incr(&key).await?;
                    if count == 1 {
                        let (): () = state.redis.expire(&key, 70, None).await?;
                    }
                    Ok::<_, fred::error::Error>(count)
                },
            )
            .await;
            if let Ok(Ok(count)) = counted {
                return count <= i64::from(limit);
            }
        }
        self.admission.windows.lock().is_ok_and(|mut windows| {
            admit(
                &mut windows,
                bucket.to_owned(),
                Instant::now(),
                limit,
                limits.tracked_peers,
            )
        })
    }
}
fn too_many(limit: u32) -> Response {
    let mut response = StatusCode::TOO_MANY_REQUESTS.into_response();
    let reset = 60 - crate::features::common::model::now().rem_euclid(60);
    if let Ok(value) = HeaderValue::from_str(&reset.to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    if let Ok(value) = HeaderValue::from_str(&limit.to_string()) {
        response.headers_mut().insert("ratelimit-limit", value);
    }
    response
}
/// Take an active slot, waiting briefly in a bounded queue during bursts so
/// short spikes are absorbed instead of shed. A full queue or an expired wait
/// sheds the request.
async fn admit_active(
    admission: &Admission,
    limits: &crate::runtime::HttpLimits,
) -> Option<OwnedSemaphorePermit> {
    if let Ok(permit) = admission.requests.clone().try_acquire_owned() {
        return Some(permit);
    }
    let _queued = admission.queue.clone().try_acquire_owned().ok()?;
    tokio::time::timeout(
        Duration::from_millis(limits.queue_wait_ms),
        admission.requests.clone().acquire_owned(),
    )
    .await
    .ok()?
    .ok()
}
fn overloaded() -> Response {
    let mut response = (
        StatusCode::SERVICE_UNAVAILABLE,
        axum::Json(serde_json::json!({"error":"server_busy"})),
    )
        .into_response();
    response
        .headers_mut()
        .insert(header::RETRY_AFTER, HeaderValue::from_static("1"));
    response
}
pub(super) async fn guard(admission: State<Admission>, request: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4();
    let route = request
        .extensions()
        .get::<axum::extract::MatchedPath>()
        .map_or("unmatched", axum::extract::MatchedPath::as_str)
        .to_owned();
    let trace_headers: HashMap<String, String> = ["traceparent", "tracestate"]
        .into_iter()
        .filter_map(|name| {
            request
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|value| (name.to_owned(), value.to_owned()))
        })
        .collect();
    let parent = opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.extract(&trace_headers)
    });
    let span = tracing::info_span!("http.request", otel.kind="server", request_id = %request_id, method = %request.method(), route = %route, http.response.status_code=tracing::field::Empty);
    let _ = span.set_parent(parent);
    async move {
        let started = Instant::now();
        let mut response = guard_inner(admission, request, next).await;
        tracing::Span::current().record("http.response.status_code", response.status().as_u16());
        let elapsed = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        tracing::info!(
            status = response.status().as_u16(),
            duration_ms = elapsed,
            "HTTP request finished"
        );
        if let Ok(value) = HeaderValue::from_str(&request_id.to_string()) {
            response.headers_mut().insert("x-request-id", value);
        }
        response
    }
    .instrument(span)
    .await
}

async fn guard_inner(State(admission): State<Admission>, request: Request, next: Next) -> Response {
    let state = &admission.state;
    let limits = &state.settings.runtime.http;
    let public = public_oauth_path(request.uri().path());
    if request.headers().get_all("origin").iter().count() > 1
        || (!public
            && request
                .headers()
                .get("origin")
                .is_some_and(|v| v.to_str().ok() != Some(state.settings.auth_uri.as_str())))
    {
        return StatusCode::FORBIDDEN.into_response();
    }
    let expected = reqwest::Url::parse(&state.settings.public_url).ok();
    let host = request.headers().get("host").and_then(|v| v.to_str().ok());
    let valid_host = expected.as_ref().zip(host).is_some_and(|(expected, host)| {
        reqwest::Url::parse(&format!("{}://{host}", expected.scheme())).is_ok_and(|url| {
            url.host_str() == expected.host_str()
                && url.port_or_known_default() == expected.port_or_known_default()
                && url.username().is_empty()
                && url.password().is_none()
                && url.path() == "/"
                && url.query().is_none()
                && url.fragment().is_none()
        })
    });
    if request.headers().get_all("host").iter().count() != 1 || !valid_host {
        return StatusCode::FORBIDDEN.into_response();
    }
    let Some(ConnectInfo(peer)) = request
        .extensions()
        .get::<ConnectInfo<std::net::SocketAddr>>()
    else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let ip = peer_key(client_ip(&request, *peer));
    let limiter = Limiter {
        admission: &admission,
    };
    if let Some(bucket) = credential_bucket(&request) {
        if !limiter
            .allow(
                &format!("ia:{ip}"),
                limits.authenticated_ip_requests_per_minute,
            )
            .await
        {
            return too_many(limits.authenticated_ip_requests_per_minute);
        }
        if !limiter
            .allow(&bucket, limits.credential_requests_per_minute)
            .await
        {
            return too_many(limits.credential_requests_per_minute);
        }
    } else if !limiter
        .allow(&format!("a:{ip}"), limits.requests_per_minute)
        .await
    {
        return too_many(limits.requests_per_minute);
    }
    let Some(permit) = admit_active(&admission, limits).await else {
        return overloaded();
    };
    let slot = AdmissionSlot {
        active: Arc::new(Mutex::new(Some(permit))),
        parked: Arc::default(),
        idle: Some(admission.idle.clone()),
    };
    let (parts, body) = request.into_parts();
    let bytes = match tokio::time::timeout(
        Duration::from_secs(limits.body_timeout_seconds),
        to_bytes(body, limits.body_bytes),
    )
    .await
    {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
    };
    let mut request = Request::from_parts(parts, Body::from(bytes));
    request.extensions_mut().insert(slot.clone());
    let mut response = finish_handler(
        next.run(request),
        slot,
        Duration::from_secs(limits.request_timeout_seconds),
    )
    .await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}
async fn finish_handler(
    handler: impl std::future::Future<Output = Response> + Send + 'static,
    slot: AdmissionSlot,
    duration: Duration,
) -> Response {
    let task = tokio::spawn(
        async move {
            let response = handler.await;
            slot.release();
            response
        }
        .in_current_span(),
    );
    match tokio::time::timeout(duration, task).await {
        Ok(Ok(response)) => response,
        Ok(Err(error)) => {
            tracing::error!(%error, "Request handler task failed");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
        Err(_) => StatusCode::GATEWAY_TIMEOUT.into_response(),
    }
}
#[cfg(test)]
mod tests {
    use super::{admit, client_ip, peer_key};
    use std::{
        collections::HashMap,
        net::{IpAddr, Ipv4Addr, SocketAddr},
        time::{Duration, Instant},
    };
    #[test]
    fn edge_verified_ip_identifies_the_client() -> anyhow::Result<()> {
        fn req(header: Option<(&str, &str)>) -> anyhow::Result<axum::extract::Request> {
            let mut builder = axum::http::Request::builder();
            if let Some((name, value)) = header {
                builder = builder.header(name, value);
            }
            Ok(builder.body(axum::body::Body::empty())?)
        }
        let peer: SocketAddr = "203.0.113.7:54321".parse()?;
        assert_eq!(client_ip(&req(None)?, peer), peer.ip());
        assert_eq!(
            client_ip(&req(Some(("CF-Connecting-IP", "198.51.100.23")))?, peer),
            "198.51.100.23".parse::<IpAddr>()?
        );
        assert_eq!(
            client_ip(&req(Some(("CF-Connecting-IP", "not-an-ip")))?, peer),
            peer.ip()
        );
        Ok(())
    }
    #[test]
    fn credentials_get_hashed_buckets_and_malformed_tokens_do_not() -> anyhow::Result<()> {
        let token = format!("Bearer hwa_{}", "a".repeat(64));
        let request = axum::http::Request::builder()
            .header("authorization", &token)
            .body(axum::body::Body::empty())?;
        let bucket = super::credential_bucket(&request).ok_or_else(|| anyhow::anyhow!("bucket"))?;
        assert!(bucket.starts_with("c:") && !bucket.contains(&"a".repeat(64)));
        for value in [
            "Bearer hwa_short",
            "Basic abc",
            &format!("Bearer hwa_{}", "z".repeat(64)),
        ] {
            let request = axum::http::Request::builder()
                .header("authorization", value)
                .body(axum::body::Body::empty())?;
            assert!(super::credential_bucket(&request).is_none(), "{value}");
        }
        assert!(super::public_oauth_path(
            "/.well-known/oauth-protected-resource/mcp"
        ));
        assert!(super::public_oauth_path("/oauth/token"));
        assert!(!super::public_oauth_path("/mcp"));
        Ok(())
    }
    #[test]
    fn peers_share_mapped_addresses_and_ipv6_prefixes() -> anyhow::Result<()> {
        let v4: IpAddr = "192.0.2.1".parse()?;
        assert_eq!(peer_key(v4), v4);
        assert_eq!(peer_key("::ffff:192.0.2.1".parse()?), v4);
        let prefix = peer_key("2001:db8:1:2::1".parse()?);
        assert_eq!(prefix, peer_key("2001:db8:1:2:ffff::2".parse()?));
        assert_ne!(prefix, peer_key("2001:db8:1:3::1".parse()?));
        let limits = crate::runtime::RuntimeConfig::load()?.http;
        let mut peers = HashMap::new();
        assert!(admit(
            &mut peers,
            prefix,
            Instant::now(),
            1,
            limits.tracked_peers
        ));
        assert!(!admit(
            &mut peers,
            peer_key("2001:db8:1:2::3".parse()?),
            Instant::now(),
            1,
            limits.tracked_peers
        ));
        assert_eq!(peers.len(), 1);
        Ok(())
    }
    #[test]
    fn flood_is_bounded_and_window_expires() {
        let limits = crate::runtime::RuntimeConfig::load()
            .map_or_else(|error| panic!("fixture configuration: {error}"), |c| c.http);
        let mut peers = HashMap::new();
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let now = Instant::now();
        for _ in 0..limits.requests_per_minute {
            assert!(admit(
                &mut peers,
                ip,
                now,
                limits.requests_per_minute,
                limits.tracked_peers
            ));
        }
        assert!(!admit(
            &mut peers,
            ip,
            now,
            limits.requests_per_minute,
            limits.tracked_peers
        ));
        assert!(admit(
            &mut peers,
            ip,
            now + Duration::from_secs(61),
            limits.requests_per_minute,
            limits.tracked_peers
        ));
    }
    // The full 4096-peer load runs natively; Miri exercises the smaller window test.
    #[cfg(not(miri))]
    #[test]
    fn unique_peer_flood_cannot_grow_map_without_bound() {
        let limits = crate::runtime::RuntimeConfig::load()
            .map_or_else(|error| panic!("fixture configuration: {error}"), |c| c.http);
        let mut peers = HashMap::new();
        let now = Instant::now();
        for index in 0..4096 {
            assert!(admit(
                &mut peers,
                IpAddr::V4(Ipv4Addr::from(index)),
                now,
                limits.requests_per_minute,
                limits.tracked_peers
            ));
        }
        assert!(!admit(
            &mut peers,
            IpAddr::V4(Ipv4Addr::from(4096)),
            now,
            limits.requests_per_minute,
            limits.tracked_peers
        ));
        assert_eq!(peers.len(), 4096);
    }
}

#[cfg(all(test, not(miri)))]
mod timeout_tests {
    use super::*;
    fn slot(permit: OwnedSemaphorePermit, idle: Arc<Semaphore>) -> AdmissionSlot {
        AdmissionSlot {
            active: Arc::new(Mutex::new(Some(permit))),
            parked: Arc::default(),
            idle: Some(idle),
        }
    }
    #[tokio::test]
    async fn timeout_keeps_handler_and_permit_alive() -> anyhow::Result<()> {
        let semaphore = Arc::new(Semaphore::new(1));
        let idle = Arc::new(Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await?;
        let (finish, wait) = tokio::sync::oneshot::channel();
        let (done, completed) = tokio::sync::oneshot::channel();
        let response = finish_handler(
            async move {
                let _ = wait.await;
                let _ = done.send(());
                StatusCode::OK.into_response()
            },
            slot(permit, idle.clone()),
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(semaphore.available_permits(), 0);
        let drain = HandlerDrain::new(semaphore.clone(), 1, idle, 1);
        let mut draining = tokio::spawn(drain.wait(Duration::from_secs(2)));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut draining)
                .await
                .is_err()
        );
        finish
            .send(())
            .map_err(|()| anyhow::anyhow!("handler was cancelled"))?;
        tokio::time::timeout(Duration::from_secs(2), completed).await??;
        let drained = draining.await??;
        assert_eq!(semaphore.available_permits(), 0);
        drop(drained);
        assert_eq!(semaphore.available_permits(), 1);
        Ok(())
    }
    #[tokio::test]
    async fn idle_waiters_free_active_capacity_and_are_drained() -> anyhow::Result<()> {
        let active = Arc::new(Semaphore::new(1));
        let idle = Arc::new(Semaphore::new(1));
        let parked = slot(active.clone().acquire_owned().await?, idle.clone());
        parked.idle();
        assert_eq!(
            active.available_permits(),
            1,
            "waiting frees active capacity"
        );
        assert_eq!(idle.available_permits(), 0);
        // Without idle capacity a second waiter keeps its active permit.
        let busy = slot(active.clone().acquire_owned().await?, idle.clone());
        busy.idle();
        assert_eq!(active.available_permits(), 0);
        let drain = HandlerDrain::new(active.clone(), 1, idle.clone(), 1);
        let mut draining = tokio::spawn(drain.wait(Duration::from_secs(2)));
        assert!(
            tokio::time::timeout(Duration::from_millis(10), &mut draining)
                .await
                .is_err()
        );
        busy.release();
        parked.release();
        let drained = draining.await??;
        drop(drained);
        assert_eq!(idle.available_permits(), 1);
        Ok(())
    }
    #[tokio::test]
    async fn draining_has_a_bounded_deadline() -> anyhow::Result<()> {
        let semaphore = Arc::new(Semaphore::new(1));
        let _permit = semaphore.clone().acquire_owned().await?;
        assert!(
            HandlerDrain::new(semaphore, 1, Arc::new(Semaphore::new(1)), 1)
                .wait(Duration::from_millis(10))
                .await
                .is_err()
        );
        Ok(())
    }
    #[tokio::test]
    async fn task_failure_is_distinct_from_timeout() -> anyhow::Result<()> {
        let semaphore = Arc::new(Semaphore::new(1));
        let response = finish_handler(
            async { panic!("fixture panic") },
            slot(
                semaphore.clone().acquire_owned().await?,
                Arc::new(Semaphore::new(1)),
            ),
            Duration::from_secs(2),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(semaphore.available_permits(), 1);
        Ok(())
    }
}
