//! Bounded process-local admission control. Proxy forwarding headers are never identity.
use crate::state::AppState;
use axum::{
    body::{Body, to_bytes},
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
type PeerWindows = HashMap<IpAddr, (Instant, u32)>;
#[derive(Clone)]
pub(super) struct Admission {
    pub state: AppState,
    pub requests: Arc<tokio::sync::Semaphore>,
    pub peers: Arc<Mutex<PeerWindows>>,
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
fn admit(
    peers: &mut HashMap<IpAddr, (Instant, u32)>,
    ip: IpAddr,
    now: Instant,
    limits: &crate::runtime::HttpLimits,
) -> bool {
    peers.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
    if peers.len() >= limits.tracked_peers && !peers.contains_key(&ip) {
        return false;
    }
    let (_, count) = peers.entry(ip).or_insert((now, 0));
    if *count >= limits.requests_per_minute {
        return false;
    }
    *count += 1;
    true
}
pub(super) async fn guard(
    State(admission): State<Admission>,
    request: Request,
    next: Next,
) -> Response {
    let state = &admission.state;
    let limits = &state.settings.runtime.http;
    if request.headers().get_all("origin").iter().count() > 1
        || request
            .headers()
            .get("origin")
            .is_some_and(|v| v.to_str().ok() != Some(state.settings.auth_uri.as_str()))
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
    let allowed = admission
        .peers
        .lock()
        .is_ok_and(|mut peers| admit(&mut peers, peer_key(peer.ip()), Instant::now(), limits));
    if !allowed {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(permit) = admission.requests.clone().try_acquire_owned() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
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
    let request = Request::from_parts(parts, Body::from(bytes));
    let mut response = finish_handler(
        next.run(request),
        permit,
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
    permit: tokio::sync::OwnedSemaphorePermit,
    duration: Duration,
) -> Response {
    let task = tokio::spawn(async move {
        let _permit = permit;
        handler.await
    });
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
    use super::{admit, peer_key};
    use std::{
        collections::HashMap,
        net::{IpAddr, Ipv4Addr},
        time::{Duration, Instant},
    };
    #[test]
    fn peers_share_mapped_addresses_and_ipv6_prefixes() -> anyhow::Result<()> {
        let v4: IpAddr = "192.0.2.1".parse()?;
        assert_eq!(peer_key(v4), v4);
        assert_eq!(peer_key("::ffff:192.0.2.1".parse()?), v4);
        let prefix = peer_key("2001:db8:1:2::1".parse()?);
        assert_eq!(prefix, peer_key("2001:db8:1:2:ffff::2".parse()?));
        assert_ne!(prefix, peer_key("2001:db8:1:3::1".parse()?));
        let mut limits = crate::runtime::RuntimeConfig::load()?.http;
        limits.requests_per_minute = 1;
        let mut peers = HashMap::new();
        assert!(admit(&mut peers, prefix, Instant::now(), &limits));
        assert!(!admit(
            &mut peers,
            peer_key("2001:db8:1:2::3".parse()?),
            Instant::now(),
            &limits
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
        for _ in 0..120 {
            assert!(admit(&mut peers, ip, now, &limits));
        }
        assert!(!admit(&mut peers, ip, now, &limits));
        assert!(admit(
            &mut peers,
            ip,
            now + Duration::from_secs(61),
            &limits
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
                &limits
            ));
        }
        assert!(!admit(
            &mut peers,
            IpAddr::V4(Ipv4Addr::from(4096)),
            now,
            &limits
        ));
        assert_eq!(peers.len(), 4096);
    }
}

#[cfg(all(test, not(miri)))]
mod timeout_tests {
    use super::*;
    #[tokio::test]
    async fn timeout_keeps_handler_and_permit_alive() -> anyhow::Result<()> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let permit = semaphore.clone().acquire_owned().await?;
        let (finish, wait) = tokio::sync::oneshot::channel();
        let (done, completed) = tokio::sync::oneshot::channel();
        let response = finish_handler(
            async move {
                let _ = wait.await;
                let _ = done.send(());
                StatusCode::OK.into_response()
            },
            permit,
            Duration::from_millis(10),
        )
        .await;
        assert_eq!(response.status(), StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(semaphore.available_permits(), 0);
        finish
            .send(())
            .map_err(|()| anyhow::anyhow!("handler was cancelled"))?;
        tokio::time::timeout(Duration::from_secs(2), completed).await??;
        let _permit =
            tokio::time::timeout(Duration::from_secs(2), semaphore.acquire_owned()).await??;
        Ok(())
    }
    #[tokio::test]
    async fn task_failure_is_distinct_from_timeout() -> anyhow::Result<()> {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(1));
        let response = finish_handler(
            async { panic!("fixture panic") },
            semaphore.clone().acquire_owned().await?,
            Duration::from_secs(2),
        )
        .await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(semaphore.available_permits(), 1);
        Ok(())
    }
}
