use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Launch {
    pub command: String,
    pub package: String,
    pub version: String,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub runner_args: Vec<String>,
    pub args: Vec<String>,
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub prepare: Vec<Vec<String>>,
}
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Integration {
    pub slug: String,
    pub tools: Vec<String>,
    pub local: Option<Launch>,
    pub shared_launch: Option<Launch>,
    #[serde(default)]
    pub annotations: BTreeMap<String, Value>,
    #[serde(default)]
    pub shared_annotations: BTreeMap<String, Value>,
    #[serde(default)]
    pub path_arguments: Vec<String>,
    #[serde(default)]
    pub url_arguments: Vec<String>,
}
#[derive(Clone, Deserialize)]
pub struct Catalog {
    pub integrations: Vec<Integration>,
    pub connectors: Limits,
}
#[derive(Clone, Deserialize)]
pub struct Limits {
    pub argument_bytes: usize,
    pub result_bytes: usize,
    pub poll_wait_seconds: u64,
    pub job_ttl_seconds: u64,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Approval {
    pub command: String,
    pub package: String,
    #[serde(default)]
    pub project: Option<PathBuf>,
    #[serde(default)]
    pub database: Option<PathBuf>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub hive: String,
    pub key_file: PathBuf,
    pub roots: Vec<PathBuf>,
    #[serde(default)]
    pub enabled: BTreeMap<String, Approval>,
    /// Shared mode must run inside the supplied network/filesystem sandbox.
    #[serde(default)]
    pub shared: bool,
}
impl Config {
    pub fn read(path: &Path) -> Result<Self> {
        let config: Self = serde_json::from_slice(
            &std::fs::read(path).context("reading Hive Local configuration")?,
        )?;
        validate_url(&config.hive)?;
        ensure!(config.key_file.is_absolute(), "key_file must be absolute");
        #[cfg(unix)]
        if config.shared {
            use std::os::unix::fs::MetadataExt;
            let metadata = std::fs::metadata(&config.key_file)?;
            ensure!(
                metadata.uid() == 0 && metadata.mode() & 0o777 == 0o600,
                "shared connector key must be root-owned with mode 0600"
            );
        }
        for root in &config.roots {
            ensure!(
                root.is_absolute() && root.is_dir(),
                "each root must be an existing absolute directory"
            );
            ensure!(
                root.parent().is_some(),
                "the filesystem root cannot be approved"
            );
            let canonical = root.canonicalize()?;
            ensure!(
                !config.key_file.canonicalize()?.starts_with(canonical),
                "the Hive key must be outside approved folders"
            );
            ensure!(
                !path.canonicalize()?.starts_with(root.canonicalize()?),
                "the helper configuration must be outside approved folders"
            );
        }
        Ok(config)
    }
    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
            options.mode(0o600);
            if path.exists() {
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
            }
        }
        options
            .open(path)?
            .write_all(&serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}
pub fn validate_url(raw: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(raw)?;
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "Hive URL cannot contain credentials"
    );
    ensure!(
        url.scheme() == "https"
            || (url.scheme() == "http"
                && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))),
        "Hive requires HTTPS outside loopback"
    );
    Ok(url)
}
impl Launch {
    pub fn validate(&self, approval: &Approval) -> Result<()> {
        ensure!(
            matches!(self.command.as_str(), "uvx" | "npx"),
            "only uvx and npx package runners are supported"
        );
        ensure!(
            self.command == approval.command && self.package == approval.package,
            "package identity changed; enable the integration again to approve it"
        );
        ensure!(
            !self.package.is_empty()
                && !self.package.starts_with('-')
                && self
                    .package
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"@/_-.".contains(&c)),
            "invalid package name"
        );
        ensure!(
            !self.version.is_empty()
                && self
                    .version
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b".-".contains(&c)),
            "an exact package version is required"
        );
        if let Some(entry) = &self.entry {
            ensure!(
                !entry.starts_with('-')
                    && entry
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c)),
                "invalid executable name"
            );
        }
        for prepare in &self.prepare {
            if !prepare
                .first()
                .is_some_and(|p| matches!(p.as_str(), "uvx" | "npx"))
            {
                bail!("preparation must use a package runner");
            }
            // Preparation is operator code. Bind it to the already approved
            // package identity rather than permitting arbitrary second packages.
            ensure!(
                prepare
                    .iter()
                    .any(|arg| arg.starts_with(&format!("{}==", self.package))
                        || arg.starts_with(&format!("{}@", self.package))),
                "preparation must use the approved package"
            );
        }
        Ok(())
    }
}
