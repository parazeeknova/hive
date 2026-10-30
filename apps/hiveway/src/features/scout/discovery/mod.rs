//! Tiered-relevance keyset pagination with a versioned Redis cache.
//!
//! Relevance tiers are integers so keyset boundaries stay exact: `0` is an
//! exact substring hit, `1` is a full-text match, `2` is trigram similarity.
//! Empty queries match everything at tier `0`, which keeps the legacy
//! recency ordering. Cached pages are keyed by a catalog version that write
//! paths bump, so publish and availability changes invalidate immediately.
mod cursor;
pub mod model;
use crate::{
    entities::capabilities::{Column, Entity},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::Capability,
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use cursor::Cursor;
use fred::interfaces::KeysInterface;
use fred::types::Expiration;
use model::{CapabilityPage, PageQuery};
use sea_orm::sea_query::ExprTrait;
use sea_orm::sea_query::SimpleExpr;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use std::collections::HashMap;
use std::time::Duration;

const CACHE_TTL_SECONDS: i64 = 30;
const VERSION_WRITE_TIMEOUT: Duration = Duration::from_millis(200);
const CACHE_TIMEOUT: Duration = Duration::from_millis(10);

/// Bump the catalog version after a write so cached discovery pages expire.
pub async fn bump_catalog_version(state: &AppState) {
    let bumped = tokio::time::timeout(
        VERSION_WRITE_TIMEOUT,
        state.redis.incr::<i64, _>("scout:catalog_ver"),
    )
    .await;
    if !matches!(bumped, Ok(Ok(_))) {
        tracing::warn!("Catalog version bump failed; discovery cache may be stale");
    }
}

async fn cache_get(state: &AppState, key: &str) -> Option<String> {
    tokio::time::timeout(CACHE_TIMEOUT, state.redis.get::<Option<String>, _>(key))
        .await
        .ok()?
        .ok()?
}

async fn catalog_version(state: &AppState) -> Option<String> {
    // A failed version read must bypass the cache, never select an older generation.
    let version: Option<String> =
        tokio::time::timeout(CACHE_TIMEOUT, state.redis.get("scout:catalog_ver"))
            .await
            .ok()?
            .ok()?;
    Some(version.unwrap_or_else(|| "0".into()))
}

/// Integer relevance tier: exact substring, full-text, then trigram similarity.
fn tier(query: &str) -> SimpleExpr {
    if query.is_empty() {
        return sea_orm::sea_query::Expr::val(0_i32);
    }
    sea_orm::sea_query::Expr::cust_with_values(
        "CASE WHEN strpos(lower(name), lower($1)) > 0 OR strpos(lower(description), lower($1)) > 0 THEN 0 \
         WHEN search_vector @@ plainto_tsquery('english', $1) THEN 1 \
         WHEN public.similarity(lower(name), lower($1)) > 0.15 OR public.similarity(lower(description), lower($1)) > 0.15 THEN 2 \
         ELSE 3 END",
        [query.to_owned()],
    )
}

#[derive(FromQueryResult)]
struct PageKey {
    id: uuid::Uuid,
    tier: i32,
    created_at: i64,
}

struct ResolvedPage {
    query: String,
    runtime: Option<String>,
    creator: Option<String>,
    mine: bool,
    include_inactive: bool,
    limit: u8,
    cursor: Option<String>,
    filter_hash: String,
}

async fn resolve(
    state: &AppState,
    headers: &HeaderMap,
    request: PageQuery,
) -> ApiResult<ResolvedPage> {
    let owner = identity::authenticate(state, headers, "capabilities:read").await?;
    let query = request.q.unwrap_or_default();
    if query.len() > 200 {
        return Err(ApiError::bad("search_too_long"));
    }
    if request.runtime.as_ref().is_some_and(|runtime| {
        !matches!(
            runtime.as_str(),
            "wasm" | "repository_stack" | "dependency_inventory" | "workflow"
        )
    }) {
        return Err(ApiError::bad("invalid_runtime_filter"));
    }
    if request.include_inactive && !request.mine {
        return Err(ApiError::forbidden());
    }
    let mut creator = request
        .creator_wallet
        .as_deref()
        .map(identity::wallet)
        .transpose()?;
    if request.mine {
        if creator
            .as_ref()
            .is_some_and(|wallet| wallet != &owner.wallet)
        {
            return Err(ApiError::bad("conflicting_creator_filter"));
        }
        creator = Some(owner.wallet);
    }
    let limit = request.limit.unwrap_or(24);
    if !(1..=100).contains(&limit) {
        return Err(ApiError::bad("invalid_page_limit"));
    }
    let filter_hash = hex::encode(identity::hash(&serde_json::to_vec(&(
        &query,
        &request.runtime,
        &creator,
        request.include_inactive,
    ))?));
    Ok(ResolvedPage {
        query,
        runtime: request.runtime,
        creator,
        mine: request.mine,
        include_inactive: request.include_inactive,
        limit,
        cursor: request.cursor,
        filter_hash,
    })
}

async fn fetch_page_keys(
    postgres: &sea_orm::DatabaseConnection,
    req: &ResolvedPage,
) -> ApiResult<(Vec<PageKey>, bool)> {
    let tx = postgres.begin().await?;
    let tier_expr = tier(&req.query);
    let mut keys = Entity::find()
        .select_only()
        .column(Column::Id)
        .column_as(tier_expr.clone(), "tier")
        .column(Column::CreatedAt)
        .filter(tier_expr.clone().lte(2));
    if !req.query.is_empty() {
        // The trigram operator can use GIN; similarity() alone scans every row.
        // SET LOCAL is pinned to this transaction and cannot leak into the pool.
        tx.execute_unprepared("SET LOCAL pg_trgm.similarity_threshold = 0.15")
            .await?;
        let pattern = format!(
            "%{}%",
            req.query
                .to_lowercase()
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_")
        );
        keys = keys.filter(sea_orm::sea_query::Expr::cust_with_values(
            "lower(name) LIKE $1 OR lower(description) LIKE $1 OR search_vector @@ plainto_tsquery('english', $2) OR lower(name) OPERATOR(public.%) lower($2) OR lower(description) OPERATOR(public.%) lower($2)",
            [pattern, req.query.clone()],
        ));
    }
    if !req.include_inactive {
        keys = keys.filter(Column::Active.eq(true));
    }
    if let Some(runtime) = req.runtime.clone() {
        keys = keys.filter(Column::Runtime.eq(runtime));
    }
    if let Some(creator) = req.creator.clone() {
        keys = keys.filter(Column::CreatorWallet.eq(creator));
    }
    if let Some(encoded) = req.cursor.clone() {
        let cursor = Cursor::decode(&encoded, &req.filter_hash)?;
        keys = keys.filter(
            Condition::any()
                .add(tier_expr.clone().gt(cursor.tier))
                .add(
                    Condition::all()
                        .add(tier_expr.clone().eq(cursor.tier))
                        .add(Column::CreatedAt.lt(cursor.created_at)),
                )
                .add(
                    Condition::all()
                        .add(tier_expr.clone().eq(cursor.tier))
                        .add(Column::CreatedAt.eq(cursor.created_at))
                        .add(Column::Id.gt(cursor.id)),
                ),
        );
    }
    let mut keys = keys
        .order_by_asc(tier_expr)
        .order_by_desc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .limit(u64::from(req.limit) + 1)
        .into_model::<PageKey>()
        .all(&tx)
        .await?;
    tx.commit().await?;
    let more = keys.len() > usize::from(req.limit);
    keys.truncate(usize::from(req.limit));
    Ok((keys, more))
}

#[utoipa::path(get,path="/v1/capabilities/page",params(PageQuery),responses((status=200,body=CapabilityPage)))]
/// # Errors
/// Rejects invalid filters, credentials and cursors; inactive listings require an owned query.
pub async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(request): Query<PageQuery>,
) -> ApiResult<Json<CapabilityPage>> {
    let req = resolve(&state, &headers, request).await?;
    let version = catalog_version(&state).await;
    let cache_key = format!(
        "scout:v{}:{}",
        version.as_deref().unwrap_or("unavailable"),
        hex::encode(identity::hash(&serde_json::to_vec(&(
            &req.query,
            &req.runtime,
            &req.creator,
            req.mine,
            req.include_inactive,
            req.limit,
            &req.cursor,
        ))?))
    );
    if version.is_some()
        && let Some(hit) = cache_get(&state, &cache_key).await
        && let Ok(page) = serde_json::from_str::<CapabilityPage>(&hit)
    {
        return Ok(Json(page));
    }
    let (keys, more) = fetch_page_keys(&state.postgres, &req).await?;
    let ids: Vec<uuid::Uuid> = keys.iter().map(|key| key.id).collect();
    let rows = if ids.is_empty() {
        Vec::new()
    } else {
        Entity::find()
            .filter(Column::Id.is_in(ids))
            .all(&state.postgres)
            .await?
    };
    let by_id: HashMap<uuid::Uuid, crate::entities::capabilities::Model> =
        rows.into_iter().map(|row| (row.id, row)).collect();
    let values: Vec<Capability> = keys
        .iter()
        .filter_map(|key| by_id.get(&key.id))
        .cloned()
        .map(Capability::from)
        .collect();
    let next_cursor = if more {
        keys.last()
            .map(|key| {
                Cursor {
                    tier: key.tier,
                    created_at: key.created_at,
                    id: key.id,
                    filter_hash: req.filter_hash.clone(),
                }
                .encode()
            })
            .transpose()?
    } else {
        None
    };
    let page = CapabilityPage {
        items: values,
        next_cursor,
    };
    if version.is_some()
        && let Ok(body) = serde_json::to_string(&page)
    {
        let stored = tokio::time::timeout(
            CACHE_TIMEOUT,
            state.redis.set::<(), _, _>(
                cache_key,
                body,
                Some(Expiration::EX(CACHE_TTL_SECONDS)),
                None,
                false,
            ),
        )
        .await;
        if !matches!(stored, Ok(Ok(()))) {
            tracing::debug!("Discovery page cache write failed");
        }
    }
    Ok(Json(page))
}
