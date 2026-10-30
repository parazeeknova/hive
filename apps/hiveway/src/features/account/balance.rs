use crate::{
    features::{
        common::error::{ApiError, ApiResult},
        identity,
    },
    state::AppState,
};
use axum::{Json, extract::State, http::HeaderMap};
use serde_json::{Value, json};
async fn rpc(state: &AppState, method: &str, params: Value) -> ApiResult<String> {
    let response: Value = state
        .http
        .post(&state.settings.rpc_url)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .send()
        .await
        .map_err(|_| ApiError::unavailable("wallet_rpc_unavailable"))?
        .error_for_status()
        .map_err(|_| ApiError::unavailable("wallet_rpc_unavailable"))?
        .json()
        .await
        .map_err(|_| ApiError::unavailable("invalid_wallet_rpc_response"))?;
    response
        .get("result")
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ApiError::unavailable("invalid_wallet_rpc_response"))
}
fn quantity(hex: &str) -> ApiResult<String> {
    let digits = hex
        .strip_prefix("0x")
        .filter(|v| !v.is_empty() && v.len() <= 64 && v.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| ApiError::unavailable("invalid_wallet_rpc_quantity"))?;
    let digits = digits.trim_start_matches('0');
    if digits.is_empty() {
        return Ok("0".into());
    }
    u128::from_str_radix(digits, 16)
        .map(|v| v.to_string())
        .map_err(|_| ApiError::unavailable("invalid_wallet_rpc_quantity"))
}
/// Read token holdings at a pinned block on the configured network. No ledger credit is fabricated.
/// # Errors
/// Rejects invalid sessions, wrong networks and unavailable or malformed RPC responses.
pub async fn get(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let c = identity::authenticate(&state, &headers, "").await?;
    let payment = &state.settings.runtime.payment;
    let mut result = json!({"wallet":null,"amount":null,"native_amount":null,"block":null,"chain_id":payment.chain_id,"network":payment.network(),"asset":payment.asset,"decimals":payment.decimals,"dodo_test_checkout":state.settings.dodo_api_key.is_some() && state.settings.dodo_product_id.is_some() && state.settings.dodo_webhook_secret.is_some()});
    if c.wallet.starts_with("github:") {
        return Ok(Json(result));
    }
    let wallet = identity::wallet(&c.wallet)?;
    if quantity(&rpc(&state, "eth_chainId", json!([])).await?)? != payment.chain_id.to_string() {
        return Err(ApiError::unavailable("wrong_payment_network"));
    }
    let block = rpc(&state, "eth_blockNumber", json!([])).await?;
    quantity(&block)?;
    let data = format!("0x70a08231000000000000000000000000{}", &wallet[2..]);
    let (amount, native) = tokio::try_join!(
        rpc(
            &state,
            "eth_call",
            json!([{"to":payment.asset,"data":data},block])
        ),
        rpc(&state, "eth_getBalance", json!([wallet, block]))
    )?;
    result["wallet"] = json!(wallet);
    result["amount"] = json!(quantity(&amount)?);
    result["native_amount"] = json!(quantity(&native)?);
    result["block"] = json!(block);
    Ok(Json(result))
}

#[cfg(test)]
mod tests {
    use super::quantity;
    #[test]
    fn rpc_quantities_are_exact_and_malformed_values_never_become_zero() -> anyhow::Result<()> {
        assert_eq!(quantity("0x0")?, "0");
        assert_eq!(quantity(&format!("0x{:064x}", 12_345_678))?, "12345678");
        for invalid in ["", "0x", "0xZZ", "100", "-1"] {
            assert!(quantity(invalid).is_err());
        }
        Ok(())
    }
}
