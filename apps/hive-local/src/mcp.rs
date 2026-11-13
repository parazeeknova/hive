//! Minimal sequential stdio MCP client. A call is never replayed after transport failure.
use crate::{config::Launch, platform, rpc::RESPONSE_BYTES};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{collections::BTreeMap, process::Stdio, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout, Command},
};

pub struct Mcp {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    sequence: u64,
    group: Option<u32>,
}
pub fn command(
    launch: &Launch,
    values: &BTreeMap<String, String>,
    roots: &[String],
) -> Result<Command> {
    let mut command = platform::command(&launch.command)?;
    command.args(&launch.runner_args);
    if launch.command == "uvx" {
        if let Some(entry) = &launch.entry {
            command.args([
                "--from",
                &format!("{}=={}", launch.package, launch.version),
                entry,
            ]);
        } else {
            command.arg(format!("{}=={}", launch.package, launch.version));
        }
    } else {
        command.args(["--yes", &format!("{}@{}", launch.package, launch.version)]);
    }
    for arg in &launch.args {
        if arg == "{roots}" {
            command.args(roots);
        } else {
            let value = expand(arg, values);
            if !value.is_empty() {
                command.arg(value);
            }
        }
    }
    // Never inherit API keys, tokens or other credentials from the shell.
    let inherited: Vec<_> = [
        "PATH",
        "HOME",
        "USERPROFILE",
        "SYSTEMROOT",
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "LANG",
        "SSL_CERT_FILE",
        "SSL_CERT_DIR",
        "UV_CACHE_DIR",
        "XDG_CACHE_HOME",
    ]
    .iter()
    .filter_map(|key| std::env::var_os(key).map(|v| (*key, v)))
    .collect();
    command.env_clear().envs(inherited);
    command.env(
        "UV_CACHE_DIR",
        std::env::var_os("UV_CACHE_DIR").unwrap_or_else(|| platform::cache_dir().into_os_string()),
    );
    if let Some(proxy) = values.get("shared_proxy") {
        for key in ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"] {
            command.env(key, proxy);
        }
    }
    for (key, value) in &launch.env {
        command.env(key, expand(value, values));
    }
    if let Some(workdir) = values.get("workdir") {
        command.current_dir(workdir);
    }
    command
        .kill_on_drop(true)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(unix)]
    command.process_group(0);
    #[cfg(unix)]
    if values.contains_key("shared_uid") {
        command.uid(10001).gid(10001);
    }
    Ok(command)
}
pub fn expand(arg: &str, values: &BTreeMap<String, String>) -> String {
    let mut result = arg.to_owned();
    for (key, value) in values {
        result = result.replace(&format!("{{{key}}}"), value);
    }
    // SQL and Python literals may themselves use braces; only documented
    // placeholders are expanded, all required values are supplied by the caller.
    result
}
impl Mcp {
    pub async fn start(
        launch: &Launch,
        values: &BTreeMap<String, String>,
        roots: &[String],
    ) -> Result<Self> {
        let mut child = command(launch, values, roots)?
            .spawn()
            .context("starting MCP package runner")?;
        let input = child.stdin.take().context("MCP stdin unavailable")?;
        let output = BufReader::new(child.stdout.take().context("MCP stdout unavailable")?);
        let group = child.id();
        let mut mcp = Self {
            child,
            input,
            output,
            sequence: 0,
            group,
        };
        let initialized = mcp.request("initialize",json!({"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"hive-local","version":env!("CARGO_PKG_VERSION")}}),Duration::from_secs(300)).await?;
        ensure!(
            initialized["capabilities"]["tools"].is_object(),
            "upstream does not support tools"
        );
        mcp.send(&json!({"jsonrpc":"2.0","method":"notifications/initialized"}))
            .await?;
        Ok(mcp)
    }
    async fn send(&mut self, value: &Value) -> Result<()> {
        let mut line = serde_json::to_vec(value)?;
        line.push(b'\n');
        self.input.write_all(&line).await?;
        self.input.flush().await?;
        Ok(())
    }
    async fn line(&mut self) -> Result<Value> {
        let mut bytes = Vec::new();
        loop {
            let chunk = self.output.fill_buf().await?;
            ensure!(!chunk.is_empty(), "MCP process closed stdout");
            let count = chunk
                .iter()
                .position(|b| *b == b'\n')
                .map_or(chunk.len(), |n| n + 1);
            ensure!(
                bytes.len() + count <= RESPONSE_BYTES,
                "MCP response exceeds limit"
            );
            let done = chunk[count - 1] == b'\n';
            bytes.extend_from_slice(&chunk[..count]);
            self.output.consume(count);
            if done {
                return Ok(serde_json::from_slice(&bytes)?);
            }
        }
    }
    pub async fn request(
        &mut self,
        method: &str,
        params: Value,
        timeout: Duration,
    ) -> Result<Value> {
        self.sequence += 1;
        let id = self.sequence;
        self.send(&json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}))
            .await?;
        tokio::time::timeout(timeout,async {
            loop {
                let response = self.line().await?;
                if response["id"] == id {
                    if response.get("error").is_some() { bail!("MCP {method} failed"); }
                    return response.get("result").cloned().context("MCP response has no result");
                }
                if response.get("method").is_some() && !response["id"].is_null() {
                    // Upstream cannot request sampling or interactive input in
                    // an unattended worker, nor access the Hive credential.
                    self.send(&json!({"jsonrpc":"2.0","id":response["id"],"error":{"code":-32601,"message":"Client requests are unavailable in this worker"}})).await?;
                }
            }
        }).await.context("MCP request timed out; outcome is unknown and was not repeated")?
    }
    pub async fn tools(&mut self) -> Result<Vec<Value>> {
        let mut tools = Vec::new();
        let mut params = json!({});
        for _ in 0..16 {
            let page = self
                .request("tools/list", params, Duration::from_secs(30))
                .await?;
            tools.extend(
                page["tools"]
                    .as_array()
                    .context("invalid tools list")?
                    .clone(),
            );
            ensure!(tools.len() <= 512, "too many upstream tools");
            match page["nextCursor"].as_str() {
                Some(cursor) => params = json!({"cursor":cursor}),
                None => return Ok(tools),
            }
        }
        bail!("too many tools pages")
    }
    pub async fn close(&mut self) {
        self.stop_group();
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
    fn stop_group(&mut self) {
        if let Some(group) = self.group.take() {
            // Package runners may spawn their server as a descendant. Killing
            // only the runner would leave that server alive after shutdown.
            platform::stop_tree(group);
        }
    }
}
impl Drop for Mcp {
    fn drop(&mut self) {
        self.stop_group();
    }
}
