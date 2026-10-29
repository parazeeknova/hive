use crate::{
    entities::wasm_artifacts::{self, Column, Entity},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::now,
        },
        identity,
    },
    state::AppState,
};
use sea_orm::sea_query::{Alias, Func, Query};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Set,
    TransactionTrait,
};
use serde_json::Value;
use uuid::Uuid;

pub(super) async fn insert(
    state: &AppState,
    wallet: &str,
    bytes: Vec<u8>,
    input: Value,
    output: Value,
) -> ApiResult<wasm_artifacts::Model> {
    let fingerprint = serde_json::to_vec(&(&bytes, &input, &output))?;
    let digest = identity::hash(&fingerprint);
    let tx = state.postgres.begin().await?;
    let lock_hash = identity::hash(format!("hive:artifacts:{wallet}").as_bytes());
    let key = i64::from_be_bytes(
        lock_hash[..8]
            .try_into()
            .map_err(|_| ApiError::bad("invalid_lock_key"))?,
    );
    tx.query_one(
        &Query::select()
            .expr(Func::cust(Alias::new("pg_advisory_xact_lock")).arg(key))
            .to_owned(),
    )
    .await?;
    let owned = Entity::find().filter(Column::OwnerWallet.eq(wallet));
    if let Some(existing) = owned
        .clone()
        .filter(Column::Digest.eq(digest.clone()))
        .one(&tx)
        .await?
    {
        tx.commit().await?;
        return Ok(existing);
    }
    if owned.count(&tx).await? >= state.settings.runtime.sandbox.artifacts_per_wallet {
        return Err(ApiError::bad("artifact_quota_exhausted"));
    }
    let artifact = wasm_artifacts::ActiveModel {
        id: Set(Uuid::new_v4()),
        owner_wallet: Set(wallet.into()),
        digest: Set(digest),
        wasm: Set(bytes),
        input_schema: Set(input),
        output_schema: Set(output),
        created_at: Set(now()),
    }
    .insert(&tx)
    .await?;
    tx.commit().await?;
    Ok(artifact)
}
