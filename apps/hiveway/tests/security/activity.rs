use super::{Result, attack};
use axum::{Router, http::StatusCode};
use hiveway::{
    entities::{agent_credentials, invocations},
    features::common::model::now,
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::json;
use uuid::Uuid;
pub(super) async fn exercise(
    app: Router,
    state: &AppState,
    browser_headers: &[(&str, &str)],
    key_bearer: &str,
) -> Result {
    let key_headers = [("authorization", key_bearer)];
    let (_, key) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        &key_headers,
    )
    .await?;
    let key_id = key["id"]
        .as_str()
        .ok_or("missing credential ID")?
        .parse::<Uuid>()?;
    let row = invocations::Entity::find()
        .filter(invocations::Column::CredentialId.eq(key_id))
        .filter(invocations::Column::StartedAt.is_not_null())
        .one(&state.postgres)
        .await?
        .ok_or("missing invocation")?;
    let (_, browser) = attack(
        app.clone(),
        "GET",
        "/v1/credentials/me",
        String::new(),
        browser_headers,
    )
    .await?;
    let browser_id = browser["id"]
        .as_str()
        .ok_or("missing browser ID")?
        .parse::<Uuid>()?;
    let mut second: invocations::ActiveModel = row.clone().into();
    let second_id = Uuid::new_v4();
    second.id = Set(second_id);
    second.credential_id = Set(browser_id);
    second.idempotency_key = Set(Uuid::new_v4());
    second.insert(&state.postgres).await?;
    let (status, history) = attack(
        app.clone(),
        "GET",
        "/v1/activity",
        String::new(),
        browser_headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{history}");
    cursor_omits_counts(&app, browser_headers, &history).await?;
    let records = history["invocations"].as_array().ok_or("missing history")?;
    assert!(records.iter().any(|r| r["id"] == row.id.to_string()));
    assert!(records.iter().any(|r| r["id"] == second_id.to_string()));
    assert!(
        history["counts"]["total"]
            .as_u64()
            .is_some_and(|count| count >= 2)
    );
    assert!(
        records
            .iter()
            .any(|r| r["client_context"]["model"] == "fixture-model")
    );
    let (status, _) = attack(
        app.clone(),
        "GET",
        &format!("/v1/invocations/{}", row.id),
        String::new(),
        browser_headers,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "browser account can read its API key receipt"
    );
    let (_, scoped) = attack(
        app.clone(),
        "GET",
        "/v1/activity",
        String::new(),
        &key_headers,
    )
    .await?;
    assert!(
        scoped["invocations"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|r| r["id"] != second_id.to_string()))
    );
    let (status, _) = attack(
        app.clone(),
        "GET",
        &format!("/v1/invocations/{second_id}"),
        String::new(),
        &key_headers,
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    metadata_and_plan(&app, browser_headers, row.capability_id).await?;
    revoked_history(app, state, browser_headers, key_bearer, key_id, row.id).await
}
async fn metadata_and_plan(
    app: &Router,
    browser_headers: &[(&str, &str)],
    capability: Uuid,
) -> Result {
    let (status, metadata) = attack(
        app.clone(),
        "POST",
        "/v1/catalog/metadata",
        json!({"ids":[capability]}).to_string(),
        browser_headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{metadata}");
    assert!(
        metadata[0]["invocation_count"]
            .as_u64()
            .is_some_and(|count| count >= 2)
    );
    assert_eq!(metadata[0]["publisher"]["verified"], false);
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/account",
        json!({"name":"Test","username":"test","avatar":"","publisher_verified":true}).to_string(),
        browser_headers,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "verification is never self-assigned"
    );
    let (_, plan) = attack(
        app.clone(),
        "GET",
        "/v1/account/plan",
        String::new(),
        browser_headers,
    )
    .await?;
    assert_eq!(plan["tier"], "free");
    assert_eq!(plan["build_credits"]["used"], 3);
    assert_eq!(plan["build_credits"]["available"], 0);
    expiry_choices(app, browser_headers).await?;
    Ok(())
}
async fn revoked_history(
    app: Router,
    state: &AppState,
    browser_headers: &[(&str, &str)],
    key_bearer: &str,
    key_id: Uuid,
    invocation_id: Uuid,
) -> Result {
    let key_headers = [("authorization", key_bearer)];
    // Revoked keys remain in the owner's history but cannot authenticate.
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now()),
        )
        .filter(agent_credentials::Column::Id.eq(key_id))
        .exec(&state.postgres)
        .await?;
    let (status, _) = attack(
        app.clone(),
        "GET",
        "/v1/activity",
        String::new(),
        &key_headers,
    )
    .await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (_, history) = attack(app, "GET", "/v1/activity", String::new(), browser_headers).await?;
    assert!(
        history["invocations"]
            .as_array()
            .is_some_and(|rows| rows.iter().any(|r| r["id"] == invocation_id.to_string()))
    );
    // Restore this fixture key for the existing key-removal tests.
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(None::<i64>),
        )
        .filter(agent_credentials::Column::Id.eq(key_id))
        .exec(&state.postgres)
        .await?;
    Ok(())
}
async fn expiry_choices(app: &Router, headers: &[(&str, &str)]) -> Result {
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/account/credentials",
        json!({"name":"Invalid expiry","expires_in":123}).to_string(),
        headers,
    )
    .await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let before = now();
    let (status, key) = attack(
        app.clone(),
        "POST",
        "/v1/account/credentials",
        json!({"name":"Long-lived key","expires_in":7_776_000}).to_string(),
        headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{key}");
    assert!(
        key["expires_at"]
            .as_i64()
            .is_some_and(|expires| expires >= before + 7_776_000 && expires <= now() + 7_776_000)
    );
    let (_, listed) = attack(
        app.clone(),
        "GET",
        "/v1/account/credentials",
        String::new(),
        headers,
    )
    .await?;
    assert!(
        listed["keys"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| row.get("token").is_none()))
    );
    Ok(())
}

async fn cursor_omits_counts(
    app: &Router,
    headers: &[(&str, &str)],
    history: &serde_json::Value,
) -> Result {
    let first = &history["invocations"][0];
    let time = first["created_at"].as_i64().ok_or("missing time")?;
    let id = first["id"].as_str().ok_or("missing ID")?;
    let (status, page) = attack(
        app.clone(),
        "GET",
        &format!("/v1/activity?before_time={time}&before_id={id}"),
        String::new(),
        headers,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{page}");
    assert!(
        page.get("counts").is_none(),
        "later pages must not repeat account-wide counts"
    );
    assert!(
        page["invocations"]
            .as_array()
            .is_some_and(|rows| rows.iter().all(|row| row["id"] != id))
    );
    Ok(())
}
