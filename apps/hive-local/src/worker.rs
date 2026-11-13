use crate::{
    config::{Approval, Config, Integration, Launch},
    guard,
    mcp::{self, Mcp},
    platform,
    rpc::Hive,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

const HTTP_WAIT: Duration = Duration::from_secs(40);

pub fn values(
    config: &Config,
    approval: &Approval,
    slug: &str,
) -> Result<(BTreeMap<String, String>, Vec<String>)> {
    if config.shared {
        ensure!(
            std::env::var("HIVE_SHARED_SANDBOX").as_deref() == Ok("1"),
            "shared mode requires the supplied sandbox container"
        );
        let proxy =
            std::env::var("HIVE_SHARED_PROXY").context("shared mode requires the egress proxy")?;
        let proxy_url = reqwest::Url::parse(&proxy)?;
        ensure!(
            proxy_url.scheme() == "http"
                && proxy_url.username().is_empty()
                && proxy_url.password().is_none(),
            "invalid shared proxy"
        );
        let workdir =
            std::env::temp_dir().join(format!("hive-shared-{}-{slug}", std::process::id()));
        std::fs::create_dir_all(&workdir)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&workdir, std::fs::Permissions::from_mode(0o777))?;
        }
        return Ok((
            BTreeMap::from([
                ("workdir".into(), platform::tool_path(&workdir)),
                (
                    "ca_bundle".into(),
                    "/etc/ssl/certs/ca-certificates.crt".into(),
                ),
                ("shared_proxy".into(), proxy),
                ("shared_uid".into(), "10001".into()),
            ]),
            Vec::new(),
        ));
    }
    let root = config
        .roots
        .first()
        .context("enable a folder with --root before enabling local tools")?
        .canonicalize()?;
    let output = root.join(".hive-local/output");
    guard::approved(&output, &config.roots)?;
    std::fs::create_dir_all(&output)?;
    let workdir = output.join(slug);
    guard::approved(&workdir, &config.roots)?;
    std::fs::create_dir_all(&workdir)?;
    let project = approval.project.as_ref().unwrap_or(&root);
    guard::approved(project, &config.roots)?;
    let database = approval
        .database
        .clone()
        .unwrap_or_else(|| workdir.join("local.duckdb"));
    guard::approved(&database, &config.roots)?;
    let roots: Vec<String> = config
        .roots
        .iter()
        .map(|p| p.canonicalize().map(|p| platform::tool_path(&p)))
        .collect::<std::io::Result<_>>()?;
    let sql_roots = roots
        .iter()
        .map(|r| format!("'{}/'", r.replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    let certificates = platform::ca_bundles()
        .iter()
        .map(|p| format!("'{}'", platform::tool_path(p).replace('\'', "''")))
        .collect::<Vec<_>>()
        .join(",");
    // Install before locking configuration. Files are limited to approved roots;
    // extensions cannot later enable arbitrary filesystem access or subprocesses.
    let init = format!(
        "INSTALL httpfs; LOAD httpfs; SET allow_persistent_secrets=false; SET allowed_directories=[{sql_roots},'https://']; SET allowed_paths=[{certificates}]; SET enable_external_access=false; SET autoinstall_known_extensions=false; SET autoload_known_extensions=false; SET allow_community_extensions=false; SET memory_limit='512MB'; SET threads=2; SET lock_configuration=true;"
    );
    Ok((
        BTreeMap::from([
            ("root".into(), platform::tool_path(&root)),
            ("output_dir".into(), platform::tool_path(&output)),
            ("workdir".into(), platform::tool_path(&workdir)),
            ("project".into(), platform::tool_path(project)),
            ("duckdb_path".into(), platform::tool_path(&database)),
            ("duckdb_flags".into(), String::new()),
            ("duckdb_init".into(), init),
            (
                "ca_bundle".into(),
                "/etc/ssl/certs/ca-certificates.crt".into(),
            ),
        ]),
        roots,
    ))
}
pub(crate) async fn prepare(launch: &Launch, values: &BTreeMap<String, String>) -> Result<()> {
    for required in &launch.requires {
        let status = platform::command(required)?
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .status()
            .await
            .with_context(|| format!("install required program {required}"))?;
        ensure!(status.success(), "required program {required} failed");
    }
    for step in &launch.prepare {
        let mut command = platform::command(&step[0])?;
        for arg in &step[1..] {
            command.arg(mcp::expand(arg, values));
        }
        command.kill_on_drop(true).env_clear();
        command.env(
            "UV_CACHE_DIR",
            std::env::var_os("UV_CACHE_DIR")
                .unwrap_or_else(|| platform::cache_dir().into_os_string()),
        );
        #[cfg(unix)]
        if values.contains_key("shared_uid") {
            command.uid(10001).gid(10001);
        }
        if let Some(proxy) = values.get("shared_proxy") {
            for key in ["HTTP_PROXY", "HTTPS_PROXY", "http_proxy", "https_proxy"] {
                command.env(key, proxy);
            }
        }
        for name in [
            "PATH",
            "HOME",
            "USERPROFILE",
            "SYSTEMROOT",
            "APPDATA",
            "LOCALAPPDATA",
            "TEMP",
            "TMP",
            "SSL_CERT_FILE",
            "SSL_CERT_DIR",
            "UV_CACHE_DIR",
            "XDG_CACHE_HOME",
        ] {
            if let Some(v) = std::env::var_os(name) {
                command.env(name, v);
            }
        }
        let status = tokio::time::timeout(Duration::from_secs(180), command.status())
            .await
            .context("MCP package preparation timed out")??;
        ensure!(status.success(), "MCP package preparation failed");
    }
    Ok(())
}
async fn snapshot(mcp: &mut Mcp, integration: &Integration, shared: bool) -> Result<Vec<Value>> {
    let mut tools = mcp.tools().await?;
    tools.retain(|tool| {
        tool["name"]
            .as_str()
            .is_some_and(|name| integration.tools.iter().any(|allowed| allowed == name))
    });
    for tool in &mut tools {
        let name = tool["name"].as_str().unwrap_or_default().to_owned();
        if let Some(annotations) = integration.annotations.get(&name) {
            tool["annotations"] = annotations.clone();
        }
        if shared && let Some(annotations) = integration.shared_annotations.get(&name) {
            tool["annotations"] = annotations.clone();
        }
        // Hive's snapshot hash includes metadata, invalidating its result cache
        // when the executable version changes with an identical tool schema.
        let launch = if shared {
            integration.shared_launch.as_ref()
        } else {
            integration.local.as_ref()
        };
        if let Some(launch) = launch {
            tool["_meta"]["dev.hive/packageVersion"] = json!(launch.version);
        }
    }
    ensure!(
        !tools.is_empty(),
        "no catalog tools are supported by this upstream version"
    );
    Ok(tools)
}
fn failure(message: &str) -> Value {
    json!({"content":[{"type":"text","text":message}],"isError":true})
}

/// One worker owns one stdio session. Independent integrations run concurrently.
/// Reconfigure only between jobs; a running mutation is never replayed.
#[allow(clippy::too_many_lines)]
pub async fn serve(file: PathBuf, slug: String) -> Result<()> {
    let mut backoff = 1;
    loop {
        let config = Config::read(&file)?;
        let Some(approval) = config.enabled.get(&slug) else {
            return Ok(());
        };
        let hive = Hive::new(&config)?;
        let catalog = hive.catalog().await?;
        let integration = catalog
            .integrations
            .iter()
            .find(|i| i.slug == slug)
            .context("integration was removed from the catalog")?;
        let mut launch = if config.shared {
            integration.shared_launch.as_ref()
        } else {
            integration.local.as_ref()
        }
        .context("integration has no launcher for this mode")?
        .clone();
        launch.validate(approval)?;
        let (values, roots) = values(&config, approval, &slug)?;
        if config.shared && slug == "duckdb" {
            let proxy = values
                .get("shared_proxy")
                .context("missing shared proxy")?
                .replace('\'', "''");
            for arg in &mut launch.args {
                *arg = arg.replace(
                    "SET lock_configuration=true",
                    &format!("SET http_proxy='{proxy}'; SET lock_configuration=true"),
                );
            }
        }
        prepare(&launch, &values).await?;
        let mut mcp = Mcp::start(&launch, &values, &roots).await?;
        let tools = snapshot(&mut mcp, integration, config.shared).await?;
        let registered = hive
            .operation(
                "register_mcp_connection",
                json!({"name":slug,"tools":tools}),
                HTTP_WAIT,
            )
            .await?;
        let id = registered["id"].as_str().context("missing connection id")?;
        eprintln!("Hive Local: {slug} connected through {}", config.hive);
        let mut refreshed = Instant::now();
        let outcome:Result<()> = async {
            loop {
                if refreshed.elapsed() >= Duration::from_secs(60) {
                    let updated=Config::read(&file)?;
                    if !updated.enabled.contains_key(&slug) {
                        let _=hive.operation("disconnect_mcp",json!({"connection_id":id}),HTTP_WAIT).await;
                        return Ok(())
                    }
                    if updated.enabled.get(&slug)!=Some(approval) || updated.roots!=config.roots || updated.hive!=config.hive || updated.key_file!=config.key_file || updated.shared!=config.shared {return Ok(())}
                    let next=hive.catalog().await?;
                    if next.integrations.iter().find(|i|i.slug==slug)!=Some(integration) {return Ok(())}
                    // Upstreams may change tools without a package/version bump.
                    let tools=snapshot(&mut mcp,integration,config.shared).await?;
                    hive.operation("register_mcp_connection",json!({"name":slug,"tools":tools}),HTTP_WAIT).await?;
                    refreshed=Instant::now();
                }
                let wait=catalog.connectors.poll_wait_seconds;
                let polled=hive.operation("poll_mcp",json!({"connection_id":id,"wait_seconds":wait}),Duration::from_secs(wait+15)).await?;
                let job=&polled["job"];
                if job.is_null() {if wait==0 {tokio::time::sleep(Duration::from_secs(1)).await;} continue;}
                let request=&job["request"];
                if config.shared {
                    // No session or temporary table can leak to the next caller.
                    mcp.close().await;
                    mcp=Mcp::start(&launch,&values,&roots).await?;
                }
                let mut args=request.get("arguments").cloned().unwrap_or_else(||json!({}));
                let tool=request["name"].as_str().unwrap_or_default();
                let validation=guard::arguments(&config,integration,&mut args).and_then(|()| {
                    ensure!(job["method"].as_str().unwrap_or("tools/call")=="tools/call","only tool calls are supported");
                    ensure!(tools.iter().any(|t|t["name"]==tool),"unknown tool");
                    ensure!(args.to_string().len()<=catalog.connectors.argument_bytes,"arguments exceed limit");
                    Ok(())
                });
                let mut broken=false;
                let mut result=if let Err(error) = validation { failure(&error.to_string()) } else {
                    // Heartbeats keep Hive aware of progress and let callers
                    // cancel an active upstream call without waiting for TTL.
                    let execute=mcp.request("tools/call",json!({"name":tool,"arguments":args}),Duration::from_secs(catalog.connectors.job_ttl_seconds));
                    tokio::pin!(execute);
                    let mut heartbeat=tokio::time::interval(Duration::from_secs(5));
                    loop {tokio::select! {
                        outcome=&mut execute=>break if let Ok(result) = outcome { result } else {broken=true;failure("Upstream failed or timed out; outcome is unknown and this call was not repeated.")},
                        _=heartbeat.tick()=> {
                            let progress=hive.operation("progress_mcp",json!({"connection_id":id,"job_id":job["id"],"lease":job["lease"],"message":"Working"}),HTTP_WAIT).await;
                            if progress.is_ok_and(|p|p["cancel_requested"]==true || p["cancelled"]==true) {broken=true;break failure("Call cancelled; upstream outcome may be unknown.");}
                        }
                    }}
                };
                if result.to_string().len()>catalog.connectors.result_bytes {result=failure("Upstream result exceeds the deployment size limit.");}
                // Retry neither execution nor an uncertain completion.
                if hive.operation("complete_mcp",json!({"connection_id":id,"job_id":job["id"],"lease":job["lease"],"result":result}),HTTP_WAIT).await.is_err() {
                    eprintln!("Hive Local: {slug} completion was not acknowledged; call will not be repeated");
                }
                backoff=1;
                if broken {return Ok(())}
            }
        }.await;
        mcp.close().await;
        if let Err(error) = outcome {
            eprintln!("Hive Local: {slug}: {error}; reconnecting");
        }
        tokio::time::sleep(Duration::from_secs(backoff)).await;
        backoff = (backoff * 2).min(30);
    }
}

pub async fn run(file: &Path) -> Result<()> {
    let mut workers: BTreeMap<String, tokio::task::JoinHandle<Result<()>>> = BTreeMap::new();
    let mut refresh = tokio::time::interval(Duration::from_secs(15));
    let shutdown = async {
        #[cfg(unix)]
        {
            let mut term =
                tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
            tokio::select! {r=tokio::signal::ctrl_c()=>r?,_=term.recv()=>{}}
        }
        #[cfg(not(unix))]
        tokio::signal::ctrl_c().await?;
        Ok::<(), std::io::Error>(())
    };
    tokio::pin!(shutdown);
    loop {
        tokio::select! {
            _=&mut shutdown=>break,
            _=refresh.tick()=> {
                let config=Config::read(file)?;
                for slug in config.enabled.keys() {
                    if workers.get(slug).is_some_and(tokio::task::JoinHandle::is_finished)
                        && let Some(worker)=workers.remove(slug)&& let Ok(Err(error))=worker.await {eprintln!("Hive Local: {slug}: {error}");}
                    if !workers.contains_key(slug) {workers.insert(slug.clone(),tokio::spawn(serve(file.to_owned(),slug.clone())));}
                }
            }
        }
    }
    for worker in workers.values() {
        worker.abort();
    }
    for (_, worker) in workers {
        let _ = worker.await;
    }
    Ok(())
}
