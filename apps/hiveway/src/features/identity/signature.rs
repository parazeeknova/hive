//! EIP-191 verification for server-issued EIP-4361 challenges. The client can
//! submit only a challenge ID and signature, never a replacement message.
use super::wallet;
use crate::features::common::error::{ApiError, ApiResult};
use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use sha3::{Digest, Keccak256};

pub(super) fn checksum(address: &str) -> ApiResult<String> {
    let address = wallet(address)?;
    let lower = &address[2..];
    let hash = hex::encode(Keccak256::digest(lower.as_bytes()));
    let chars = lower
        .bytes()
        .zip(hash.bytes())
        .map(|(c, h)| {
            if c.is_ascii_alphabetic() && h > b'7' {
                char::from(c.to_ascii_uppercase())
            } else {
                char::from(c)
            }
        })
        .collect::<String>();
    Ok(format!("0x{chars}"))
}
pub(super) fn message(
    settings: &crate::features::common::settings::Settings,
    address: &str,
    statement: &str,
    id: uuid::Uuid,
    at: i64,
    expires: i64,
) -> ApiResult<String> {
    let domain = &settings.auth_domain;
    let origin = &settings.auth_uri;
    let chain_id = settings.runtime.payment.chain_id;
    let date = |t| {
        time::OffsetDateTime::from_unix_timestamp(t)
            .map_err(|_| ApiError::bad("invalid_timestamp"))?
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|_| ApiError::bad("invalid_timestamp"))
    };
    Ok(format!(
        "{domain} wants you to sign in with your Ethereum account:\n{}\n\n{statement}\n\nURI: {origin}\nVersion: 1\nChain ID: {chain_id}\nNonce: {}\nIssued At: {}\nExpiration Time: {}\nRequest ID: {id}",
        checksum(address)?,
        id.simple(),
        date(at)?,
        date(expires)?
    ))
}
#[must_use]
pub fn digest(message: &str) -> [u8; 32] {
    Keccak256::digest(
        format!("\x19Ethereum Signed Message:\n{}{message}", message.len()).as_bytes(),
    )
    .into()
}
/// Recover an EOA address from a canonical Ethereum personal signature.
/// # Errors
/// Rejects malformed signatures, unsupported recovery IDs, and invalid recovery.
pub fn recover(message: &str, signature: &str) -> ApiResult<String> {
    let raw = signature.strip_prefix("0x").unwrap_or(signature);
    if raw.len() != 130 {
        return Err(ApiError::bad("invalid_signature"));
    }
    let bytes = hex::decode(raw).map_err(|_| ApiError::bad("invalid_signature"))?;
    if bytes.len() != 65 {
        return Err(ApiError::bad("invalid_signature"));
    }
    let sig =
        Signature::from_slice(&bytes[..64]).map_err(|_| ApiError::bad("invalid_signature"))?;
    if sig.normalize_s() != sig {
        return Err(ApiError::bad("noncanonical_signature"));
    }
    let v = match bytes[64] {
        0 | 1 => bytes[64],
        27 | 28 => bytes[64] - 27,
        _ => return Err(ApiError::bad("invalid_signature")),
    };
    let recovery = RecoveryId::try_from(v).map_err(|_| ApiError::bad("invalid_signature"))?;
    let key = VerifyingKey::recover_from_prehash(&digest(message), &sig, recovery)
        .map_err(|_| ApiError::bad("invalid_signature"))?;
    let hash = Keccak256::digest(&key.to_sec1_point(false).as_bytes()[1..]);
    Ok(format!("0x{}", hex::encode(&hash[12..])))
}
#[cfg(test)]
mod tests {
    #[test]
    fn checksum_matches_eip55_vector() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            super::checksum("0x52908400098527886e0f7030069857d2e4169ee7")?,
            "0x52908400098527886E0F7030069857D2E4169EE7"
        );
        assert!(super::recover("message", "0x00").is_err());
        Ok(())
    }
    #[test]
    fn recovery_binds_exact_message() -> Result<(), Box<dyn std::error::Error>> {
        let key = k256::ecdsa::SigningKey::from_slice(&[1; 32])?;
        let (sig, id) = key.sign_prehash_recoverable(&super::digest("Hive credential"));
        let mut bytes = sig.to_bytes().to_vec();
        bytes.push(id.to_byte() + 27);
        let signature = hex::encode(bytes);
        let recovered = super::recover("Hive credential", &signature)?;
        assert_eq!(recovered, "0x1a642f0e3c3af545e7acbd38b07251b3990914f1");
        assert_ne!(super::recover("changed budget", &signature)?, recovered);
        Ok(())
    }
}
