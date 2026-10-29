#![cfg(not(miri))]
use axum::{
    Router,
    body::{Body, to_bytes},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use hiveway::{
    features::{
        common::{
            model::{AuthResponse, ChallengeResponse},
            settings::Settings,
        },
        identity::signature,
    },
    state::AppState,
};
use sea_orm::{ConnectionTrait, Database};
use serde_json::{Value, json};
use std::{net::SocketAddr, sync::Arc};
use tower::ServiceExt;
use uuid::Uuid;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
async fn call(
    app: Router,
    uri: &str,
    body: Value,
    token: Option<&str>,
) -> TestResult<(StatusCode, Value)> {
    let mut request = Request::post(uri).header("content-type", "application/json");
    if let Some(token) = token {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let mut request = request.body(Body::from(serde_json::to_vec(&body)?))?;
    request
        .extensions_mut()
        .insert(ConnectInfo("127.0.0.1:4000".parse::<SocketAddr>()?));
    let response = app.oneshot(request).await?;
    let status = response.status();
    let value = serde_json::from_slice(&to_bytes(response.into_body(), 1_000_000).await?)?;
    Ok((status, value))
}
fn sign(key: &k256::ecdsa::SigningKey, message: &str) -> String {
    let (sig, id) = key.sign_prehash_recoverable(&signature::digest(message));
    let mut bytes = sig.to_bytes().to_vec();
    bytes.push(id.to_byte() + 27);
    format!("0x{}", hex::encode(bytes))
}
async fn exercise(app: Router) -> TestResult {
    let key = k256::ecdsa::SigningKey::from_slice(&[1; 32])?;
    let wallet = "0x1a642f0e3c3af545e7acbd38b07251b3990914f1";
    let action = json!({"type":"issue","policy":{"name":"creator","scopes":["capabilities:publish"],"per_call_limit":"10000","total_limit":"100000","expires_at":time::OffsetDateTime::now_utc().unix_timestamp()+3600}});
    let (status, challenge) = call(
        app.clone(),
        "/v1/auth/challenges",
        json!({"wallet":wallet,"action":action}),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let challenge: ChallengeResponse = serde_json::from_value(challenge)?;
    assert!(challenge.message.contains("Total limit: 100000"));
    let tampered = sign(&key, &challenge.message.replace("100000", "999999"));
    let (status, _) = call(
        app.clone(),
        "/v1/auth/verify",
        json!({"challenge_id":challenge.id,"signature":tampered}),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let signature = sign(&key, &challenge.message);
    let (status, auth) = call(
        app.clone(),
        "/v1/auth/verify",
        json!({"challenge_id":challenge.id,"signature":signature}),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let auth: AuthResponse = serde_json::from_value(auth)?;
    let token = auth.token.ok_or("missing token")?;
    let (status, _) = call(
        app.clone(),
        "/v1/auth/verify",
        json!({"challenge_id":challenge.id,"signature":signature}),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let(status,_)=call(app.clone(),"/v1/quotes",json!({"capability_id":Uuid::new_v4(),"input":{"repository_url":"https://github.com/a/b","revision":"a".repeat(40)}}),Some(&token)).await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    let(status,cap)=call(app.clone(),"/v1/capabilities",json!({"name":"Repository analysis","description":"bounded execution","runtime":"repository_stack","price":"10000"}),Some(&token)).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cap["creator_wallet"], wallet);
    let (status, challenge) = call(
        app.clone(),
        "/v1/auth/challenges",
        json!({"wallet":wallet,"action":{"type":"revoke","credential_id":auth.credential_id}}),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let challenge: ChallengeResponse = serde_json::from_value(challenge)?;
    let (status, _) = call(
        app.clone(),
        "/v1/auth/verify",
        json!({"challenge_id":challenge.id,"signature":sign(&key,&challenge.message)}),
        None,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = call(
        app,
        "/v1/capabilities",
        json!({"name":"rejected","description":"revoked","runtime":"repository_stack","price":"1"}),
        Some(&token),
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    Ok(())
}
#[tokio::test]
#[ignore = "requires HIVE_TEST_DATABASE_URL pointing at local PostgreSQL"]
async fn wallet_signatures_replay_scopes_and_revocation() -> TestResult {
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
    let state = AppState {
        sandbox: Arc::new(hiveway::features::cell::runtime::Sandbox::new(
            hiveway::runtime::RuntimeConfig::load()?.sandbox,
        )),
        postgres: db.clone(),
        settings: Arc::new(Settings::from_env()?),
        redis: fred::prelude::Builder::default_centralized().build()?,
        http: reqwest::Client::new(),
        rig: None,
    };
    let app = hiveway::api::router().with_state(state.clone());
    let outcome = async {
        exercise(app.clone()).await?;
        challenge_lock_boundaries(app.clone(), &state).await?;
        challenge_limits_and_retention(app, &state).await
    }
    .await;
    db.close().await?;
    admin
        .execute_unprepared(&format!("DROP SCHEMA {schema} CASCADE"))
        .await?;
    admin.close().await?;
    outcome
}

async fn challenge_limits_and_retention(app: Router, state: &AppState) -> TestResult {
    use hiveway::entities::wallet_challenges;
    use sea_orm::{ActiveModelTrait, EntityTrait, Set};
    let at = time::OffsetDateTime::now_utc().unix_timestamp();
    let victim = "0x2222222222222222222222222222222222222222";
    for _ in 0..10 {
        wallet_challenges::ActiveModel {
            id: Set(Uuid::new_v4()),
            wallet: Set(victim.into()),
            peer_ip: Set("192.0.2.1".into()),
            message: Set("fixture".into()),
            action: Set(json!({})),
            created_at: Set(at),
            expires_at: Set(at + 300),
            consumed_at: Set(None),
        }
        .insert(&state.postgres)
        .await?;
    }
    let request =
        json!({"wallet":victim,"action":{"type":"revoke","credential_id":Uuid::new_v4()}});
    let (status, _) = call(app.clone(), "/v1/auth/challenges", request.clone(), None).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "other peers must not exhaust a victim wallet limit"
    );
    let mut old_id = Uuid::nil();
    let recent_id = Uuid::new_v4();
    for index in 0..11 {
        let id = if index == 10 {
            recent_id
        } else {
            Uuid::new_v4()
        };
        let expired = if index == 9 {
            old_id = id;
            at - state.settings.runtime.worker.challenge_retention_seconds - 1
        } else {
            at - 1
        };
        wallet_challenges::ActiveModel {
            id: Set(id),
            wallet: Set(victim.into()),
            peer_ip: Set("127.0.0.1".into()),
            message: Set("fixture".into()),
            action: Set(json!({})),
            created_at: Set(at),
            expires_at: Set(expired),
            consumed_at: Set(None),
        }
        .insert(&state.postgres)
        .await?;
    }
    hiveway::features::identity::cleanup_challenges(state).await?;
    assert!(
        wallet_challenges::Entity::find_by_id(old_id)
            .one(&state.postgres)
            .await?
            .is_none()
    );
    assert!(
        wallet_challenges::Entity::find_by_id(recent_id)
            .one(&state.postgres)
            .await?
            .is_some()
    );
    let (status, _) = call(app, "/v1/auth/challenges", request, None).await?;
    assert_eq!(
        status,
        StatusCode::TOO_MANY_REQUESTS,
        "recent expired challenges still count per peer"
    );
    Ok(())
}

async fn challenge_lock_boundaries(app: Router, state: &AppState) -> TestResult {
    use sea_orm::TransactionTrait;
    use sea_orm::sea_query::{Alias, Func, Query};
    use sha2::{Digest, Sha256};
    let wallet = "0x3333333333333333333333333333333333333333";
    for (resource, blocks) in [
        (format!("hive:wallet:{wallet}"), false),
        ("hive:ip:127.0.0.1".into(), true),
    ] {
        let tx = state.postgres.begin().await?;
        let digest = Sha256::digest(resource.as_bytes());
        let key = i64::from_be_bytes(digest[..8].try_into()?);
        tx.query_one(
            &Query::select()
                .expr(Func::cust(Alias::new("pg_advisory_xact_lock")).arg(key))
                .to_owned(),
        )
        .await?;
        let request =
            json!({"wallet":wallet,"action":{"type":"revoke","credential_id":Uuid::new_v4()}});
        let pending = call(app.clone(), "/v1/auth/challenges", request, None);
        tokio::pin!(pending);
        if blocks {
            assert!(
                tokio::time::timeout(std::time::Duration::from_millis(50), &mut pending)
                    .await
                    .is_err()
            );
            tx.commit().await?;
            assert_eq!(pending.await?.0, StatusCode::OK);
        } else {
            assert_eq!(
                tokio::time::timeout(std::time::Duration::from_secs(2), pending)
                    .await??
                    .0,
                StatusCode::OK
            );
            tx.commit().await?;
        }
    }
    Ok(())
}
