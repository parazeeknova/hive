#![cfg(not(miri))]
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{HeaderValue, Request, StatusCode},
};
use hiveway::{
    features::{common::settings::Settings, identity},
    state::AppState,
};
use sea_orm::{ConnectionTrait, Database, DbBackend, Statement};
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
    for (credential, token, wallet) in [
        (a, &token_a, "0x1111111111111111111111111111111111111111"),
        (b, &token_b, "0x2222222222222222222222222222222222222222"),
    ] {
        db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO agent_credentials(id,wallet,name,token_hash,scopes,per_call_limit,total_limit,expires_at,created_at) VALUES($1,$2,'fixture',$3,ARRAY['repository:analyze','receipts:read','capabilities:read','capabilities:publish'],100,100,$4,0)",[credential.into(),wallet.into(),identity::hash(token.as_bytes()).into(),i64::MAX.into()])).await?;
    }
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO capabilities(id,creator_wallet,name,description,version,runtime,price,created_at) VALUES($1,'0x1111111111111111111111111111111111111111','fixture','security fixture','1','repository_stack',1,0)",[cap.into()])).await?;
    db.execute_raw(Statement::from_sql_and_values(DbBackend::Postgres,"INSERT INTO invocations(id,credential_id,capability_id,idempotency_key,request_hash,input,requirements,resource,state,price,expires_at,created_at,updated_at,creator_wallet) VALUES($1,$2,$3,$4,'x','{}','{}','{}','quoted',1,$5,0,0,'0x1111111111111111111111111111111111111111')",[id.into(),a.into(),cap.into(),Uuid::new_v4().into(),i64::MAX.into()])).await?;
    let mut settings = Settings::from_env()?;
    settings.dodo_webhook_secret = Some("AwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwMDAwM=".into());
    let state = AppState {
        postgres: db.clone(),
        settings: Arc::new(settings),
        redis: fred::prelude::Builder::default_centralized().build()?,
        http: reqwest::Client::new(),
        rig: None,
    };
    let app = hiveway::api::protect(hiveway::api::router().with_state(state.clone()), state);
    let outcome = probe(app.clone(), id, &token_a, &token_b).await;
    let socket_result = live_socket_probe(app).await;
    let inbox = db
        .query_one_raw(Statement::from_string(
            DbBackend::Postgres,
            "SELECT count(*) AS count FROM payment_webhook_inbox",
        ))
        .await?
        .ok_or("missing count")?
        .try_get::<i64>("", "count")?;
    assert_eq!(inbox, 0, "forged payments must never enter the inbox");
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome?;
    socket_result
}
