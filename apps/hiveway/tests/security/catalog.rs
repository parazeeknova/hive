//! Pagination, ownership and pinned-quote lifecycle checks with real storage.
use super::{Result, attack};
use axum::{Router, http::StatusCode};
use hiveway::{entities::capabilities, state::AppState};
use sea_orm::{ActiveModelTrait, Set};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) async fn exercise(app: Router, state: &AppState, tokens: (&str, &str)) -> Result {
    let bearer = format!("Bearer {}", tokens.0);
    let foreign = format!("Bearer {}", tokens.1);
    let name = format!("catalog-{}", Uuid::new_v4().simple());
    let mut expected = Vec::new();
    for _ in 0..4 {
        expected.push(seed(state, &name, 100).await?);
    }
    expected.sort();
    let path = format!("/v1/capabilities/page?q={name}&limit=2");
    let (status, first) = get(app.clone(), &path, &bearer).await?;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["items"].as_array().ok_or("missing items")?.len(), 2);
    // A newer listing must not shift the subsequent page or repeat previous rows.
    seed(state, &name, 200).await?;
    let cursor = first["next_cursor"].as_str().ok_or("missing cursor")?;
    let (status, second) = get(app.clone(), &format!("{path}&cursor={cursor}"), &bearer).await?;
    assert_eq!(status, StatusCode::OK);
    assert!(second["next_cursor"].is_null());
    let actual: Vec<Uuid> = first["items"]
        .as_array()
        .ok_or("missing items")?
        .iter()
        .chain(second["items"].as_array().ok_or("missing items")?)
        .map(|row| serde_json::from_value(row["id"].clone()))
        .collect::<std::result::Result<_, _>>()?;
    assert_eq!(
        actual, expected,
        "timestamp ties must use stable UUID ordering"
    );
    for path in [
        format!("/v1/capabilities/page?q=changed&cursor={cursor}"),
        "/v1/capabilities/page?limit=0".into(),
        "/v1/capabilities/page?runtime=container".into(),
        "/v1/capabilities/page?cursor=not-a-cursor".into(),
    ] {
        let (status, _) = get(app.clone(), &path, &bearer).await?;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    let (status, _) = get(
        app.clone(),
        "/v1/capabilities/page?include_inactive=true",
        &bearer,
    )
    .await?;
    assert_eq!(status, StatusCode::FORBIDDEN);
    availability(app, expected[0], &name, (&bearer, &foreign)).await
}
async fn seed(state: &AppState, name: &str, created_at: i64) -> Result<Uuid> {
    let id = Uuid::new_v4();
    capabilities::ActiveModel {
        id: Set(id),
        creator_wallet: Set("0x1111111111111111111111111111111111111111".into()),
        name: Set(name.into()),
        description: Set("catalog test".into()),
        version: Set("1".into()),
        runtime: Set("repository_stack".into()),
        price: Set(1),
        created_at: Set(created_at),
        ..Default::default()
    }
    .insert(&state.postgres)
    .await?;
    Ok(id)
}
async fn get(app: Router, path: &str, bearer: &str) -> Result<(StatusCode, Value)> {
    attack(
        app,
        "GET",
        path,
        String::new(),
        &[("authorization", bearer)],
    )
    .await
}
#[allow(clippy::too_many_lines)]
async fn availability(app: Router, id: Uuid, name: &str, bearers: (&str, &str)) -> Result {
    let (a, b) = bearers;
    let key = Uuid::new_v4().to_string();
    let input = json!({"capability_id":id,"input":{"repository_url":"https://github.com/a/b","revision":"a".repeat(40)}});
    let quote_headers = [("authorization", a), ("idempotency-key", key.as_str())];
    let (status, quote) = attack(
        app.clone(),
        "POST",
        "/v1/quotes",
        input.to_string(),
        &quote_headers,
    )
    .await?;
    assert_eq!(status, StatusCode::CREATED, "{quote}");
    let path = format!("/v1/capabilities/{id}/availability");
    let (status, _) = attack(
        app.clone(),
        "POST",
        &path,
        json!({"active":false}).to_string(),
        &[("authorization", b)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "another wallet must not alter a listing"
    );
    let (status, cap) = attack(
        app.clone(),
        "POST",
        &path,
        json!({"active":false}).to_string(),
        &[("authorization", a)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(cap["active"], false);
    let (status, detail) = get(app.clone(), &format!("/v1/capabilities/{id}"), b).await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "pinned public metadata remains readable"
    );
    assert_eq!(detail["id"], cap["id"]);
    let (status, own) = get(
        app.clone(),
        &format!("/v1/capabilities/page?q={name}&mine=true&include_inactive=true"),
        a,
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(own["items"].as_array().ok_or("missing items")?.len(), 5);
    let (_, foreign) = get(
        app.clone(),
        &format!("/v1/capabilities/page?q={name}&mine=true&include_inactive=true"),
        b,
    )
    .await?;
    assert_eq!(foreign["items"], json!([]));
    let (_, active) = get(app.clone(), &format!("/v1/capabilities/page?q={name}"), a).await?;
    assert!(
        !active["items"]
            .as_array()
            .ok_or("missing items")?
            .iter()
            .any(|cap| cap["id"] == id.to_string())
    );
    let (status, replay) = attack(
        app.clone(),
        "POST",
        "/v1/quotes",
        input.to_string(),
        &quote_headers,
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::OK,
        "an existing quote survives unlisting"
    );
    assert_eq!(replay["id"], quote["id"]);
    let new_key = Uuid::new_v4().to_string();
    let (status, _) = attack(
        app.clone(),
        "POST",
        "/v1/quotes",
        input.to_string(),
        &[("authorization", a), ("idempotency-key", &new_key)],
    )
    .await?;
    assert_eq!(
        status,
        StatusCode::NOT_FOUND,
        "unlisted capabilities cannot receive new quotes"
    );
    let (status, _) = attack(
        app,
        "POST",
        &path,
        json!({"active":true}).to_string(),
        &[("authorization", a)],
    )
    .await?;
    assert_eq!(status, StatusCode::OK);
    Ok(())
}
