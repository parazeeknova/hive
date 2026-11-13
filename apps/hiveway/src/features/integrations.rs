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
    /// `HIVE_SHARED_OWNER_<SLUG>` (slug upper-cased, `-` as `_`) overrides it per deployment.
    #[serde(default)]
    pub shared_owner: Option<String>,
    /// One sentence appended to the MCP session instructions.
    #[serde(default)]
    pub session_hint: String,
    /// Entry-point tools that `search_tools` should rank first for this provider.
    #[serde(default)]
    pub search_tools: Vec<String>,
    /// Whether cached shared results may be reused across callers (`public`)
    /// or are partitioned per caller (`caller`, the safe default).
    #[serde(default)]
    pub cache_scope: CacheScope,
    /// Bring-your-own-key upstream that Hive Local connects to on the user's machine.
    #[serde(default)]
    pub upstream: Option<Upstream>,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum CacheScope {
    Public,
    #[default]
    Caller,
}
#[derive(Clone, Debug, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Upstream {
    pub url: String,
    /// `bearer` sends `Authorization: Bearer <key>`; `header` sends the raw key.
    pub auth: String,
    pub header: String,
    /// Tools the operator verified as read-only even when the upstream omits hints.
    #[serde(default)]
    pub read_only_tools: Vec<String>,
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
        let mut items: Vec<Integration> =
            serde_json::from_str(include_str!("../../integrations.json")).unwrap_or_default();
        for item in &mut items {
            let key = format!(
                "HIVE_SHARED_OWNER_{}",
                item.slug.to_ascii_uppercase().replace('-', "_")
            );
            if let Ok(owner) = std::env::var(key) {
                let owner = owner.trim();
                // An empty override disables sharing for this deployment.
                item.shared_owner = (!owner.is_empty()).then(|| owner.to_owned());
            }
        }
        items
    })
}
/// Operator guidance appended to the MCP session instructions, built from the registry.
#[must_use]
pub fn session_hints() -> String {
    registry()
        .iter()
        .map(|item| item.session_hint.trim())
        .filter(|hint| !hint.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}
/// Whether `tool` is a registered entry point for the `mcp_<slug>__` namespace of `name`.
#[must_use]
pub fn is_search_entry(name: &str) -> bool {
    registry().iter().any(|item| {
        name.strip_prefix(&format!("mcp_{}__", item.slug))
            .is_some_and(|tool| item.search_tools.iter().any(|entry| entry == tool))
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
            assert!(
                item.search_tools
                    .iter()
                    .all(|tool| item.tools.contains(tool)),
                "{} search entry points must be registered tools",
                item.slug
            );
            if let Some(upstream) = &item.upstream {
                assert!(upstream.url.starts_with("https://"), "{}", item.slug);
                assert!(matches!(upstream.auth.as_str(), "bearer" | "header"));
                assert!(
                    upstream
                        .read_only_tools
                        .iter()
                        .all(|tool| item.tools.contains(tool))
                );
            }
        }
        assert!(super::session_hints().contains("templr"));
        assert!(super::is_search_entry("mcp_templr__search_templates"));
        assert!(!super::is_search_entry("mcp_templr__get_preview"));
        Ok(())
    }
    // Asset existence needs the host filesystem, which isolated Miri cannot access.
    #[cfg(not(miri))]
    #[test]
    fn registry_preview_files_exist() {
        for item in registry() {
            for preview in &item.previews {
                let file = format!(
                    "{}/../nectar/public{}",
                    env!("CARGO_MANIFEST_DIR"),
                    preview.src
                );
                assert!(std::path::Path::new(&file).is_file(), "missing {file}");
            }
        }
    }
    #[test]
    fn user_rows_cannot_claim_official_ids() {
        let item = &registry()[0];
        assert!(for_capability(item.id, "github:1").is_none());
        assert!(for_capability(item.id, PUBLISHER).is_some());
    }
}
