//! Pin settlement proofs to a finalized block and a matching canonical receipt.
use super::{hash_string, rpc};
use crate::{
    features::common::error::{ApiError, ApiResult},
    state::AppState,
};
use serde_json::{Value, json};

pub(super) fn quantity(value: &Value) -> ApiResult<u64> {
    value
        .as_str()
        .and_then(|value| value.strip_prefix("0x"))
        .and_then(|value| u64::from_str_radix(value, 16).ok())
        .ok_or_else(|| ApiError::unavailable("invalid_chain_quantity"))
}
pub(super) async fn boundary(state: &AppState) -> ApiResult<Value> {
    let tag = state.settings.runtime.payment.settlement_finality.tag();
    let block = rpc(state, "eth_getBlockByNumber", json!([tag, false])).await?;
    quantity(&block["number"])?;
    if block["hash"].as_str().is_none_or(|hash| !hash_string(hash)) {
        return Err(ApiError::unavailable("invalid_settlement_boundary"));
    }
    Ok(block)
}
pub(super) async fn canonical_receipt(
    state: &AppState,
    log: &Value,
    receipt: &Value,
    boundary: &Value,
) -> ApiResult<bool> {
    let number = quantity(&receipt["blockNumber"])?;
    if number > quantity(&boundary["number"])? {
        return Ok(false);
    }
    if log["blockHash"] != receipt["blockHash"] || log["blockNumber"] != receipt["blockNumber"] {
        return Err(ApiError::unavailable("settlement_receipt_block_mismatch"));
    }
    let hash = receipt["blockHash"]
        .as_str()
        .filter(|hash| hash_string(hash))
        .ok_or_else(|| ApiError::unavailable("invalid_settlement_receipt_hash"))?;
    let block = rpc(
        state,
        "eth_getBlockByNumber",
        json!([receipt["blockNumber"], false]),
    )
    .await?;
    Ok(block["hash"] == hash)
}
