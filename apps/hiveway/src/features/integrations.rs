//! Operator-maintained integrations. Publisher verification never comes from user metadata.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const PUBLISHER: &str = "hive:official";
/// Hive mark served by Nectar, shown as the avatar of official integrations.
pub const HIVE_AVATAR: &str = "/hive-mark.png";
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Integration {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub default_tool: String,
    pub tools: Vec<String>,
    pub byok: bool,
    pub guidance: String,
    pub previews: Vec<Preview>,
    pub legacy_ids: Vec<Uuid>,
    /// Operator account whose connection named `slug` serves every caller.
    /// Only tools listed in `tools` and snapshotted as read-only are shared.
    #[serde(default)]
    pub shared_owner: Option<String>,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
pub struct Preview {
    pub slug: String,
    pub title: String,
    pub src: String,
}
/// The compiled-in registry. A malformed file is caught by the unit test below,
/// so production never silently runs without its integrations.
#[must_use]
pub fn registry() -> &'static [Integration] {
    static REGISTRY: std::sync::OnceLock<Vec<Integration>> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        serde_json::from_str(include_str!("../../integrations.json")).unwrap_or_default()
    })
}
/// The shared integration served under `slug`, if the operator enabled sharing.
#[must_use]
pub fn shared(slug: &str) -> Option<&'static Integration> {
    registry()
        .iter()
        .find(|item| item.slug == slug && item.shared_owner.is_some())
}
/// Match only operator-owned rows or the fixed IDs listed in the registry.
#[must_use]
pub fn for_capability(id: Uuid, owner: &str) -> Option<Integration> {
    registry()
        .iter()
        .find(|item| (item.id == id && owner == PUBLISHER) || item.legacy_ids.contains(&id))
        .cloned()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_is_valid() -> Result<(), serde_json::Error> {
        let parsed: Vec<Integration> =
            serde_json::from_str(include_str!("../../integrations.json"))?;
        assert!(!parsed.is_empty());
        assert_eq!(registry().len(), parsed.len());
        let mut slugs = std::collections::HashSet::new();
        for item in &parsed {
            assert!(slugs.insert(item.slug.as_str()), "duplicate {}", item.slug);
            assert!(
                item.slug
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "slug {} must be a valid mcp_ alias label",
                item.slug
            );
            assert!(item.tools.contains(&item.default_tool));
            assert!(
                item.tools
                    .iter()
                    .all(|tool| super::super::connections::valid_name(tool))
            );
            for preview in &item.previews {
                let file = format!(
                    "{}/../nectar/public{}",
                    env!("CARGO_MANIFEST_DIR"),
                    preview.src
                );
                assert!(std::path::Path::new(&file).is_file(), "missing {file}");
            }
        }
        Ok(())
    }
    #[test]
    fn user_rows_cannot_claim_official_ids() {
        let item = &registry()[0];
        assert!(for_capability(item.id, "github:1").is_none());
        assert!(for_capability(item.id, PUBLISHER).is_some());
    }
}
