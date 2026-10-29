//! Keyset pagination preserves the original array discovery endpoint.
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
use model::{CapabilityPage, PageQuery};
use sea_orm::{ColumnTrait, Condition, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

#[utoipa::path(get,path="/v1/capabilities/page",params(PageQuery),responses((status=200,body=CapabilityPage)))]
/// # Errors
/// Rejects invalid filters, credentials and cursors; inactive listings require an owned query.
pub async fn page(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(request): Query<PageQuery>,
) -> ApiResult<Json<CapabilityPage>> {
    let owner = identity::authenticate(&state, &headers, "capabilities:read").await?;
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
    let mut selection = Entity::find().filter(super::matching(&query));
    if !request.include_inactive {
        selection = selection.filter(Column::Active.eq(true));
    }
    if let Some(runtime) = request.runtime {
        selection = selection.filter(Column::Runtime.eq(runtime));
    }
    if let Some(creator) = creator {
        selection = selection.filter(Column::CreatorWallet.eq(creator));
    }
    if let Some(encoded) = request.cursor {
        let cursor = Cursor::decode(&encoded, &filter_hash)?;
        selection = selection.filter(
            Condition::any()
                .add(Column::CreatedAt.lt(cursor.created_at))
                .add(
                    Condition::all()
                        .add(Column::CreatedAt.eq(cursor.created_at))
                        .add(Column::Id.gt(cursor.id)),
                ),
        );
    }
    let mut values = selection
        .order_by_desc(Column::CreatedAt)
        .order_by_asc(Column::Id)
        .limit(u64::from(limit) + 1)
        .all(&state.postgres)
        .await?;
    let more = values.len() > usize::from(limit);
    values.truncate(usize::from(limit));
    let next_cursor = if more {
        values
            .last()
            .map(|row| {
                Cursor {
                    created_at: row.created_at,
                    id: row.id,
                    filter_hash,
                }
                .encode()
            })
            .transpose()?
    } else {
        None
    };
    Ok(Json(CapabilityPage {
        items: values.into_iter().map(Capability::from).collect(),
        next_cursor,
    }))
}
