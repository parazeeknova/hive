//! Persistent account identity and authenticated, read-only wallet information.
pub mod balance;
pub mod credentials;
use crate::{
    entities::account_profiles::{self, Entity},
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        identity,
    },
    state::AppState,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use base64::{Engine, engine::general_purpose::STANDARD};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub username: String,
    pub avatar: String,
}
impl From<account_profiles::Model> for Profile {
    fn from(row: account_profiles::Model) -> Self {
        Self {
            name: row.name,
            username: row.username,
            avatar: row.avatar,
        }
    }
}
fn validate(profile: &Profile) -> ApiResult<()> {
    if profile.name.trim().is_empty()
        || profile.name.chars().count() > 60
        || !(3..=30).contains(&profile.username.len())
        || !profile
            .username
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"_-".contains(&c))
    {
        return Err(ApiError::bad("invalid_profile"));
    }
    if profile.avatar.is_empty() {
        return Ok(());
    }
    if let Ok(url) = reqwest::Url::parse(&profile.avatar)
        && url.scheme() == "https"
        && url.host_str() == Some("avatars.githubusercontent.com")
        && url.port().is_none()
        && url.username().is_empty()
        && url.password().is_none()
        && url
            .path()
            .strip_prefix("/u/")
            .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
        && url.query() == Some("v=4")
        && url.fragment().is_none()
    {
        return Ok(());
    }
    let (prefix, data) = profile
        .avatar
        .split_once(',')
        .ok_or_else(|| ApiError::bad("invalid_avatar"))?;
    if data.len() > 90_000 {
        return Err(ApiError::bad("avatar_too_large"));
    }
    let bytes = STANDARD
        .decode(data)
        .map_err(|_| ApiError::bad("invalid_avatar"))?;
    let valid = match prefix {
        "data:image/png;base64" => bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "data:image/jpeg;base64" => bytes.starts_with(&[0xff, 0xd8, 0xff]),
        "data:image/webp;base64" => bytes.starts_with(b"RIFF") && bytes.get(8..12) == Some(b"WEBP"),
        _ => false,
    };
    if !valid {
        return Err(ApiError::bad("invalid_avatar"));
    }
    Ok(())
}
async fn ensure(state: &AppState, c: &Credential) -> ApiResult<account_profiles::Model> {
    if let Some(row) = Entity::find_by_id(c.wallet.clone())
        .one(&state.postgres)
        .await?
    {
        return Ok(row);
    }
    // Deterministic fallback avoids collisions between usernames from different identity providers.
    let fallback = format!(
        "member-{}",
        &hex::encode(identity::hash(c.wallet.as_bytes()))[..20]
    );
    let preferred = c.name.to_ascii_lowercase();
    let username = if (3..=30).contains(&preferred.len())
        && preferred
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b"_-".contains(&b))
    {
        preferred
    } else {
        fallback.clone()
    };
    for username in [username, fallback] {
        Entity::insert(account_profiles::ActiveModel {
            owner: Set(c.wallet.clone()),
            name: Set(c.name.chars().take(60).collect()),
            username: Set(username),
            avatar: Set(c
                .wallet
                .strip_prefix("github:")
                .map_or_else(String::new, |id| {
                    format!("https://avatars.githubusercontent.com/u/{id}?v=4")
                })),
            updated_at: Set(now()),
        })
        .on_conflict(
            sea_orm::sea_query::OnConflict::new()
                .do_nothing()
                .to_owned(),
        )
        .exec_without_returning(&state.postgres)
        .await?;
        if let Some(row) = Entity::find_by_id(c.wallet.clone())
            .one(&state.postgres)
            .await?
        {
            return Ok(row);
        }
    }
    Err(ApiError::unavailable("profile_creation_failed"))
}
/// Read the profile shared by all sessions of the same authenticated identity.
/// # Errors
/// Rejects invalid credentials and database failures.
pub async fn get(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<Profile>> {
    let c = identity::authenticate(&state, &headers, "").await?;
    Ok(Json(ensure(&state, &c).await?.into()))
}
/// Persist an owned profile; identity is always selected from the credential.
/// # Errors
/// Rejects invalid fields, conflicting usernames and database failures.
pub async fn save(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(mut profile): Json<Profile>,
) -> ApiResult<Json<Profile>> {
    let c = identity::authenticate(&state, &headers, "").await?;
    if crate::entities::account_api_keys::Entity::find_by_id(c.id)
        .one(&state.postgres)
        .await?
        .is_some()
    {
        return Err(ApiError::forbidden());
    }
    profile.name = profile.name.trim().to_owned();
    validate(&profile)?;
    let row = ensure(&state, &c).await?;
    let mut model: account_profiles::ActiveModel = row.into();
    model.name = Set(profile.name);
    model.username = Set(profile.username);
    model.avatar = Set(profile.avatar);
    model.updated_at = Set(now());
    let saved = model.update(&state.postgres).await.map_err(|error| {
        if matches!(
            error.sql_err(),
            Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
        ) {
            ApiError::new(StatusCode::CONFLICT, "username_already_taken")
        } else {
            error.into()
        }
    })?;
    Ok(Json(saved.into()))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_invalid_identity_and_active_image_content() {
        for (username, avatar) in [
            ("UPPER", ""),
            ("ab", ""),
            ("valid", "data:image/svg+xml;base64,PHN2Zz4="),
            ("valid", "data:image/png;base64,aGVsbG8="),
        ] {
            assert!(
                validate(&Profile {
                    name: "Member".into(),
                    username: username.into(),
                    avatar: avatar.into()
                })
                .is_err()
            );
        }
        assert!(
            validate(&Profile {
                name: "Member".into(),
                username: "valid-user".into(),
                avatar: String::new()
            })
            .is_ok()
        );
    }
}
