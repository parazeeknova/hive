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
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};
static REQUESTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(32);
static PEERS: LazyLock<Mutex<HashMap<IpAddr, (Instant, u32)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
fn admit(peers: &mut HashMap<IpAddr, (Instant, u32)>, ip: IpAddr, now: Instant) -> bool {
    peers.retain(|_, (start, _)| now.duration_since(*start) < Duration::from_secs(60));
    if peers.len() >= 4096 && !peers.contains_key(&ip) {
        return false;
    }
    let (_, count) = peers.entry(ip).or_insert((now, 0));
    if *count >= 120 {
        return false;
    }
    *count += 1;
    true
}
pub(super) async fn guard(State(state): State<AppState>, request: Request, next: Next) -> Response {
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
    let allowed = PEERS
        .lock()
        .is_ok_and(|mut peers| admit(&mut peers, peer.ip(), Instant::now()));
    if !allowed {
        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }
    let Ok(_permit) = REQUESTS.try_acquire() else {
        return StatusCode::SERVICE_UNAVAILABLE.into_response();
    };
    let (parts, body) = request.into_parts();
    let bytes = match tokio::time::timeout(Duration::from_secs(5), to_bytes(body, 131_072)).await {
        Ok(Ok(bytes)) => bytes,
        Ok(Err(_)) => return StatusCode::PAYLOAD_TOO_LARGE.into_response(),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
    };
    let request = Request::from_parts(parts, Body::from(bytes));
    let mut response = match tokio::time::timeout(Duration::from_secs(180), next.run(request)).await
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
        let mut peers = HashMap::new();
        let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
        let now = Instant::now();
        for _ in 0..120 {
            assert!(admit(&mut peers, ip, now));
        }
        assert!(!admit(&mut peers, ip, now));
        assert!(admit(&mut peers, ip, now + Duration::from_secs(61)));
    }
    // The full 4096-peer load runs natively; Miri exercises the smaller window test.
    #[cfg(not(miri))]
    #[test]
    fn unique_peer_flood_cannot_grow_map_without_bound() {
        let mut peers = HashMap::new();
        let now = Instant::now();
        for index in 0..4096 {
            assert!(admit(&mut peers, IpAddr::V4(Ipv4Addr::from(index)), now));
        }
        assert!(!admit(&mut peers, IpAddr::V4(Ipv4Addr::from(4096)), now));
        assert_eq!(peers.len(), 4096);
    }
}
