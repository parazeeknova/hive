use crate::{
    entities::{agent_credentials as credentials, invocations as runs},
    features::{
        common::{error::ApiResult, model::Credential},
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use sea_orm::{
    ColumnTrait, Condition, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect,
    QueryTrait,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;
#[derive(Default, Deserialize, utoipa::IntoParams)]
#[serde(deny_unknown_fields)]
pub struct ActivityQuery {
    pub before_time: Option<i64>,
    pub before_id: Option<Uuid>,
}
/// Browser accounts see their owned credentials; scoped API keys retain credential-only visibility.
/// # Errors
/// Returns database failures.
pub async fn credential_scope(
    state: &AppState,
    credential: &Credential,
) -> ApiResult<sea_orm::sea_query::SimpleExpr> {
    let account = account_history(state, credential).await?;
    Ok(if account {
        runs::Column::CredentialId.in_subquery(
            credentials::Entity::find()
                .select_only()
                .column(credentials::Column::Id)
                .filter(credentials::Column::Wallet.eq(&credential.wallet))
                .into_query(),
        )
    } else {
        runs::Column::CredentialId.eq(credential.id)
    })
}
/// Whether this credential is a browser account, rather than an account-scoped API key.
/// # Errors
/// Returns database failures.
pub async fn account_history(state: &AppState, credential: &Credential) -> ApiResult<bool> {
    let account = credential.wallet.starts_with("github:")
        && crate::entities::account_api_keys::Entity::find_by_id(credential.id)
            .one(&state.postgres)
            .await?
            .is_none();
    Ok(account)
}
/// Account-wide browser history, with keyset pagination and counts over all matching records.
/// # Errors
/// Rejects invalid credentials and database failures.
#[utoipa::path(get,path="/v1/activity",params(ActivityQuery),responses((status=200,body=Value)))]
pub async fn get(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ActivityQuery>,
) -> ApiResult<Json<Value>> {
    let agent = identity::authenticate(&state, &headers, "receipts:read").await?;
    let scope = credential_scope(&state, &agent).await?;
    let all = runs::Entity::find().filter(scope);
    let counts = if query.before_time.is_none() && query.before_id.is_none() {
        Some(history_counts(&state, all.clone()).await?)
    } else {
        None
    };
    let filtered = match (query.before_time, query.before_id) {
        (Some(time), Some(id)) => all.filter(
            Condition::any().add(runs::Column::CreatedAt.lt(time)).add(
                Condition::all()
                    .add(runs::Column::CreatedAt.eq(time))
                    .add(runs::Column::Id.lt(id)),
            ),
        ),
        (None, None) => all,
        _ => {
            return Err(crate::features::common::error::ApiError::bad(
                "invalid_activity_cursor",
            ));
        }
    };
    let mut records = filtered
        .order_by_desc(runs::Column::CreatedAt)
        .order_by_desc(runs::Column::Id)
        .limit(101)
        .all(&state.postgres)
        .await?;
    let account = account_history(&state, &agent).await?;
    let before = query.before_time.zip(query.before_id);
    let mut tool_calls =
        crate::features::tool_activity::list(&state, &agent, account, before).await?;
    let mut keys: Vec<(i64, Uuid)> = records.iter().map(|r| (r.created_at, r.id)).collect();
    for event in &tool_calls {
        if let (Some(time), Some(id)) = (
            event["created_at"].as_i64(),
            event["id"].as_str().and_then(|v| v.parse::<Uuid>().ok()),
        ) {
            keys.push((time, id));
        }
    }
    keys.sort_unstable_by(|a, b| b.cmp(a));
    let more = keys.len() > 100;
    keys.truncate(100);
    let selected: std::collections::HashSet<Uuid> = keys.iter().map(|(_, id)| *id).collect();
    records.retain(|r| selected.contains(&r.id));
    tool_calls.retain(|r| {
        r["id"]
            .as_str()
            .and_then(|v| v.parse::<Uuid>().ok())
            .is_some_and(|id| selected.contains(&id))
    });
    let cursor = if more {
        keys.last()
            .map(|(time, id)| json!({"before_time":time,"before_id":id}))
    } else {
        None
    };
    let ids: Vec<Uuid> = records.iter().map(|r| r.capability_id).collect();
    let caps: std::collections::HashMap<_, _> = crate::entities::capabilities::Entity::find()
        .filter(crate::entities::capabilities::Column::Id.is_in(ids))
        .all(&state.postgres)
        .await?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();
    let ids: Vec<Uuid> = records.iter().map(|r| r.credential_id).collect();
    let agents: std::collections::HashMap<_, _> = credentials::Entity::find()
        .filter(credentials::Column::Id.is_in(ids))
        .all(&state.postgres)
        .await?
        .into_iter()
        .map(|c| (c.id, c.name))
        .collect();
    let invocations:Vec<Value>=records.into_iter().map(|r|json!({"id":r.id,"capability_id":r.capability_id,"capability_name":caps.get(&r.capability_id),"agent_name":agents.get(&r.credential_id),"client_context":r.client_context,"state":r.state,"price":r.price.to_string(),"payment_method":r.payment_method,"payment_issue":r.payment_issue,"execution_ms":r.execution_ms,"result_bytes":r.result_bytes,"created_at":r.created_at,"completed_at":r.completed_at})).collect();
    let mut response = json!({"invocations":invocations,"tool_calls":tool_calls,"next_cursor":cursor,"budget":{"limit":agent.total_limit.to_string(),"spent":agent.spent.to_string(),"reserved":agent.reserved.to_string()}});
    if let Some(counts) = counts {
        response["counts"] = counts;
        response["tool_counts"] =
            crate::features::tool_activity::counts(&state, &agent, account).await?;
    }
    Ok(Json(response))
}

async fn history_counts(state: &AppState, all: sea_orm::Select<runs::Entity>) -> ApiResult<Value> {
    let total = all
        .clone()
        .filter(runs::Column::StartedAt.is_not_null())
        .count(&state.postgres)
        .await?;
    let succeeded = all
        .clone()
        .filter(runs::Column::State.is_in(["succeeded", "settled"]))
        .count(&state.postgres)
        .await?;
    let failed = all
        .clone()
        .filter(runs::Column::StartedAt.is_not_null())
        .filter(runs::Column::State.is_in(["execution_failed", "settlement_failed", "failed"]))
        .count(&state.postgres)
        .await?;
    Ok(json!({"total":total,"succeeded":succeeded,"failed":failed}))
}
