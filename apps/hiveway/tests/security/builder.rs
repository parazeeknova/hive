use super::{Result, attack};
use axum::{
    Json, Router,
    http::{HeaderMap, StatusCode},
};
use hiveway::{
    entities::{agent_credentials, build_jobs, capabilities},
    features::{builder::worker, common::model::now, identity},
    state::AppState,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};
use uuid::Uuid;

pub(super) async fn provider(headers: HeaderMap, Json(request): Json<Value>) -> Json<Value> {
    assert_eq!(
        headers.get("authorization").and_then(|v| v.to_str().ok()),
        Some("Bearer fixture")
    );
    assert_eq!(request["model"], "test-wasm-generator");
    let source = if request["messages"]
        .as_array()
        .is_some_and(|messages| messages.len() == 2)
    {
        "(module invalid)"
    } else {
        assert!(request["messages"][3]["content"].as_str().is_some_and(|text|text.contains("builder_invalid_wat") && text.contains("detail")));
        include_str!("../../../../packages/local/fixtures/wasm/echo.wat")
    };
    Json(
        json!({"choices":[{"finish_reason":"stop","message":{"content":serde_json::to_string(&json!({"wat":source,"input_schema":{"type":"object"},"output_schema":{"type":"object"}})).unwrap_or_default()}}]}),
    )
}
async fn submit(app: &Router, bearer: &str, request: &Value) -> Result<(StatusCode, Value)> {
    attack(
        app.clone(),
        "POST",
        "/v1/builds",
        request.to_string(),
        &[("authorization", bearer)],
    )
    .await
}
pub(super) async fn exercise(app: Router, state: &AppState, bearer: &str) -> Result {
    let request = json!({"idempotency_key":Uuid::new_v4(),"name":"Hosted echo","description":"Free build without a wallet","task":"Return any input object unchanged","examples":[{"input":{"hello":"world"},"expected":{"hello":"world"}},{"input":{"different":42},"expected":{"different":42}}]});
    let (status, _) = attack(app.clone(), "POST", "/v1/builds", request.to_string(), &[]).await?;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let mut invalid = request.clone();
    invalid["examples"][0]["input"] = json!(3);
    assert_eq!(
        submit(&app, bearer, &invalid).await?.0,
        StatusCode::BAD_REQUEST
    );
    let (status, job) = submit(&app, bearer, &request).await?;
    assert_eq!(status, StatusCode::OK, "{job}");
    let id = job["id"]
        .as_str()
        .ok_or("missing build ID")?
        .parse::<Uuid>()?;
    let (_, retry) = submit(&app, bearer, &request).await?;
    assert_eq!(retry["id"], job["id"]);
    let mut altered = request.clone();
    altered["task"] = json!("Changed task");
    assert_eq!(
        submit(&app, bearer, &altered).await?.0,
        StatusCode::CONFLICT
    );
    altered["idempotency_key"] = json!(Uuid::new_v4());
    assert_eq!(
        submit(&app, bearer, &altered).await?.0,
        StatusCode::CONFLICT
    );
    let (first, second) = tokio::join!(worker::tick(state), worker::tick(state));
    first?;
    second?;
    let row = build_jobs::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing job")?;
    assert_eq!(row.state, "published", "{:?}", row.error_code);
    assert_eq!(
        row.attempts, 2,
        "the provider repairs a real compiler failure"
    );
    let cap = capabilities::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing capability")?;
    assert_eq!(cap.creator_wallet, "github:123");
    assert_eq!(cap.price, 0);
    assert_eq!(cap.runtime, "wasm");
    let headers = [("authorization", bearer)];
    let (_, tools) = attack(
        app.clone(),
        "POST",
        "/mcp",
        json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}).to_string(),
        &headers,
    )
    .await?;
    assert!(tools["result"]["tools"].as_array().is_some_and(|tools| {
        tools
            .iter()
            .any(|t| t["name"] == format!("hive_{}", id.simple()))
    }));
    let (_,status)=attack(app.clone(),"POST","/mcp",json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_build","arguments":{"build_id":id}}}).to_string(),&headers).await?;
    assert_eq!(status["result"]["structuredContent"]["state"], "published");
    let mut failure = request.clone();
    failure["idempotency_key"] = json!(Uuid::new_v4());
    failure["examples"][0]["expected"] = json!({"wrong":true});
    let (_, failed) = submit(&app, bearer, &failure).await?;
    worker::tick(state).await?;
    let failed_id = failed["id"]
        .as_str()
        .ok_or("missing failed ID")?
        .parse::<Uuid>()?;
    let row = build_jobs::Entity::find_by_id(failed_id)
        .one(&state.postgres)
        .await?
        .ok_or("missing failed build")?;
    assert_eq!(row.state, "failed");
    assert_eq!(row.attempts, 2);
    assert!(row.capability_id.is_none());
    assert!(
        capabilities::Entity::find_by_id(failed_id)
            .one(&state.postgres)
            .await?
            .is_none()
    );
    stale_job(state, failed_id).await?;
    revoked_and_limits(&app, state, bearer, &request, id).await
}
async fn stale_job(state: &AppState, id: Uuid) -> Result {
    use sea_orm::sea_query::Expr;
    build_jobs::Entity::update_many()
        .col_expr(build_jobs::Column::State, Expr::value("generating"))
        .col_expr(build_jobs::Column::UpdatedAt, Expr::value(now() - 601))
        .filter(build_jobs::Column::Id.eq(id))
        .exec(&state.postgres)
        .await?;
    worker::tick(state).await?;
    let row = build_jobs::Entity::find_by_id(id)
        .one(&state.postgres)
        .await?
        .ok_or("missing stale job")?;
    assert_eq!(row.state, "failed");
    assert_eq!(row.error_code.as_deref(), Some("builder_interrupted"));
    assert_eq!(row.attempts, 2);
    Ok(())
}
async fn revoked_and_limits(
    app: &Router,
    state: &AppState,
    bearer: &str,
    request: &Value,
    id: Uuid,
) -> Result {
    let original = agent_credentials::Entity::find()
        .filter(agent_credentials::Column::TokenHash.eq(identity::hash(
            bearer.trim_start_matches("Bearer ").as_bytes(),
        )))
        .one(&state.postgres)
        .await?
        .ok_or("missing original credential")?;
    let mut credential: agent_credentials::ActiveModel = original.into();
    let token = format!("hwa_{}", Uuid::new_v4().simple().to_string().repeat(2));
    let credential_id = Uuid::new_v4();
    credential.id = Set(credential_id);
    credential.token_hash = Set(identity::hash(token.as_bytes()));
    credential.insert(&state.postgres).await?;
    let mut request = request.clone();
    request["idempotency_key"] = json!(Uuid::new_v4());
    let (queued_status, queued) = submit(app, &format!("Bearer {token}"), &request).await?;
    assert_eq!(queued_status, StatusCode::OK, "{queued}");
    agent_credentials::Entity::update_many()
        .col_expr(
            agent_credentials::Column::RevokedAt,
            sea_orm::sea_query::Expr::value(now()),
        )
        .filter(agent_credentials::Column::Id.eq(credential_id))
        .exec(&state.postgres)
        .await?;
    worker::tick(state).await?;
    let (_, failed) = attack(
        app.clone(),
        "GET",
        &format!(
            "/v1/builds/{}",
            queued["id"].as_str().ok_or("missing queued ID")?
        ),
        String::new(),
        &[("authorization", bearer)],
    )
    .await?;
    assert_eq!(failed["state"], "failed");
    assert_eq!(failed["attempts"], 0);
    request["idempotency_key"] = json!(Uuid::new_v4());
    assert_eq!(
        submit(app, bearer, &request).await?.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    // A valid credential for a different account cannot read generated private source.
    let mut stranger: agent_credentials::ActiveModel =
        agent_credentials::Entity::find_by_id(credential_id)
            .one(&state.postgres)
            .await?
            .ok_or("missing credential")?
            .into();
    stranger.id = Set(Uuid::new_v4());
    stranger.wallet = Set("github:987654322".into());
    stranger.revoked_at = Set(None);
    let stranger_token = format!("hwa_{}", Uuid::new_v4().simple().to_string().repeat(2));
    stranger.token_hash = Set(identity::hash(stranger_token.as_bytes()));
    stranger.insert(&state.postgres).await?;
    let stranger_bearer = format!("Bearer {stranger_token}");
    let (status, _) = attack(
        app.clone(),
        "GET",
        &format!("/v1/builds/{id}"),
        String::new(),
        &[("authorization", &stranger_bearer)],
    )
    .await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}
