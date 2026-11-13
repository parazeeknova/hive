//! Native paths and process launch/cleanup. Never route tool arguments through a shell.
use anyhow::{Context, Result};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

pub fn config_path() -> Result<PathBuf> {
    configuration_path(
        |key| std::env::var_os(key),
        cfg!(windows),
        cfg!(target_os = "macos"),
    )
}
fn configuration_path(
    env: impl Fn(&str) -> Option<OsString>,
    windows: bool,
    macos: bool,
) -> Result<PathBuf> {
    let get = |key| env(key).filter(|s| !s.is_empty()).map(PathBuf::from);
    let home = || {
        get(if windows { "USERPROFILE" } else { "HOME" })
            .context("home directory unavailable; pass --config explicitly")
    };
    let base = if windows {
        get("APPDATA").map_or_else(|| home().map(|h| h.join("AppData/Roaming")), Ok)?
    } else if macos {
        home()?.join("Library/Application Support")
    } else {
        get("XDG_CONFIG_HOME")
            .filter(|p| p.is_absolute())
            .map_or_else(|| home().map(|h| h.join(".config")), Ok)?
    };
    Ok(base.join("hive-local/config.json"))
}

/// Windows canonical paths include a verbatim prefix that many Python/Node
/// upstreams do not accept. Keep canonical paths for guards, regular paths for tools.
pub fn tool_path(path: &Path) -> String {
    let raw = path.to_string_lossy();
    #[cfg(windows)]
    {
        let regular = raw.strip_prefix(r"\\?\UNC\").map_or_else(
            || raw.strip_prefix(r"\\?\").unwrap_or(&raw).to_owned(),
            |rest| format!(r"\\{rest}"),
        );
        regular.replace('\\', "/")
    }
    #[cfg(not(windows))]
    raw.into_owned()
}

pub fn ca_bundles() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = ["SSL_CERT_FILE", "CURL_CA_BUNDLE", "REQUESTS_CA_BUNDLE"]
        .iter()
        .filter_map(std::env::var_os)
        .map(PathBuf::from)
        .collect();
    paths.extend(
        [
            "/etc/ssl/certs/ca-certificates.crt",
            "/etc/ssl/cert.pem",
            "/etc/pki/tls/certs/ca-bundle.crt",
        ]
        .map(PathBuf::from),
    );
    paths
        .into_iter()
        .filter(|p| p.is_absolute() && p.is_file())
        .flat_map(|p| std::iter::once(p.clone()).chain(p.canonicalize().ok()))
        .collect()
}
pub fn cache_dir() -> PathBuf {
    #[cfg(windows)]
    {
        std::env::var_os("LOCALAPPDATA")
            .map_or_else(std::env::temp_dir, PathBuf::from)
            .join("uv/cache")
    }
    #[cfg(not(windows))]
    {
        std::env::var_os("XDG_CACHE_HOME")
            .map_or_else(
                || {
                    std::env::var_os("HOME")
                        .map_or_else(std::env::temp_dir, PathBuf::from)
                        .join(".cache")
                },
                PathBuf::from,
            )
            .join("uv")
    }
}

#[cfg_attr(not(windows), allow(clippy::unnecessary_wraps))] // Windows resolves npm's entry point.
pub fn command(runner: &str) -> Result<Command> {
    #[cfg(windows)]
    if runner == "npx" {
        use std::os::windows::process::CommandExt;
        let path = std::env::var_os("PATH").unwrap_or_default();
        let script=std::env::split_paths(&path)
            .map(|dir|dir.join("node_modules/npm/bin/npx-cli.js"))
            .find(|p|p.is_file())
            .or_else(|| {
                // Volta and other version managers expose shims on PATH. Ask
                // Node for its actual executable rather than invoking npx.cmd.
                let output=std::process::Command::new("node.exe").args(["-p","process.execPath"]).creation_flags(0x0800_0000).output().ok()?;
                if !output.status.success() {return None;}
                let executable=PathBuf::from(String::from_utf8(output.stdout).ok()?.trim());
                let candidate=executable.parent()?.join("node_modules/npm/bin/npx-cli.js");
                candidate.is_file().then_some(candidate)
            })
            .context("npm's npx-cli.js was not found on PATH; install Node.js with npm and restart the terminal")?;
        let mut command = Command::new("node.exe");
        command.arg(script);
        hide_console(&mut command);
        return Ok(command);
    }
    let mut command = Command::new(runner);
    hide_console(&mut command);
    Ok(command)
}
fn hide_console(command: &mut Command) {
    #[cfg(windows)]
    command.creation_flags(0x0800_0000);
    #[cfg(not(windows))]
    let _ = command;
}

pub fn stop_tree(id: u32) {
    #[cfg(unix)]
    let mut command = {
        let mut c = std::process::Command::new("kill");
        c.args(["-TERM", "--", &format!("-{id}")]);
        c
    };
    #[cfg(windows)]
    let mut command = {
        use std::os::windows::process::CommandExt;
        let mut c = std::process::Command::new("taskkill.exe");
        c.args(["/PID", &id.to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000);
        c
    };
    let _ = command.stdout(Stdio::null()).stderr(Stdio::null()).status();
}

#[cfg(all(test, not(miri)))]
mod tests {
    use super::*;
    #[test]
    fn defaults_follow_each_operating_system() -> Result<()> {
        let env = |key: &str| match key {
            "HOME" => Some(OsString::from("/users/bee")),
            "USERPROFILE" => Some(OsString::from("C:/Users/Bee")),
            "APPDATA" => Some(OsString::from("C:/Users/Bee/AppData/Roaming")),
            _ => None,
        };
        assert_eq!(
            configuration_path(env, false, false)?,
            PathBuf::from("/users/bee/.config/hive-local/config.json")
        );
        assert_eq!(
            configuration_path(env, false, true)?,
            PathBuf::from("/users/bee/Library/Application Support/hive-local/config.json")
        );
        assert_eq!(
            configuration_path(env, true, false)?,
            PathBuf::from("C:/Users/Bee/AppData/Roaming/hive-local/config.json")
        );
        assert!(configuration_path(|_| None, false, false).is_err());
        Ok(())
    }
    #[cfg(windows)]
    #[test]
    fn windows_paths_are_suitable_for_upstreams() {
        assert_eq!(
            tool_path(Path::new(r"\\?\C:\Users\Bee\project")),
            "C:/Users/Bee/project"
        );
        assert_eq!(
            tool_path(Path::new(r"\\?\UNC\server\share\project")),
            "//server/share/project"
        );
    }
}
