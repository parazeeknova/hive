//! Dynamic public GitHub starters. Local clients perform writes; Hive never executes repository code.
use crate::{
    features::{
        cell,
        common::{
            error::{ApiError, ApiResult},
            model::RepositoryInput,
        },
    },
    state::AppState,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

const MAX_ARCHIVE_BYTES: usize = 8 * 1024 * 1024;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    project_name: String,
    repository_url: String,
    revision: String,
}
fn source(args: Value) -> ApiResult<(Request, String)> {
    let request: Request =
        serde_json::from_value(args).map_err(|_| ApiError::bad("invalid_scaffold_arguments"))?;
    let name = &request.project_name;
    if name.is_empty()
        || name.len() > 63
        || !name.as_bytes()[0].is_ascii_alphanumeric()
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err(ApiError::bad("invalid_project_name"));
    }
    let (owner, repo) = cell::validate(&RepositoryInput {
        repository_url: request.repository_url.clone(),
        revision: request.revision.clone(),
    })?;
    let url = format!(
        "https://codeload.github.com/{owner}/{repo}/tar.gz/{}",
        request.revision
    );
    Ok((request, url))
}
/// Verify a dynamic pinned source and return an authenticated-client install plan.
/// # Errors
/// Rejects unsafe names, non-public-GitHub URLs, non-commit revisions and failed/oversized archives.
pub async fn plan(state: &AppState, args: Value) -> ApiResult<Value> {
    let (request, url) = source(args)?;
    // The shared client disables redirects. No Hive credentials are sent to GitHub.
    let mut response = state
        .http
        .get(&url)
        .send()
        .await
        .map_err(|_| ApiError::unavailable("template_download_failed"))?;
    if !response.status().is_success() {
        return Err(ApiError::unavailable(
            "template_unavailable_check_repository_and_commit",
        ));
    }
    let mut size = 0;
    let mut hash = Sha256::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| ApiError::unavailable("template_download_failed"))?
    {
        size += chunk.len();
        if size > MAX_ARCHIVE_BYTES {
            return Err(ApiError::bad("template_archive_too_large"));
        }
        hash.update(&chunk);
    }
    if size == 0 {
        return Err(ApiError::unavailable("template_archive_empty"));
    }
    Ok(json!({
        "project_name":request.project_name, "price":"0",
        "source_repository":request.repository_url, "revision":request.revision,
        "archive_sha256":hex::encode(hash.finalize()), "archive_bytes":size,
        "archive_url":url, "strip_components":1,
        "limits":{"archive_bytes":MAX_ARCHIVE_BYTES,"files":2000,"expanded_bytes":33_554_432},
        "instructions":"Download the public archive without sending your Hive API key to GitHub. Verify archive_sha256, then validate every tar entry before writing. Strip the single GitHub archive root directory. Reject absolute paths, traversal, .git entries, symlinks, hardlinks, duplicate paths and oversized contents. Extract regular files only into a new directory named project_name; never overwrite an existing directory. Preserve executable modes and original package/workspace names. Use local filesystem MCP or shell tools; this tool has not created files or a GitHub repository. Read the source README to determine its prerequisites and setup commands. Never execute template scripts or install dependencies automatically.",
        "next_commands":[["git","init"]],
        "github_repository":"Optional: use your local authenticated GitHub CLI or GitHub MCP to create/push the repository. Hive login does not grant repository creation permissions."
    }))
}

#[cfg(test)]
mod tests {
    #[test]
    fn sources_are_dynamic_and_inputs_are_bounded() -> anyhow::Result<()> {
        let args = serde_json::json!({"project_name":"my-app","repository_url":"https://github.com/another/starter","revision":"a".repeat(40)});
        let (_, url) = super::source(args.clone())?;
        assert_eq!(
            url,
            format!(
                "https://codeload.github.com/another/starter/tar.gz/{}",
                "a".repeat(40)
            )
        );
        for name in ["", "../bad", "/tmp/x", "a;touch x", ".git"] {
            let mut bad = args.clone();
            bad["project_name"] = serde_json::json!(name);
            assert!(super::source(bad).is_err());
        }
        for repository in [
            "https://evil.example/a/b",
            "https://github.com/a/b?x=1",
            "http://127.0.0.1/a/b",
        ] {
            let mut bad = args.clone();
            bad["repository_url"] = serde_json::json!(repository);
            assert!(super::source(bad).is_err());
        }
        let mut bad = args;
        bad["revision"] = serde_json::json!("main");
        assert!(super::source(bad).is_err());
        Ok(())
    }
}
