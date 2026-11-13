use crate::{
    config::{Config, Integration},
    platform,
};
use anyhow::{Result, bail, ensure};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};

/// Resolve an existing path or the nearest existing parent of a new output.
/// Canonicalization catches symlinks escaping a permitted folder.
pub fn approved(path: &Path, roots: &[PathBuf]) -> Result<PathBuf> {
    ensure!(
        path.is_absolute() && !path.components().any(|c| matches!(c, Component::ParentDir)),
        "paths must be absolute without parent traversal"
    );
    let mut parent = path.to_owned();
    let mut tail = Vec::new();
    while !parent.exists() {
        // Reject dangling symlinks rather than following a future target.
        ensure!(
            std::fs::symlink_metadata(&parent).is_err(),
            "unresolved symlink"
        );
        tail.push(
            parent
                .file_name()
                .ok_or_else(|| anyhow::anyhow!("path has no parent"))?
                .to_owned(),
        );
        ensure!(parent.pop(), "path has no existing parent");
    }
    let mut resolved = parent.canonicalize()?;
    for component in tail.iter().rev() {
        resolved.push(component);
    }
    ensure!(
        roots.iter().any(|root| root
            .canonicalize()
            .is_ok_and(|root| resolved.starts_with(root))),
        "path is outside approved folders"
    );
    Ok(resolved)
}
fn paths(value: &Value, f: &mut impl FnMut(&str) -> Result<()>) -> Result<()> {
    match value {
        Value::Null => Ok(()),
        Value::String(s) if s.is_empty() => Ok(()),
        Value::String(s) => f(s),
        Value::Array(items) => {
            for item in items {
                paths(item, f)?;
            }
            Ok(())
        }
        _ => bail!("invalid path argument"),
    }
}
fn normalize_paths(value: &mut Value, roots: &[PathBuf], uri: bool) -> Result<()> {
    match value {
        Value::Null => Ok(()),
        Value::String(raw) if raw.is_empty() => Ok(()),
        Value::String(raw) => {
            if uri {
                let url = reqwest::Url::parse(raw)?;
                if matches!(url.scheme(), "https" | "http" | "data") {
                    return Ok(());
                }
                ensure!(url.scheme() == "file", "unsupported document URI");
                let resolved = approved(
                    &url.to_file_path()
                        .map_err(|()| anyhow::anyhow!("invalid file URL"))?,
                    roots,
                )?;
                *raw = reqwest::Url::from_file_path(platform::tool_path(&resolved))
                    .map_err(|()| anyhow::anyhow!("invalid file path"))?
                    .to_string();
            } else {
                *raw = platform::tool_path(&approved(Path::new(raw), roots)?);
            }
            Ok(())
        }
        Value::Array(items) => {
            for item in items {
                normalize_paths(item, roots, uri)?;
            }
            Ok(())
        }
        _ => bail!("invalid path argument"),
    }
}
pub fn arguments(config: &Config, integration: &Integration, args: &mut Value) -> Result<()> {
    ensure!(args.is_object(), "tool arguments must be an object");
    if config.shared {
        for name in &integration.url_arguments {
            paths(&args[name], &mut |raw| {
                let url = reqwest::Url::parse(raw)?;
                ensure!(
                    matches!(url.scheme(), "https" | "data")
                        && url.username().is_empty()
                        && url.password().is_none(),
                    "shared tools accept public HTTPS or data URLs only"
                );
                Ok(())
            })?;
        }
        return Ok(());
    }
    for name in &integration.path_arguments {
        if let Some(value) = args.get_mut(name) {
            // Forward the checked real path, not the caller's alias. Upstream
            // roots use real paths too (/private/var on macOS, long names on Windows).
            normalize_paths(value, &config.roots, name == "uri")?;
        }
    }
    if integration.slug == "serena" {
        // Serena takes project-relative paths; never permit a second project.
        let project = config
            .enabled
            .get("serena")
            .and_then(|a| a.project.as_ref())
            .ok_or_else(|| anyhow::anyhow!("Serena needs an approved project"))?;
        for field in ["relative_path", "path", "file_path"] {
            if let Some(raw) = args[field].as_str() {
                ensure!(
                    !Path::new(raw).is_absolute(),
                    "Serena paths must be relative to its project"
                );
                approved(&project.join(raw), std::slice::from_ref(project))?;
            }
        }
    }
    Ok(())
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    #[test]
    fn normalized_paths_forward_checked_targets() -> Result<()> {
        let root = std::env::temp_dir().join(format!("hive-normalize-test-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let roots = vec![root.clone()];
        let input = root.join("new/note.md");
        let mut paths = serde_json::json!([input, null, ""]);
        normalize_paths(&mut paths, &roots, false)?;
        assert_eq!(paths[0], platform::tool_path(&approved(&input, &roots)?));
        let mut uri = Value::String(
            reqwest::Url::from_file_path(&input)
                .map_err(|()| anyhow::anyhow!("invalid test file URL"))?
                .to_string(),
        );
        normalize_paths(&mut uri, &roots, true)?;
        assert_eq!(
            uri,
            reqwest::Url::from_file_path(platform::tool_path(&approved(&input, &roots)?))
                .map_err(|()| anyhow::anyhow!("invalid expected URL"))?
                .to_string()
        );
        #[cfg(unix)]
        {
            let alias = root.with_extension("alias");
            std::os::unix::fs::symlink(&root, &alias)?;
            let mut value = Value::String(alias.join("new/note.md").to_string_lossy().into_owned());
            normalize_paths(&mut value, &roots, false)?;
            assert_eq!(value, paths[0]);
            std::fs::remove_file(alias)?;
        }
        std::fs::remove_dir(root)?;
        Ok(())
    }
    #[test]
    fn outputs_and_symlinks_stay_inside_roots() -> Result<()> {
        let root = std::env::temp_dir().join(format!("hive-path-test-{}", std::process::id()));
        std::fs::create_dir_all(&root)?;
        let roots = vec![root.clone()];
        assert!(approved(&root.join("new/note.md"), &roots).is_ok());
        assert!(approved(&root.join("../escape"), &roots).is_err());
        assert!(approved(&root.with_extension("sibling"), &roots).is_err());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(std::env::temp_dir(), root.join("escape"))?;
            assert!(approved(&root.join("escape/out"), &roots).is_err());
            std::fs::remove_file(root.join("escape"))?;
        }
        std::fs::remove_dir(root)?;
        Ok(())
    }
}
