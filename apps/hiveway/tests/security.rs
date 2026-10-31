#![cfg(not(miri))]
#[path = "security/account.rs"]
mod account;
#[path = "security/activity.rs"]
mod activity;
#[path = "security/api_keys.rs"]
mod api_keys;
#[path = "security/artifacts.rs"]
mod artifacts;
#[path = "security/builder.rs"]
mod builder;
#[path = "security/catalog.rs"]
mod catalog;
#[path = "security/dodo.rs"]
mod dodo;
#[path = "security/free_publishing.rs"]
mod free_publishing;
#[path = "security/recovery.rs"]
mod recovery;
#[path = "security/refunds.rs"]
mod refunds;
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{HeaderValue, Request, StatusCode},
};
use hiveway::entities::agent_credentials;
use hiveway::{
    features::{common::settings::Settings, identity},
    state::AppState,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, Database, EntityTrait, PaginatorTrait,
    QueryFilter, Set,
};
use serde_json::{Value, json};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;
use uuid::Uuid;
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
async fn attack(
    app: Router,
    method: &str,
    path: &str,
    body: String,
    headers: &[(&str, &str)],
) -> Result<(StatusCode, Value)> {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("host", "127.0.0.1:3001")
        .body(Body::from(body))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("192.0.2.9:4000".parse::<SocketAddr>()?));
    for (name, value) in headers {
        request.headers_mut().append(
            (*name).parse::<axum::http::HeaderName>()?,
            HeaderValue::from_str(value)?,
        );
    }
    let response = app.oneshot(request).await?;
    let status = response.status();
    let bytes = to_bytes(response.into_body(), 1_000_000).await?;
    Ok((
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    ))
}
async fn probe(app: Router, id: Uuid, token_a: &str, token_b: &str) -> Result {
    let bearer_a = format!("Bearer {token_a}");
    let bearer_b = format!("Bearer {token_b}");
    let uri = format!("/v1/invocations/{id}");
    let (status, body) = attack(
        app.clone(),
        "GET",
        &uri,
        String::new(),
        &[("authorization", &bearer_a)],
    )
    .await?;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert!(body["result"].is_null());
    let (status, _) = attack(
        app.clone(),
        "GET",
        &uri,
        String::new(),
        &[("authorization", &bearer_b)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "another valid agent must not read an invocation"
    );
    let (status, _) = attack(
        app.clone(),
        "POST",
        &format!("{uri}/reconcile"),
        "{}".into(),
        &[("authorization", &bearer_b)],
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _) = attack(
        app.clone(),
        "GET",
        &uri,
        String::new(),
        &[("authorization", &bearer_a), ("authorization", &bearer_b)],
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    probe_inputs(app, &uri, &bearer_a).await
}
async fn probe_inputs(app: Router, uri: &str, bearer_a: &str) -> Result {
    let (status, _) = attack(
        app.clone(),
        "GET",
        uri,
        String::new(),
        &[
            ("authorization", bearer_a),
            ("origin", "https://evil.example"),
        ],
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = attack(
        app.clone(),
        "GET",
        uri,
        String::new(),
        &[("authorization", bearer_a), ("host", "evil.example")],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "duplicate/forged hosts must not bypass admission"
    );
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &[
            ("authorization", bearer_a),
            ("origin", "http://127.0.0.1:3000"),
            ("origin", "https://evil.example"),
        ],
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/auth/verify",
        json!({"challenge_id":Uuid::new_v4(),"signature":"f".repeat(140_000)}).to_string(),
        &[],
    )
    .await?;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/webhooks/dodo",
        json!({"type":"payment.succeeded","data":{"payment_id":"pay_forged"}}).to_string(),
        &[
            ("webhook-id", "msg_forged"),
            ("webhook-timestamp", "1"),
            ("webhook-signature", "v1,forged"),
        ],
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, body) = attack(
        app.clone(),
        "GET",
        "/v1/capabilities?q=%27%20OR%201%3D1%20--",
        String::new(),
        &[("authorization", bearer_a)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!([]),
        "SQL fragments must remain literal search data"
    );
    let (status,_)=attack(app.clone(),"POST","/v1/capabilities",json!({"name":"attack","description":"mass assignment","runtime":"repository_stack","price":"1","creator_wallet":"0x2222222222222222222222222222222222222222"}).to_string(),&[("authorization",bearer_a)]).await?;
    assert!(
        status.is_client_error(),
        "a publisher cannot assign another creator wallet"
    );
    probe_quotes(app.clone(), bearer_a).await?;
    let (status, _) = attack(
        app,
        "POST",
        "/v1/quotes",
        "{}".into(),
        &[("authorization", "Bearer ' OR 1=1 --")],
    )
    .await?;
    assert!(status.is_client_error());
    Ok(())
}
async fn probe_quotes(app: Router, bearer: &str) -> Result {
    // Scout finds the owned fixture capability without embedding database IDs in the API.
    let (status, caps) = attack(
        app.clone(),
        "GET",
        "/v1/capabilities",
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let cap = caps
        .as_array()
        .and_then(|values| values.first())
        .ok_or("missing capability")?;
    let key = Uuid::new_v4().to_string();
    let body=json!({"capability_id":cap["id"],"input":{"repository_url":"https://github.com/a/b","revision":"1".repeat(40)}}).to_string();
    let headers = [
        ("authorization", bearer),
        ("idempotency-key", key.as_str()),
        ("x-hive-model", "fixture-model"),
        ("x-hive-harness", "fixture-harness"),
    ];
    let (status, first) = attack(app.clone(), "POST", "/v1/quotes", body.clone(), &headers).await?;
    assert_eq!(
        status,
        StatusCode::CREATED,
        "native quote reservation failed: {first}"
    );
    let (status, retry) = attack(app.clone(), "POST", "/v1/quotes", body, &headers).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(first["id"], retry["id"]);
    let altered=json!({"capability_id":cap["id"],"input":{"repository_url":"https://github.com/a/b","revision":"2".repeat(40)}}).to_string();
    let (status, _) = attack(app, "POST", "/v1/quotes", altered, &headers).await?;
    assert_eq!(status, StatusCode::CONFLICT);
    Ok(())
}
async fn live_socket_probe(app: Router) -> Result {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
    });
    let result=async {
        let response=reqwest::Client::new().get(format!("http://{address}/v1/credentials/me")).header("host","evil.example").send().await?;
        assert_eq!(response.status(),StatusCode::FORBIDDEN);
        let mut slow=tokio::net::TcpStream::connect(address).await?;
        slow.write_all(b"POST /v1/auth/verify HTTP/1.1\r\nHost: 127.0.0.1:3001\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n").await?;
        let mut bytes=[0;1024];
        let count=tokio::time::timeout(std::time::Duration::from_secs(8),slow.read(&mut bytes)).await??;
        assert!(String::from_utf8_lossy(&bytes[..count]).starts_with("HTTP/1.1 408"),"a slow body must time out before execution admission is held indefinitely");
        Ok::<(),Box<dyn std::error::Error>>(())
    }.await;
    server.abort();
    let _ = server.await;
    result
}
#[tokio::test]
#[ignore = "requires HIVE_TEST_DATABASE_URL pointing at local PostgreSQL"]
#[allow(clippy::too_many_lines)]
async fn adversarial_http_access_and_input_boundaries() -> Result {
    let url = std::env::var("HIVE_TEST_DATABASE_URL")?;
    let admin = Database::connect(url.clone()).await?;
    let schema = format!("hive_test_{}", Uuid::new_v4().simple());
    admin
        .execute_unprepared(&format!("CREATE SCHEMA {schema}"))
        .await?;
    let mut options = sea_orm::ConnectOptions::new(url);
    options
        .set_schema_search_path(schema.clone())
        .sqlx_logging(false);
    let db = Database::connect(options).await?;
    hiveway::database::migrate(&db).await?;
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let cap = Uuid::new_v4();
    let id = Uuid::new_v4();
    let token_a = format!("hwa_{}", "a".repeat(64));
    let token_b = format!("hwa_{}", "b".repeat(64));
    seed(&db, a, b, cap, id, &token_a, &token_b).await?;
    let mut settings = Settings::from_env()?;
    // This suite exercises many independent authorization scenarios on one test peer.
    // Rate-limit behavior has dedicated guard tests.
    settings.runtime.http.requests_per_minute = 1000;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    settings.facilitator_url = format!("http://{}", listener.local_addr()?);
    settings.builder = Some(hiveway::features::builder::config::Config {
        url: format!("{}/chat/completions", settings.facilitator_url),
        api_key: "fixture".into(),
        model: "test-wasm-generator".into(),
        reasoning_effort: None,
        owner_daily: 3,
        global_daily: 20,
    });
    settings.dodo_api_url = settings.facilitator_url.clone();
    settings.dodo_api_key = Some("fixture".into());
    settings.dodo_product_id = Some("pdt_fixture".into());
    let provider_status = Arc::new(std::sync::atomic::AtomicU16::new(400));
    let fixture_status = provider_status.clone();
    let chain = Arc::new(recovery::ChainFixture::default());
    let fixture_chain = chain.clone();
    let provider = Arc::new(dodo::ProviderFixture::default());
    let refund_fixture = Arc::new(refunds::Fixture::new(provider.clone()));
    let server_refunds = refund_fixture.clone();
    let payment_provider = provider.clone();
    let session_provider = provider.clone();
    settings.rpc_url = settings.facilitator_url.clone();
    let facilitator = tokio::spawn(async move {
        let router = Router::new().route("/chat/completions", axum::routing::post(builder::provider)).merge(refunds::router(server_refunds))
            .route("/payments/{id}", axum::routing::get(dodo::payment).with_state(payment_provider))
            .route("/checkouts/{id}", axum::routing::get(dodo::session).with_state(session_provider))
            .route("/", axum::routing::post(recovery::rpc).with_state(fixture_chain))
            .route("/supported", axum::routing::get(|| async {
                axum::Json(json!({"kinds":[{"x402Version":2,"scheme":"exact","network":"eip155:84532"}]}))
            }))
            .route("/checkouts", axum::routing::post(move || {
                let status = fixture_status.load(std::sync::atomic::Ordering::Relaxed);
                async move { StatusCode::from_u16(status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR) }
            }));
        axum::serve(listener, router).await
    });
    settings.dodo_webhook_secret = Some("AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=".into());
    let state = AppState {
        sandbox: Arc::new(hiveway::features::cell::runtime::Sandbox::new(
            hiveway::runtime::RuntimeConfig::load()?.sandbox,
        )),
        postgres: db.clone(),
        settings: Arc::new(settings),
        redis: fred::prelude::Builder::default_centralized().build()?,
        http: reqwest::Client::new(),
    };
    let (app, _drain) = hiveway::api::protect(
        hiveway::api::router().with_state(state.clone()),
        state.clone(),
    );
    let outcome = probe(app.clone(), id, &token_a, &token_b).await;
    let payment_result =
        probe_checkout_rejections(app.clone(), &db, a, id, &token_a, &provider_status).await;
    let recovery_result = async {
        recovery::checkout_queue(&state, id, a).await?;
        dodo::exercise(app.clone(), &state, &provider, id, (&token_a, &token_b)).await?;
        refunds::exercise(
            app.clone(),
            &state,
            &refund_fixture,
            id,
            (&token_a, &token_b),
        )
        .await?;
        recovery::expiry(&state, &chain, a, cap).await?;
        recovery::settlement_finality(&state, &chain, a, cap).await
    }
    .await;
    let artifacts_result = artifacts::exercise(app.clone(), &state, (&token_a, &token_b)).await;
    let catalog_result = catalog::exercise(app.clone(), &state, (&token_a, &token_b)).await;
    let account_result = account::exercise(app.clone(), &state, (&token_a, &token_b)).await;
    let session_result = session_lifecycle(app.clone(), &state).await;
    let socket_result = live_socket_probe(app).await;
    facilitator.abort();
    let _ = facilitator.await;
    let inbox = hiveway::entities::payment_webhook_inbox::Entity::find()
        .count(&db)
        .await?;
    assert_eq!(inbox, 0, "forged payments must never enter the inbox");
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome?;
    payment_result?;
    recovery_result?;
    artifacts_result?;
    catalog_result?;
    session_result?;
    account_result?;
    socket_result
}

async fn seed(
    db: &sea_orm::DatabaseConnection,
    a: Uuid,
    b: Uuid,
    cap: Uuid,
    id: Uuid,
    token_a: &str,
    token_b: &str,
) -> Result {
    for (credential, token, wallet) in [
        (a, &token_a, "0x1111111111111111111111111111111111111111"),
        (b, &token_b, "0x2222222222222222222222222222222222222222"),
    ] {
        agent_credentials::ActiveModel {
            id: Set(credential),
            wallet: Set(wallet.into()),
            name: Set("fixture".into()),
            token_hash: Set(identity::hash(token.as_bytes())),
            scopes: Set(vec![
                if credential == a {
                    "capabilities:invoke"
                } else {
                    "repository:analyze"
                }
                .into(),
                "receipts:read".into(),
                "capabilities:read".into(),
                "capabilities:publish".into(),
                "sales:read".into(),
            ]),
            per_call_limit: Set(100),
            total_limit: Set(100),
            reserved: Set(i64::from(credential == a)),
            expires_at: Set(i64::MAX),
            created_at: Set(0),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    hiveway::entities::capabilities::ActiveModel {
        id: Set(cap),
        creator_wallet: Set("0x1111111111111111111111111111111111111111".into()),
        name: Set("fixture".into()),
        description: Set("security fixture".into()),
        version: Set("1".into()),
        runtime: Set("repository_stack".into()),
        price: Set(1),
        created_at: Set(0),
        ..Default::default()
    }
    .insert(db)
    .await?;
    hiveway::entities::invocations::ActiveModel {
        id: Set(id),
        credential_id: Set(a),
        capability_id: Set(cap),
        idempotency_key: Set(Uuid::new_v4()),
        request_hash: Set(vec![0; 32]),
        input: Set(json!({})),
        requirements: Set(json!({})),
        resource: Set(json!({})),
        state: Set("quoted".into()),
        price: Set(1),
        expires_at: Set(0),
        created_at: Set(0),
        updated_at: Set(0),
        creator_wallet: Set("0x1111111111111111111111111111111111111111".into()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    Ok(())
}

async fn probe_checkout_rejections(
    app: Router,
    db: &sea_orm::DatabaseConnection,
    credential: Uuid,
    id: Uuid,
    token: &str,
    provider_status: &std::sync::atomic::AtomicU16,
) -> Result {
    use hiveway::entities::invocations;
    use sea_orm::sea_query::{Expr, ExprTrait};
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::PerCallLimit,
            Expr::val(100_000_i64),
        )
        .col_expr(
            agent_credentials::Column::TotalLimit,
            Expr::val(1_000_000_i64),
        )
        .filter(agent_credentials::Column::Id.eq(credential))
        .exec(db)
        .await?;
    let bearer = format!("Bearer {token}");
    for (status, expected) in [(400, "failed"), (500, "checkout_creating")] {
        let before = agent_credentials::Entity::find_by_id(credential)
            .one(db)
            .await?
            .ok_or("missing credential")?
            .reserved;
        agent_credentials::Entity::update_many()
            .col_expr(
                agent_credentials::Column::Reserved,
                Expr::col(agent_credentials::Column::Reserved).add(10_000_i64),
            )
            .filter(agent_credentials::Column::Id.eq(credential))
            .exec(db)
            .await?;
        invocations::Entity::update_many()
            .col_expr(invocations::Column::State, Expr::val("quoted"))
            .col_expr(invocations::Column::Price, Expr::val(10_000_i64))
            .col_expr(
                invocations::Column::PaymentMethod,
                Expr::val(Option::<String>::None),
            )
            .col_expr(
                invocations::Column::ExpiresAt,
                Expr::val(time::OffsetDateTime::now_utc().unix_timestamp() + 300),
            )
            .filter(invocations::Column::Id.eq(id))
            .exec(db)
            .await?;
        provider_status.store(status, std::sync::atomic::Ordering::Relaxed);
        let (response, _) = attack(
            app.clone(),
            "POST",
            &format!("/v1/invocations/{id}/checkout"),
            String::new(),
            &[("authorization", &bearer)],
        )
        .await?;
        assert_eq!(
            response,
            if status == 400 {
                StatusCode::BAD_REQUEST
            } else {
                StatusCode::SERVICE_UNAVAILABLE
            }
        );
        let inv = invocations::Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or("missing invocation")?;
        assert_eq!(inv.state, expected);
        let after = agent_credentials::Entity::find_by_id(credential)
            .one(db)
            .await?
            .ok_or("missing credential")?
            .reserved;
        assert_eq!(after, before + if status == 400 { 0 } else { 10_000 });
        if status == 400 {
            assert_eq!(inv.error_code.as_deref(), Some("dodo_request_rejected"));
            // Retrying a rejected invocation must not release the same reservation twice.
            let _ = attack(
                app.clone(),
                "POST",
                &format!("/v1/invocations/{id}/checkout"),
                String::new(),
                &[("authorization", &bearer)],
            )
            .await?;
            assert_eq!(
                agent_credentials::Entity::find_by_id(credential)
                    .one(db)
                    .await?
                    .ok_or("missing credential")?
                    .reserved,
                before
            );
        }
    }
    Ok(())
}

async fn session_lifecycle(app: Router, state: &AppState) -> Result {
    let token = format!("hwa_{}", "d".repeat(64));
    let bearer = format!("Bearer {token}");
    let id = Uuid::new_v4();
    agent_credentials::ActiveModel {
        id: Set(id),
        wallet: Set("github:123".into()),
        name: Set("github-user".into()),
        token_hash: Set(identity::hash(token.as_bytes())),
        scopes: Set(vec![
            "capabilities:read".into(),
            "receipts:read".into(),
            "capabilities:invoke".into(),
        ]),
        per_call_limit: Set(100),
        total_limit: Set(100),
        expires_at: Set(hiveway::features::common::model::now() + 3600),
        created_at: Set(0),
        ..Default::default()
    }
    .insert(&state.postgres)
    .await?;
    let headers = [("authorization", bearer.as_str())];
    free_agent_credential(app.clone(), state, &headers).await?;
    let (status, me) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["wallet"], "github:123");
    let (status, balance) = attack(
        app.clone(),
        "GET",
        "/v1/account/balance",
        String::new(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert!(balance["amount"].is_null());
    assert!(
        balance["wallet"].is_null(),
        "GitHub sign-in must not invent a wallet"
    );
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/auth/logout",
        "{}".into(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        &headers,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::UNAUTHORIZED,
        "revoked sessions must fail database verification"
    );
    Ok(())
}

async fn free_agent_credential(app: Router, state: &AppState, headers: &[(&str, &str)]) -> Result {
    let (status, issued) = attack(
        app.clone(),
        "POST",
        "/v1/account/credentials",
        json!({"name":"Test agent"}).to_string(),
        headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{issued}");
    let token = issued["token"].as_str().ok_or("missing credential")?;
    let bearer = format!("Bearer {token}");
    let (status, me) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        &[("authorization", &bearer)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(me["wallet"], "github:123");
    assert_eq!(
        me["scopes"],
        json!([
            "capabilities:read",
            "receipts:read",
            "capabilities:invoke",
            "capabilities:publish",
            "workflows:publish"
        ])
    );
    free_publishing::exercise(app.clone(), &bearer).await?;
    Box::pin(builder::exercise(app.clone(), state, &bearer)).await?;
    free_agent_invocation(app.clone(), state, &bearer).await?;
    let (status, _) = attack(
        app.clone(),
        "GET",
        "/v1/sales",
        String::new(),
        &[("authorization", &bearer)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "free credentials must not gain creator permissions"
    );
    Box::pin(activity::exercise(app.clone(), state, headers, &bearer)).await?;
    api_keys::exercise(app, state, headers, &issued, &bearer).await
}

async fn free_agent_invocation(app: Router, state: &AppState, bearer: &str) -> Result {
    use hiveway::entities::capabilities;
    let cap = capabilities::Entity::find()
        .filter(capabilities::Column::Name.eq("Dynamic echo"))
        .one(&state.postgres)
        .await?
        .ok_or("missing echo")?;
    let paid_id = cap.id;
    let mut free: capabilities::ActiveModel = cap.into();
    let id = Uuid::new_v4();
    free.id = Set(id);
    free.price = Set(0);
    free.insert(&state.postgres).await?;
    let key = Uuid::new_v4().to_string();
    let headers = [
        ("authorization", bearer),
        ("idempotency-key", key.as_str()),
        ("x-hive-model", "fixture-model"),
        ("x-hive-harness", "fixture-harness"),
    ];
    let input = json!({"capability_id":id,"input":{"value":"free with GitHub"}});
    let (status, quote) = attack(
        app.clone(),
        "POST",
        "/v1/quotes",
        input.to_string(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED, "{quote}");
    let invocation = quote["id"].as_str().ok_or("missing invocation")?;
    let (status, receipt) = attack(
        app.clone(),
        "POST",
        &format!("/v1/invocations/{invocation}"),
        "{}".into(),
        &headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{receipt}");
    assert_eq!(receipt["result"], input["input"]);
    let paid = json!({"capability_id":paid_id,"input":{"value":"cannot pay"}});
    let (status, _) = attack(
        app,
        "POST",
        "/v1/quotes",
        paid.to_string(),
        &[
            ("authorization", bearer),
            ("idempotency-key", &Uuid::new_v4().to_string()),
        ],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::FORBIDDEN,
        "free credentials cannot create paid quotes"
    );
    Ok(())
}
