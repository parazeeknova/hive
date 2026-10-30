//! Profiles are persistent, owned, and independent of sessions; balances come from RPC.
use super::{Result, attack};
use axum::{Json, Router, http::StatusCode, routing::post};
use hiveway::{features::common::settings::Settings, state::AppState};
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub(super) async fn exercise(app: Router, state: &AppState, tokens: (&str, &str)) -> Result {
    let a = format!("Bearer {}", tokens.0);
    let b = format!("Bearer {}", tokens.1);
    profiles(app.clone(), &a, &b).await?;
    shared_profile(app.clone(), state, tokens.0, &a).await?;
    balances(state, &a).await
}
async fn profiles(app: Router, a: &str, b: &str) -> Result {
    let get = |app| attack(app, "GET", "/v1/account", String::new(), &[]);
    let (status, _) = get(app.clone()).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, initial) = attack(
        app.clone(),
        "GET",
        "/v1/account",
        String::new(),
        &[("authorization", a)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{initial}");
    let name = format!("user-{}", &Uuid::new_v4().simple().to_string()[..20]);
    let profile = json!({"name":"Saved profile","username":name,"avatar":""});
    let (status, saved) = attack(
        app.clone(),
        "POST",
        "/v1/account",
        profile.to_string(),
        &[("authorization", a)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let (status, persisted) = attack(
        app.clone(),
        "GET",
        "/v1/account",
        String::new(),
        &[("authorization", a)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(persisted, profile);
    let (status, other) = attack(
        app.clone(),
        "GET",
        "/v1/account",
        String::new(),
        &[("authorization", b)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_ne!(other["username"], profile["username"]);
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/account",
        profile.to_string(),
        &[("authorization", b)],
    )
    .await?;
    assert_eq!(status, StatusCode::CONFLICT);
    for invalid in [
        json!({"name":"","username":"valid","avatar":""}),
        json!({"name":"Name","username":"valid","avatar":"data:image/svg+xml;base64,PHN2Zz4="}),
        json!({"name":"Name","username":"valid","avatar":"","owner":"someone-else"}),
    ] {
        let (status, _) = attack(
            app.clone(),
            "POST",
            "/v1/account",
            invalid.to_string(),
            &[("authorization", a)],
        )
        .await?;
        assert!(status.is_client_error());
    }
    let (_, unchanged) = attack(
        app,
        "GET",
        "/v1/account",
        String::new(),
        &[("authorization", b)],
    )
    .await?;
    assert_eq!(
        unchanged, other,
        "another identity cannot overwrite the profile"
    );
    Ok(())
}
async fn balances(state: &AppState, bearer: &str) -> Result {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let mut settings = Settings::from_env()?;
    settings.rpc_url = format!("http://{}", listener.local_addr()?);
    let requests = Arc::new(Mutex::new(Vec::<Value>::new()));
    let capture = requests.clone();
    let chain_id = settings.runtime.payment.chain_id;
    let served_chain = Arc::new(std::sync::atomic::AtomicU64::new(chain_id));
    let fixture_chain = served_chain.clone();
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route(
                "/",
                post(move |Json(input): Json<Value>| {
                    let capture = capture.clone();
                    let fixture_chain = fixture_chain.clone();
                    async move {
                        let result = match input["method"].as_str() {
                            Some("eth_chainId") => json!(format!(
                                "0x{:x}",
                                fixture_chain.load(std::sync::atomic::Ordering::Relaxed)
                            )),
                            Some("eth_blockNumber") => json!("0xabc"),
                            Some("eth_call") => json!(format!("0x{:064x}", 12_345_678)),
                            Some("eth_getBalance") => json!("0x0"),
                            _ => Value::Null,
                        };
                        if let Ok(mut requests) = capture.lock() {
                            requests.push(input);
                        }
                        Json(json!({"jsonrpc":"2.0","id":1,"result":result}))
                    }
                }),
            ),
        )
        .await
    });
    let fixture = AppState {
        settings: Arc::new(settings),
        ..state.clone()
    };
    let (app, _drain) =
        hiveway::api::protect(hiveway::api::router().with_state(fixture.clone()), fixture);
    let (status, balance) = attack(
        app.clone(),
        "GET",
        "/v1/account/balance",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    served_chain.store(1, std::sync::atomic::Ordering::Relaxed);
    let (wrong, body) = attack(
        app,
        "GET",
        "/v1/account/balance",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(wrong, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["error"], "wrong_payment_network");
    server.abort();
    let _ = server.await;
    assert_eq!(status, StatusCode::OK, "{balance}");
    assert_eq!(balance["amount"], "12345678");
    assert_eq!(balance["native_amount"], "0");
    assert_eq!(balance["block"], "0xabc");
    let requests = requests.lock().map_err(|_| "poisoned fixture")?;
    for call in requests
        .iter()
        .filter(|call| matches!(call["method"].as_str(), Some("eth_call" | "eth_getBalance")))
    {
        assert_eq!(
            call["params"][1], "0xabc",
            "reads must use the same pinned block"
        );
    }
    Ok(())
}

async fn shared_profile(app: Router, state: &AppState, original: &str, bearer: &str) -> Result {
    use hiveway::{
        entities::agent_credentials,
        features::{common::model::now, identity},
    };
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
    let row = agent_credentials::Entity::find()
        .filter(agent_credentials::Column::TokenHash.eq(identity::hash(original.as_bytes())))
        .one(&state.postgres)
        .await?
        .ok_or("missing credential")?;
    let token = format!("hwa_{}", hex::encode(rand::random::<[u8; 32]>()));
    let other = format!("Bearer {token}");
    let mut model: agent_credentials::ActiveModel = row.into();
    model.id = Set(Uuid::new_v4());
    model.token_hash = Set(identity::hash(token.as_bytes()));
    model.created_at = Set(now());
    model.insert(&state.postgres).await?;
    let (_, expected) = attack(
        app.clone(),
        "GET",
        "/v1/account",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    let (status, actual) = attack(
        app,
        "GET",
        "/v1/account",
        String::new(),
        &[("authorization", &other)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        actual, expected,
        "profile survives signing in with another session"
    );
    Ok(())
}
