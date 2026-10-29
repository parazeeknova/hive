use crate::features::common::error::{ApiError, ApiResult};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Cursor {
    pub tier: i32,
    pub created_at: i64,
    pub id: Uuid,
    pub filter_hash: String,
}
impl Cursor {
    pub fn decode(encoded: &str, expected_hash: &str) -> ApiResult<Self> {
        if encoded.len() > 512 {
            return Err(ApiError::bad("invalid_discovery_cursor"));
        }
        let bytes = URL_SAFE_NO_PAD
            .decode(encoded)
            .map_err(|_| ApiError::bad("invalid_discovery_cursor"))?;
        let cursor: Self = serde_json::from_slice(&bytes)
            .map_err(|_| ApiError::bad("invalid_discovery_cursor"))?;
        if cursor.filter_hash != expected_hash {
            return Err(ApiError::bad("discovery_cursor_filter_mismatch"));
        }
        Ok(cursor)
    }
    pub fn encode(&self) -> ApiResult<String> {
        Ok(URL_SAFE_NO_PAD.encode(serde_json::to_vec(self)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cursors_are_bounded_and_bound_to_query_filters() -> anyhow::Result<()> {
        let cursor = Cursor {
            tier: 1,
            created_at: 10,
            id: Uuid::nil(),
            filter_hash: "fixture".into(),
        };
        let encoded = cursor.encode()?;
        assert_eq!(Cursor::decode(&encoded, "fixture")?.id, cursor.id);
        assert_eq!(Cursor::decode(&encoded, "fixture")?.tier, 1);
        assert!(Cursor::decode(&encoded, "different").is_err());
        assert!(Cursor::decode(&"x".repeat(513), "fixture").is_err());
        assert!(Cursor::decode("invalid!", "fixture").is_err());
        Ok(())
    }
}
