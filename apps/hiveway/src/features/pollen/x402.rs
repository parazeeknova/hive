use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Invocation, PaymentPayload, PaymentRequirements, Resource, Settlement},
        },
        identity,
    },
    state::AppState,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha3::{Digest, Keccak256};

/// # Errors
/// Rejects unsuccessful responses, oversized bodies, malformed JSON, and transport failures.
pub async fn json_response<T: DeserializeOwned>(mut response: reqwest::Response) -> ApiResult<T> {
    if !response.status().is_success() {
        return Err(ApiError::unavailable("payment_provider_rejected_request"));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ApiError::unavailable("payment_provider_unavailable"))?
    {
        if bytes.len() + chunk.len() > 131_072 {
            return Err(ApiError::unavailable("payment_provider_response_too_large"));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|_| ApiError::unavailable("invalid_payment_provider_response"))
}
/// # Errors
/// Rejects unavailable facilitators and unsupported payment networks.
pub async fn supported(state: &AppState) -> ApiResult<()> {
    let response = state
        .http
        .get(format!("{}/supported", state.settings.facilitator_url))
        .send()
        .await
        .map_err(|_| ApiError::unavailable("facilitator_unavailable"))?;
    let data: Value = json_response(response).await?;
    if !data["kinds"].as_array().is_some_and(|kinds| {
        kinds.iter().any(|k| {
            k["x402Version"] == 2
                && k["scheme"] == "exact"
                && k["network"] == state.settings.runtime.payment.network()
        })
    }) {
        return Err(ApiError::unavailable(
            "facilitator_does_not_support_configured_network",
        ));
    }
    Ok(())
}
/// # Errors
/// Returns an error if JSON serialization fails.
pub fn encode<T: serde::Serialize>(value: &T) -> ApiResult<String> {
    Ok(STANDARD.encode(serde_json::to_vec(value)?))
}
/// # Errors
/// Rejects oversized, malformed, or invalidly encoded payment headers.
pub fn decode(value: &str) -> ApiResult<PaymentPayload> {
    if value.len() > 16384 {
        return Err(ApiError::bad("payment_header_too_large"));
    }
    let data = STANDARD
        .decode(value)
        .map_err(|_| ApiError::bad("invalid_payment_header"))?;
    serde_json::from_slice(&data).map_err(|_| ApiError::bad("invalid_payment_payload"))
}
/// # Errors
/// Rejects invalid input, unsupported network bindings, or mismatched payment authorization fields.
pub fn validate(
    payload: &PaymentPayload,
    requirements: &PaymentRequirements,
    resource: &Resource,
    payer: &str,
    expires: i64,
    at: i64,
) -> ApiResult<String> {
    if payload.x402_version != 2
        || payload.accepted != *requirements
        || payload.resource != *resource
    {
        return Err(ApiError::bad("payment_quote_mismatch"));
    }
    let auth = &payload.payload.authorization;
    if identity::wallet(&auth.from)? != payer
        || identity::wallet(&auth.to)? != identity::wallet(&requirements.pay_to)?
        || auth.value != requirements.amount
    {
        return Err(ApiError::bad("payment_authorization_mismatch"));
    }
    let after = auth
        .valid_after
        .parse::<i64>()
        .map_err(|_| ApiError::bad("invalid_payment_time"))?;
    let before = auth
        .valid_before
        .parse::<i64>()
        .map_err(|_| ApiError::bad("invalid_payment_time"))?;
    if after > at || before <= at || before > expires.saturating_add(30) {
        return Err(ApiError::bad("payment_authorization_expired_or_too_long"));
    }
    let nonce = hex::decode(
        auth.nonce
            .strip_prefix("0x")
            .ok_or_else(|| ApiError::bad("invalid_payment_nonce"))?,
    )
    .map_err(|_| ApiError::bad("invalid_payment_nonce"))?;
    if nonce.len() != 32 {
        return Err(ApiError::bad("invalid_payment_nonce"));
    }
    let signature = hex::decode(
        payload
            .payload
            .signature
            .strip_prefix("0x")
            .ok_or_else(|| ApiError::bad("invalid_payment_signature"))?,
    )
    .map_err(|_| ApiError::bad("invalid_payment_signature"))?;
    if signature.len() != 65 {
        return Err(ApiError::bad("eoa_payment_signature_required"));
    }
    Ok(format!(
        "{}:{payer}:{}",
        requirements.network,
        hex::encode(nonce)
    ))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Verification {
    is_valid: bool,
    payer: Option<String>,
}
/// # Errors
/// Rejects unsuccessful facilitator verification or a payer inconsistent with the authorization.
pub async fn verify(
    state: &AppState,
    payload: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> ApiResult<()> {
    let response = state
        .http
        .post(format!("{}/verify", state.settings.facilitator_url))
        .json(&json!({"x402Version":2,"paymentPayload":payload,"paymentRequirements":requirements}))
        .send()
        .await
        .map_err(|_| ApiError::unavailable("facilitator_unavailable"))?;
    let verified: Verification = json_response(response).await?;
    if !verified.is_valid
        || verified
            .payer
            .as_deref()
            .and_then(|p| identity::wallet(p).ok())
            != Some(identity::wallet(&payload.payload.authorization.from)?)
    {
        return Err(ApiError::bad("payment_verification_failed"));
    }
    Ok(())
}
/// # Errors
/// Returns an error if the facilitator request or response fails; failure does not establish that funds stayed put.
pub async fn settle(
    state: &AppState,
    payload: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> ApiResult<Settlement> {
    let response = state
        .http
        .post(format!("{}/settle", state.settings.facilitator_url))
        .json(&json!({"x402Version":2,"paymentPayload":payload,"paymentRequirements":requirements}))
        .send()
        .await
        .map_err(|_| ApiError::unavailable("settlement_outcome_unknown"))?;
    json_response(response).await
}
/// # Errors
/// Rejects unsuccessful, malformed, or oversized JSON-RPC responses and transport failures.
pub async fn rpc(state: &AppState, method: &str, params: Value) -> ApiResult<Value> {
    let response = state
        .http
        .post(&state.settings.rpc_url)
        .json(&json!({"jsonrpc":"2.0","id":1,"method":method,"params":params}))
        .send()
        .await
        .map_err(|_| ApiError::unavailable("chain_rpc_unavailable"))?;
    let body: Value = json_response(response).await?;
    if body.get("error").is_some() || body.get("result").is_none() {
        return Err(ApiError::unavailable("chain_rpc_failed"));
    }
    Ok(body["result"].clone())
}
/// # Errors
/// Rejects unavailable RPC endpoints, invalid responses, and an unexpected chain identifier.
pub async fn block_number(state: &AppState) -> ApiResult<String> {
    let chain = rpc(state, "eth_chainId", json!([])).await?;
    if chain
        .as_str()
        .and_then(|n| u64::from_str_radix(n.trim_start_matches("0x"), 16).ok())
        != Some(state.settings.runtime.payment.chain_id)
    {
        return Err(ApiError::unavailable("wrong_payment_network"));
    }
    rpc(state, "eth_blockNumber", json!([]))
        .await?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| ApiError::unavailable("invalid_block_number"))
}
fn topic(signature: &str) -> String {
    format!("0x{}", hex::encode(Keccak256::digest(signature.as_bytes())))
}
fn address_topic(address: &str) -> ApiResult<String> {
    Ok(format!("0x{:0>64}", &identity::wallet(address)?[2..]))
}
fn hash_string(value: &str) -> bool {
    value.len() == 66
        && value.starts_with("0x")
        && value[2..].bytes().all(|b| b.is_ascii_hexdigit())
}
/// Reconcile the actual token events, including a lost facilitator response.
/// # Errors
/// Rejects malformed recovery data, invalid receipts, and unavailable RPC responses.
pub async fn confirm(
    state: &AppState,
    invocation: &Invocation,
    payload: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> ApiResult<Option<Settlement>> {
    let Some(from_block) = &invocation.settlement_block else {
        return Ok(None);
    };
    let nonce = payload.payload.authorization.nonce.to_lowercase();
    let logs=rpc(state,"eth_getLogs",json!([{"address":state.settings.runtime.payment.asset,"fromBlock":from_block,"toBlock":"latest","topics":[topic("AuthorizationUsed(address,bytes32)"),address_topic(&payload.payload.authorization.from)?,nonce]}])).await?;
    let Some(log) = logs
        .as_array()
        .and_then(|l| l.iter().find(|l| l["removed"] != true))
    else {
        return Ok(None);
    };
    let Some(tx_hash) = log["transactionHash"].as_str().filter(|s| hash_string(s)) else {
        return Err(ApiError::unavailable("invalid_chain_log"));
    };
    let receipt = rpc(state, "eth_getTransactionReceipt", json!([tx_hash])).await?;
    if receipt.is_null() {
        return Ok(None);
    }
    if receipt["status"] != "0x1" {
        return Err(ApiError::unavailable("settlement_transaction_failed"));
    }
    let from = address_topic(&payload.payload.authorization.from)?;
    let to = address_topic(&requirements.pay_to)?;
    let transfer = topic("Transfer(address,address,uint256)");
    let valid = receipt["logs"].as_array().is_some_and(|logs| {
        logs.iter().any(|l| {
            l["address"]
                .as_str()
                .is_some_and(|a| a.eq_ignore_ascii_case(&state.settings.runtime.payment.asset))
                && l["topics"][0] == transfer
                && l["topics"][1] == from
                && l["topics"][2] == to
                && l["data"]
                    .as_str()
                    .and_then(|d| u128::from_str_radix(d.trim_start_matches("0x"), 16).ok())
                    == requirements.amount.parse::<u128>().ok()
        })
    });
    if !valid {
        return Err(ApiError::unavailable("settlement_transfer_mismatch"));
    }
    Ok(Some(Settlement {
        success: true,
        transaction: tx_hash.to_owned(),
        network: state.settings.runtime.payment.network(),
        payer: Some(identity::wallet(&payload.payload.authorization.from)?),
        error_reason: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::{decode, validate};
    use crate::features::common::model::{
        ASSET, NETWORK, PaymentPayload, PaymentRequirements, Resource,
    };
    use serde_json::json;
    #[test]
    fn attacker_cannot_change_quote_or_authorization() -> Result<(), Box<dyn std::error::Error>> {
        let payer = "0x1111111111111111111111111111111111111111";
        let receiver = "0x2222222222222222222222222222222222222222";
        let requirements: PaymentRequirements = serde_json::from_value(
            json!({"scheme":"exact","network":NETWORK,"amount":"10000","asset":ASSET,"payTo":receiver,"maxTimeoutSeconds":120,"extra":{"name":"USDC","version":"2"}}),
        )?;
        let resource: Resource = serde_json::from_value(
            json!({"url":"http://127.0.0.1:3001/v1/invocations/fixture","description":"fixture","mimeType":"application/json"}),
        )?;
        let original = json!({"x402Version":2,"accepted":requirements,"resource":resource,"payload":{"signature":format!("0x{}","00".repeat(65)),"authorization":{"from":payer,"to":receiver,"value":"10000","validAfter":"90","validBefore":"200","nonce":format!("0x{}","11".repeat(32))}}});
        // This checks structure and binding only; cryptographic verification is a separate facilitator operation.
        let payload: PaymentPayload = serde_json::from_value(original.clone())?;
        validate(&payload, &requirements, &resource, payer, 220, 100)?;
        for (path, value) in [
            ("/accepted/amount", json!("1")),
            ("/accepted/network", json!("eip155:1")),
            ("/resource/url", json!("https://evil.example")),
            ("/payload/authorization/from", json!(receiver)),
            ("/payload/authorization/to", json!(payer)),
            ("/payload/authorization/value", json!("999999")),
            ("/payload/authorization/validBefore", json!("999999")),
            ("/payload/authorization/validAfter", json!("999999")),
            ("/payload/authorization/nonce", json!("0x11")),
            ("/payload/signature", json!("0x11")),
        ] {
            let mut modified = original.clone();
            *modified.pointer_mut(path).ok_or("missing fixture path")? = value;
            let payload = serde_json::from_value(modified)?;
            assert!(
                validate(&payload, &requirements, &resource, payer, 220, 100).is_err(),
                "accepted tampering: {path}"
            );
        }
        assert!(decode("invalid base64!").is_err());
        assert!(decode(&"A".repeat(20_000)).is_err());
        Ok(())
    }
}
