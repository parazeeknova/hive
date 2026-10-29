//! Pagination, ownership and pinned-quote lifecycle checks with real storage.
use super::{Result, attack};
use axum::{Router, http::StatusCode};
use hiveway::{entities::capabilities, state::AppState};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
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
    availability(app.clone(), expected[0], &name, (&bearer, &foreign)).await?;
    search_tiers(app.clone(), state, &bearer).await?;
    earnings(app, state, &bearer, &foreign, &expected).await
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
async fn search_tiers(app: Router, state: &AppState, bearer: &str) -> Result {
    let weather = named_capability(
        state,
        "Weather oracle",
        "Current temperature and rainfall reports",
    )
    .await?;
    let crash = named_capability(
        state,
        "Crash reporting",
        "Error aggregation for mobile builds",
    )
    .await?;
    named_capability(
        state,
        "Oracle standings",
        "Leaderboard snapshots for seasons",
    )
    .await?;
    named_capability(
        state,
        "Unrelated helper",
        "Miscellaneous developer utilities",
    )
    .await?;
    // Stemmed multi-word query: full-text tier outranks trigram-only matches.
    let (status, page) = get(
        app.clone(),
        "/v1/capabilities/page?q=rainfall%20reporting&limit=10",
        bearer,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{page}");
    let ids: Vec<String> = page["items"]
        .as_array()
        .ok_or("missing items")?
        .iter()
        .filter_map(|row| row["id"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(ids, vec![weather.to_string(), crash.to_string()]);
    // Transposed letters defeat substring and stemming but not trigrams.
    let (status, page) = get(
        app.clone(),
        "/v1/capabilities/page?q=weather%20orcale&limit=10",
        bearer,
    )
    .await?;
    assert_eq!(status, StatusCode::OK, "{page}");
    let ids: Vec<String> = page["items"]
        .as_array()
        .ok_or("missing items")?
        .iter()
        .filter_map(|row| row["id"].as_str().map(str::to_owned))
        .collect();
    assert_eq!(ids, vec![weather.to_string()]);
    Ok(())
}

async fn named_capability(state: &AppState, name: &str, description: &str) -> Result<Uuid> {
    let id = Uuid::new_v4();
    capabilities::ActiveModel {
        id: Set(id),
        creator_wallet: Set("0x1111111111111111111111111111111111111111".into()),
        name: Set(name.into()),
        description: Set(description.into()),
        version: Set("1".into()),
        runtime: Set("repository_stack".into()),
        price: Set(1),
        created_at: Set(300),
        ..Default::default()
    }
    .insert(&state.postgres)
    .await?;
    Ok(id)
}

async fn earnings(
    app: Router,
    state: &AppState,
    bearer: &str,
    foreign: &str,
    caps: &[Uuid],
) -> Result {
    let (status, before) = get(app.clone(), "/v1/earnings/summary", bearer).await?;
    assert_eq!(status, StatusCode::OK, "{before}");
    seed_earnings(state, caps).await?;
    // Direct fixture inserts bypass the write paths, so invalidate through a
    // real one before asserting.
    let path = format!("/v1/capabilities/{}/availability", caps[0]);
    for active in [false, true] {
        let (status, _) = attack(
            app.clone(),
            "POST",
            &path,
            serde_json::json!({"active": active}).to_string(),
            &[("authorization", bearer)],
        )
        .await?;
        assert_eq!(status, StatusCode::OK);
    }
    let baseline = |key: &str| -> i64 {
        before[key]
            .as_str()
            .unwrap_or("0")
            .parse::<i64>()
            .unwrap_or(0)
    };
    let (status, summary) = get(app.clone(), "/v1/earnings/summary", bearer).await?;
    assert_eq!(status, StatusCode::OK, "{summary}");
    let total = |key: &str| -> i64 {
        summary[key]
            .as_str()
            .unwrap_or("0")
            .parse::<i64>()
            .unwrap_or(0)
    };
    assert_eq!(total("lifetime_amount") - baseline("lifetime_amount"), 400);
    assert_eq!(
        summary["settled_runs"].as_i64().unwrap_or(-1)
            - before["settled_runs"].as_i64().unwrap_or(-1),
        2
    );
    assert_eq!(total("refunded_amount") - baseline("refunded_amount"), 200);
    assert_eq!(
        summary["refunded_runs"].as_i64().unwrap_or(-1)
            - before["refunded_runs"].as_i64().unwrap_or(-1),
        1
    );
    let (status, by_capability) = get(app.clone(), "/v1/earnings/by-capability", bearer).await?;
    assert_eq!(status, StatusCode::OK, "{by_capability}");
    let rows = by_capability.as_array().ok_or("missing rows")?;
    let row = |id: Uuid| -> Result<&Value> {
        rows.iter()
            .find(|row| row["capability_id"] == id.to_string())
            .ok_or_else(|| "capability earnings row".into())
    };
    assert_eq!(row(caps[1])?["amount"], "300");
    assert_eq!(row(caps[0])?["amount"], "100");
    assert_eq!(row(caps[0])?["refunded_amount"], "200");
    // Owner-only: another wallet sees none of these credits.
    let (status, foreign_summary) = get(app, "/v1/earnings/summary", foreign).await?;
    assert_eq!(status, StatusCode::OK, "{foreign_summary}");
    assert_eq!(foreign_summary["lifetime_amount"], "0");
    assert_eq!(foreign_summary["settled_runs"], 0);
    Ok(())
}

async fn seed_earnings(state: &AppState, caps: &[Uuid]) -> Result {
    use hiveway::entities::{agent_credentials, creator_earnings, invocations};
    let credential = agent_credentials::Entity::find()
        .filter(agent_credentials::Column::Wallet.eq("0x1111111111111111111111111111111111111111"))
        .one(&state.postgres)
        .await?
        .ok_or("missing fixture credential")?
        .id;
    let mut invocations_ids = Vec::new();
    for cap in [caps[0], caps[0], caps[1]] {
        let id = Uuid::new_v4();
        invocations::ActiveModel {
            id: Set(id),
            credential_id: Set(credential),
            capability_id: Set(cap),
            creator_wallet: Set("0x1111111111111111111111111111111111111111".into()),
            idempotency_key: Set(Uuid::new_v4()),
            request_hash: Set(vec![1; 32]),
            input: Set(serde_json::json!({})),
            requirements: Set(serde_json::json!({})),
            resource: Set(serde_json::json!({})),
            state: Set("settled".into()),
            result: Set(Some(serde_json::json!({"ok": true}))),
            settlement: Set(Some(serde_json::json!({"success": true}))),
            price: Set(1),
            expires_at: Set(0),
            created_at: Set(0),
            updated_at: Set(0),
            ..Default::default()
        }
        .insert(&state.postgres)
        .await?;
        invocations_ids.push(id);
    }
    for (invocation_id, capability_id, method, amount, network, refunded) in [
        (
            invocations_ids[0],
            caps[0],
            "x402",
            100_i64,
            "eip155:84532",
            false,
        ),
        (
            invocations_ids[1],
            caps[0],
            "dodo",
            200_i64,
            "dodo:test",
            true,
        ),
        (
            invocations_ids[2],
            caps[1],
            "x402",
            300_i64,
            "eip155:84532",
            false,
        ),
    ] {
        creator_earnings::ActiveModel {
            invocation_id: Set(invocation_id),
            capability_id: Set(capability_id),
            creator_wallet: Set("0x1111111111111111111111111111111111111111".into()),
            payer_wallet: Set("0x9999999999999999999999999999999999999999".into()),
            amount: Set(amount),
            network: Set(network.into()),
            transaction_hash: Set(format!("0x{invocation_id:?}")),
            payment_method: Set(method.into()),
            refunded: Set(refunded),
            created_at: Set(0),
        }
        .insert(&state.postgres)
        .await?;
    }
    Ok(())
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
