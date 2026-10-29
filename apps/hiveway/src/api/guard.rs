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
        .is_ok_and(|mut peers| admit(&mut peers, peer.ip(), Instant::now(), limits));
    if !allowed {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(_permit) = admission.requests.try_acquire() else {
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
    let mut response = match tokio::time::timeout(
        Duration::from_secs(limits.request_timeout_seconds),
        next.run(request),
    )
    .await
    {
        Ok(response) => response,
        Err(_) => StatusCode::GATEWAY_TIMEOUT.into_response(),
    };
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    response
}
#[cfg(test)]
mod tests {
    use super::admit;
    use std::{
        collections::HashMap,
        net::{IpAddr, Ipv4Addr},
        time::{Duration, Instant},
    };
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
