use crate::{
    features::common::{
        error::{ApiError, ApiResult},
        model::RepositoryInput,
    },
    state::AppState,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use sea_orm::FromQueryResult;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use utoipa::ToSchema;

/// # Errors
/// Rejects non-GitHub URLs, unsafe path components, and moving or malformed revisions.
pub fn validate(input: &RepositoryInput) -> ApiResult<(String, String)> {
    let url = reqwest::Url::parse(&input.repository_url)
        .map_err(|_| ApiError::bad("invalid_repository_url"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("github.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ApiError::bad("public_github_repository_required"));
    }
    let parts: Vec<_> = url.path().trim_matches('/').split('/').collect();
    if parts.len() != 2
        || parts.iter().any(|p| {
            p.is_empty()
                || p.len() > 100
                || !p
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
        || parts.contains(&".")
        || parts.contains(&"..")
    {
        return Err(ApiError::bad("invalid_repository_path"));
    }
    if input.revision.len() != 40 || !input.revision.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::bad("full_commit_sha_required"));
    }
    Ok((parts[0].to_owned(), parts[1].to_owned()))
}
#[derive(Deserialize)]
struct Tree {
    tree: Vec<TreeEntry>,
    truncated: bool,
}
#[derive(Deserialize)]
struct TreeEntry {
    path: String,
    sha: String,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Deserialize)]
struct Blob {
    content: String,
    encoding: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct RepositoryReport {
    pub repository_url: String,
    pub revision: String,
    pub manifests: Vec<String>,
    pub technologies: Vec<String>,
    pub dependencies: Vec<Dependency>,
    pub analysis_limit_reached: bool,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct Dependency {
    pub manifest: String,
    pub ecosystem: String,
    pub name: String,
    pub requirement: String,
}
fn dependencies(path: &str, text: &str) -> Vec<Dependency> {
    let mut output = Vec::new();
    if path.ends_with("package.json") {
        if let Ok(value) = serde_json::from_str::<Value>(text) {
            for key in ["dependencies", "devDependencies", "peerDependencies"] {
                if let Some(deps) = value[key].as_object() {
                    for (name, version) in deps {
                        if let Some(version) = version.as_str() {
                            output.push(Dependency {
                                manifest: path.into(),
                                ecosystem: "npm".into(),
                                name: name.clone(),
                                requirement: version.into(),
                            });
                        }
                    }
                }
            }
        }
    } else if path.ends_with("Cargo.toml")
        && let Ok(value) = toml::from_str::<toml::Value>(text)
    {
        for deps in [
            value.get("dependencies"),
            value.get("dev-dependencies"),
            value.get("workspace").and_then(|v| v.get("dependencies")),
        ]
        .into_iter()
        .flatten()
        {
            if let Some(deps) = deps.as_table() {
                for (name, version) in deps {
                    output.push(Dependency {
                        manifest: path.into(),
                        ecosystem: "cargo".into(),
                        name: name.clone(),
                        requirement: version
                            .as_str()
                            .map_or_else(|| version.to_string(), str::to_owned),
                    });
                }
            }
        }
    }
    output
}
fn output(runtime: &str, report: &Value) -> ApiResult<Value> {
    match runtime {
        "repository_stack" => Ok(
            json!({"repository_url":report["repository_url"],"revision":report["revision"],"manifests":report["manifests"],"technologies":report["technologies"],"analysis_limit_reached":report["analysis_limit_reached"]}),
        ),
        "dependency_inventory" => Ok(
            json!({"repository_url":report["repository_url"],"revision":report["revision"],"dependencies":report["dependencies"],"analysis_limit_reached":report["analysis_limit_reached"]}),
        ),
        _ => Err(ApiError::bad("unsupported_runtime")),
    }
}

async fn github_json<T: serde::de::DeserializeOwned>(
    state: &AppState,
    path: &str,
    limit: usize,
) -> ApiResult<T> {
    let request = state
        .http
        .get(format!("https://api.github.com{path}"))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");
    let mut response = request
        .send()
        .await
        .map_err(|_| ApiError::unavailable("github_unavailable"))?;
    if !response.status().is_success() {
        return Err(ApiError::unavailable("github_request_failed"));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ApiError::unavailable("github_response_failed"))?
    {
        if body.len() + chunk.len() > limit {
            return Err(ApiError::bad("repository_exceeds_analysis_limits"));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| ApiError::unavailable("invalid_github_response"))
}
fn manifest(path: &str) -> bool {
    matches!(
        path.rsplit('/').next(),
        Some(
            "package.json"
                | "Cargo.toml"
                | "pyproject.toml"
                | "requirements.txt"
                | "go.mod"
                | "compose.yaml"
                | "docker-compose.yml"
                | "Dockerfile"
        )
    )
}
fn inspect(path: &str, content: &str, found: &mut BTreeSet<String>) {
    let name = path.rsplit('/').next().unwrap_or_default();
    match name {
        "package.json" => {
            found.insert("JavaScript/TypeScript".into());
            if let Ok(value) = serde_json::from_str::<Value>(content) {
                for key in ["dependencies", "devDependencies"] {
                    if let Some(deps) = value[key].as_object() {
                        for (package, tech) in [
                            ("next", "Next.js"),
                            ("react", "React"),
                            ("typescript", "TypeScript"),
                            ("expo", "Expo"),
                            ("pg", "PostgreSQL client"),
                            ("redis", "Redis client"),
                        ] {
                            if deps.contains_key(package) {
                                found.insert(tech.into());
                            }
                        }
                    }
                }
            }
        }
        "Cargo.toml" => {
            found.insert("Rust".into());
            if let Ok(value) = toml::from_str::<toml::Value>(content) {
                for deps in [
                    value.get("dependencies"),
                    value.get("workspace").and_then(|v| v.get("dependencies")),
                ]
                .into_iter()
                .flatten()
                {
                    for (package, tech) in [
                        ("axum", "Axum"),
                        ("tokio", "Tokio"),
                        ("sqlx", "SQLx"),
                        ("sea-orm", "SeaORM"),
                        ("fred", "Fred/Redis"),
                        ("rig-core", "Rig"),
                    ] {
                        if deps.get(package).is_some() {
                            found.insert(tech.into());
                        }
                    }
                }
            }
        }
        "pyproject.toml" | "requirements.txt" => {
            found.insert("Python".into());
        }
        "go.mod" => {
            found.insert("Go".into());
        }
        "Dockerfile" => {
            found.insert("Docker".into());
        }
        "compose.yaml" | "docker-compose.yml" => {
            found.insert("Docker Compose".into());
        }
        _ => {}
    }
}
/// Execute only bounded manifest inspection. No repository code is run.
/// # Errors
/// Rejects invalid repositories, unsuccessful upstream requests, and oversized or malformed manifests.
pub async fn execute(state: &AppState, input: &RepositoryInput) -> ApiResult<Value> {
    let (owner, repo) = validate(input)?;
    let tree: Tree = github_json(
        state,
        &format!(
            "/repos/{owner}/{repo}/git/trees/{}?recursive=1",
            input.revision
        ),
        2_000_000,
    )
    .await?;
    if tree.truncated {
        return Err(ApiError::bad("repository_tree_truncated"));
    }
    let mut entries: Vec<_> = tree
        .tree
        .into_iter()
        .filter(|e| e.kind == "blob" && manifest(&e.path))
        .collect();
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    let limited = entries.len() > 20;
    let mut paths = Vec::new();
    let mut found = BTreeSet::new();
    let mut inventory = Vec::new();
    for entry in entries.into_iter().take(20) {
        if entry.sha.len() != 40 || !entry.sha.bytes().all(|c| c.is_ascii_hexdigit()) {
            return Err(ApiError::unavailable("invalid_github_blob"));
        }
        let blob: Blob = github_json(
            state,
            &format!("/repos/{owner}/{repo}/git/blobs/{}", entry.sha),
            180_000,
        )
        .await?;
        if blob.encoding != "base64" {
            return Err(ApiError::unavailable("unsupported_blob_encoding"));
        }
        let content = STANDARD
            .decode(blob.content.replace('\n', ""))
            .map_err(|_| ApiError::unavailable("invalid_blob_content"))?;
        if content.len() > 128_000 {
            return Err(ApiError::bad("manifest_too_large"));
        }
        let text = String::from_utf8(content).map_err(|_| ApiError::bad("non_utf8_manifest"))?;
        inspect(&entry.path, &text, &mut found);
        inventory.extend(dependencies(&entry.path, &text));
        paths.push(entry.path);
    }
    Ok(json!(RepositoryReport {
        repository_url: input.repository_url.clone(),
        revision: input.revision.clone(),
        manifests: paths,
        technologies: found.into_iter().collect(),
        dependencies: inventory,
        analysis_limit_reached: limited
    }))
}

/// # Errors
/// Rejects invalid runtimes and workflow definitions and propagates database or repository failures.
pub async fn execute_capability(
    state: &AppState,
    id: uuid::Uuid,
    input: &RepositoryInput,
) -> ApiResult<Value> {
    use crate::features::common::model::Capability;
    let capability = Capability::find_by_statement(sea_orm::Statement::from_sql_and_values(
        sea_orm::DbBackend::Postgres,
        "SELECT * FROM capabilities WHERE id=$1",
        vec![(id).into()],
    ))
    .one(&state.postgres)
    .await?
    .ok_or_else(|| sea_orm::DbErr::RecordNotFound("required row missing".into()))?;
    let report = execute(state, input).await?;
    match capability.runtime.as_str() {
        "repository_stack" | "dependency_inventory" => output(&capability.runtime, &report),
        "workflow" => {
            let ids: Vec<uuid::Uuid> =
                serde_json::from_value(capability.definition["steps"].clone())?;
            let steps = Capability::find_by_statement(sea_orm::Statement::from_sql_and_values(
                sea_orm::DbBackend::Postgres,
                "SELECT * FROM capabilities WHERE id=ANY($1)",
                vec![ids.clone().into()],
            ))
            .all(&state.postgres)
            .await?;
            if steps.len() != ids.len()
                || steps.iter().any(|s| {
                    s.creator_wallet != capability.creator_wallet || s.runtime == "workflow"
                })
            {
                return Err(ApiError::bad("invalid_workflow_definition"));
            }
            let mut outputs = Vec::new();
            for id in ids {
                let step = steps
                    .iter()
                    .find(|s| s.id == id)
                    .ok_or_else(|| ApiError::bad("missing_workflow_step"))?;
                outputs.push(json!({"capability_id":id,"version":step.version,"runtime":step.runtime,"result":output(&step.runtime,&report)?}));
            }
            Ok(
                json!({"repository_url":input.repository_url,"revision":input.revision,"steps":outputs}),
            )
        }
        _ => Err(ApiError::bad("unsupported_runtime")),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ssrf_payloads_never_select_an_outbound_host() {
        for url in [
            "http://169.254.169.254/latest/meta-data/",
            "https://127.0.0.1/a/b",
            "https://[::1]/a/b",
            "https://github.com.evil.example/a/b",
            "https://github.com%2eevil.example/a/b",
            "https://github.com/a/%2e%2e/b",
            "https://github.com/a/b?url=http://127.0.0.1",
            "https://user:pass@github.com/a/b",
            "https://github.com:444/a/b",
            "file:///etc/passwd",
        ] {
            assert!(
                validate(&RepositoryInput {
                    repository_url: url.into(),
                    revision: "a".repeat(40)
                })
                .is_err(),
                "accepted SSRF input: {url}"
            );
        }
    }
    #[test]
    fn rejects_private_hosts_and_moving_revisions() {
        for url in [
            "http://127.0.0.1/a/b",
            "https://github.com@evil.test/a/b",
            "https://github.com/a/b?ref=x",
            "https://github.com/a/../b",
        ] {
            let input = RepositoryInput {
                repository_url: url.into(),
                revision: "a".repeat(40),
            };
            assert!(validate(&input).is_err());
        }
        assert!(
            validate(&RepositoryInput {
                repository_url: "https://github.com/owner/repo".into(),
                revision: "main".into()
            })
            .is_err()
        );
    }
    #[test]
    fn detects_only_declared_package_names() {
        let mut found = BTreeSet::new();
        inspect(
            "package.json",
            r#"{"dependencies":{"next":"16","react":"19"},"description":"postgres redis"}"#,
            &mut found,
        );
        assert!(found.contains("Next.js"));
        assert!(!found.contains("Redis client"));
    }
}
