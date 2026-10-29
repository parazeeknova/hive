//! Ledger-only creator earnings. Credits are analytics history written at
//! settlement; they never move funds and there is no withdraw endpoint.
use crate::entities::capabilities::{Column as CapabilitiesColumn, Entity as CapabilitiesEntity};
use crate::entities::creator_earnings::{Column as EarningsColumn, Entity as EarningsEntity};
use crate::{
    features::{
        common::{
            error::ApiResult,
            model::{CapabilityEarnings, EarningsSummary, MethodEarnings},
        },
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use sea_orm::sea_query::{Alias, Expr, ExprTrait, Func};
use sea_orm::{ColumnTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(FromQueryResult)]
struct EarningRow {
    method: Option<String>,
    capability_id: Option<Uuid>,
    refunded: bool,
    total: Option<i64>,
    runs: Option<i64>,
}

#[utoipa::path(get,path="/v1/earnings/summary",responses((status=200,body=EarningsSummary)))]
/// # Errors
/// Rejects invalid credentials and propagates database failures.
pub async fn summary(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<EarningsSummary>> {
    let creator = identity::authenticate(&state, &headers, "sales:read").await?;
    let rows = EarningsEntity::find()
        .select_only()
        .column_as(EarningsColumn::PaymentMethod, "method")
        .column_as(EarningsColumn::CapabilityId, "capability_id")
        .column(EarningsColumn::Refunded)
        .column_as(
            Expr::col(EarningsColumn::Amount)
                .sum()
                .cast_as(Alias::new("bigint")),
            "total",
        )
        .column_as(
            Expr::expr(Func::count(Expr::col(EarningsColumn::InvocationId))),
            "runs",
        )
        .filter(EarningsColumn::CreatorWallet.eq(creator.wallet))
        .group_by(EarningsColumn::PaymentMethod)
        .group_by(EarningsColumn::CapabilityId)
        .group_by(EarningsColumn::Refunded)
        .into_model::<EarningRow>()
        .all(&state.postgres)
        .await?;
    let mut methods: HashMap<String, MethodEarnings> = HashMap::new();
    let mut lifetime_amount = 0_i64;
    let mut settled_runs = 0_i64;
    let mut refunded_amount = 0_i64;
    let mut refunded_runs = 0_i64;
    for row in rows {
        let method = row.method.unwrap_or_else(|| "unknown".into());
        let total = row.total.unwrap_or(0);
        let runs = row.runs.unwrap_or(0);
        let entry = methods.entry(method.clone()).or_insert(MethodEarnings {
            method,
            amount: 0,
            runs: 0,
            refunded_amount: 0,
            refunded_runs: 0,
        });
        if row.refunded {
            entry.refunded_amount += total;
            entry.refunded_runs += runs;
            refunded_amount += total;
            refunded_runs += runs;
        } else {
            entry.amount += total;
            entry.runs += runs;
            lifetime_amount += total;
            settled_runs += runs;
        }
    }
    let mut methods: Vec<MethodEarnings> = methods.into_values().collect();
    methods.sort_by(|a, b| a.method.cmp(&b.method));
    Ok(Json(EarningsSummary {
        lifetime_amount,
        settled_runs,
        refunded_amount,
        refunded_runs,
        methods,
    }))
}

#[utoipa::path(get,path="/v1/earnings/by-capability",responses((status=200,body=Vec<CapabilityEarnings>)))]
/// # Errors
/// Rejects invalid credentials and propagates database failures.
pub async fn by_capability(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<Vec<CapabilityEarnings>>> {
    let creator = identity::authenticate(&state, &headers, "sales:read").await?;
    let rows = EarningsEntity::find()
        .select_only()
        .column_as(EarningsColumn::PaymentMethod, "method")
        .column_as(EarningsColumn::CapabilityId, "capability_id")
        .column(EarningsColumn::Refunded)
        .column_as(
            Expr::col(EarningsColumn::Amount)
                .sum()
                .cast_as(Alias::new("bigint")),
            "total",
        )
        .column_as(
            Expr::expr(Func::count(Expr::col(EarningsColumn::InvocationId))),
            "runs",
        )
        .filter(EarningsColumn::CreatorWallet.eq(creator.wallet))
        .group_by(EarningsColumn::PaymentMethod)
        .group_by(EarningsColumn::CapabilityId)
        .group_by(EarningsColumn::Refunded)
        .into_model::<EarningRow>()
        .all(&state.postgres)
        .await?;
    let mut totals: HashMap<Uuid, (i64, i64, i64)> = HashMap::new();
    for row in rows {
        let Some(id) = row.capability_id else {
            continue;
        };
        let entry = totals.entry(id).or_insert((0, 0, 0));
        let total = row.total.unwrap_or(0);
        let runs = row.runs.unwrap_or(0);
        if row.refunded {
            entry.2 += total;
        } else {
            entry.0 += total;
            entry.1 += runs;
        }
    }
    let ids: Vec<Uuid> = totals.keys().copied().collect();
    let names: HashMap<Uuid, String> = if ids.is_empty() {
        HashMap::new()
    } else {
        CapabilitiesEntity::find()
            .select_only()
            .column(CapabilitiesColumn::Id)
            .column(CapabilitiesColumn::Name)
            .filter(CapabilitiesColumn::Id.is_in(ids))
            .into_tuple::<(Uuid, String)>()
            .all(&state.postgres)
            .await?
            .into_iter()
            .collect()
    };
    let mut values: Vec<CapabilityEarnings> = totals
        .into_iter()
        .map(
            |(capability_id, (amount, runs, refunded_amount))| CapabilityEarnings {
                name: names
                    .get(&capability_id)
                    .cloned()
                    .unwrap_or_else(|| "unknown".into()),
                capability_id,
                amount,
                runs,
                refunded_amount,
            },
        )
        .collect();
    values.sort_by_key(|a| std::cmp::Reverse(a.amount));
    Ok(Json(values))
}
