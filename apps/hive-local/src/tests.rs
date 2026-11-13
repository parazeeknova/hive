use super::*;
use anyhow::Result;
use serde_json::{Value, json};
use std::{collections::BTreeMap, time::Duration};

async fn remove_fixture(path: &std::path::Path) -> Result<()> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    loop {
        match std::fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(error)
                if cfg!(windows)
                    && matches!(error.raw_os_error(), Some(5 | 32))
                    && tokio::time::Instant::now() < deadline =>
            {
                // Windows may release descendant process file handles shortly
                // after taskkill has returned. Persistent orphan locks still fail.
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[test]
fn package_approvals_bind_identity_but_allow_version_updates() -> Result<()> {
    let mut launch: config::Launch = serde_json::from_value(
        json!({"command":"uvx","package":"markitdown-mcp","version":"0.0.1a8","args":[]}),
    )?;
    let approval = config::Approval {
        command: launch.command.clone(),
        package: launch.package.clone(),
        project: None,
        database: None,
    };
    launch.validate(&approval)?;
    launch.version = "0.0.1a9".into();
    launch.validate(&approval)?;
    launch.package = "other-package".into();
    assert!(launch.validate(&approval).is_err());
    launch.package = approval.package.clone();
    launch.version = "latest; touch x".into();
    assert!(launch.validate(&approval).is_err());
    assert!(config::validate_url("http://example.com/mcp").is_err());
    assert!(config::validate_url("https://user:pass@example.com/mcp").is_err());
    assert!(config::validate_url("http://127.0.0.1:3001/mcp").is_ok());
    Ok(())
}

#[test]
fn command_expands_roots_as_separate_arguments_and_drops_shell_secrets() -> Result<()> {
    let launch: config::Launch = serde_json::from_value(
        json!({"command":"npx","package":"@modelcontextprotocol/server-filesystem","version":"2.3.0","args":["{roots}"]}),
    )?;
    let command = mcp::command(
        &launch,
        &BTreeMap::new(),
        &["/tmp/a b".into(), "/tmp/c".into()],
    )?;
    let command = command.as_std();
    let args: Vec<_> = command
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    #[cfg(windows)]
    let args = args.into_iter().skip(1).collect::<Vec<_>>();
    assert_eq!(
        args,
        vec![
            "--yes",
            "@modelcontextprotocol/server-filesystem@2.3.0",
            "/tmp/a b",
            "/tmp/c"
        ]
    );
    assert!(
        !command
            .get_envs()
            .any(|(k, _)| k == "OPENAI_API_KEY" || k == "HIVE_API_KEY")
    );
    Ok(())
}

/// Real upstream smoke tests are explicit because they fetch pinned packages.
#[tokio::test]
#[ignore = "requires uvx and network access to download the pinned upstream"]
async fn real_markitdown_stdio_converts_a_document() -> Result<()> {
    if std::env::var("HIVE_TEST_UPSTREAMS").as_deref() != Ok("1") {
        return Ok(());
    }
    let entries: Vec<Value> =
        serde_json::from_str(include_str!("../../hiveway/integrations.json"))?;
    let item: config::Integration = serde_json::from_value(
        entries
            .into_iter()
            .find(|i| i["slug"] == "markitdown")
            .ok_or_else(|| anyhow::anyhow!("missing integration"))?,
    )?;
    let launch = item
        .local
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing launch"))?;
    let mut mcp = mcp::Mcp::start(launch, &BTreeMap::new(), &[]).await?;
    let outcome=async {
        assert!(mcp.tools().await?.iter().any(|t|t["name"]=="convert_to_markdown"));
        let result=mcp.request("tools/call",json!({"name":"convert_to_markdown","arguments":{"uri":"data:text/html,%3Ch1%3EHive%20fixture%3C%2Fh1%3E%3Cp%3EConnected%20document.%3C%2Fp%3E"}}),Duration::from_secs(30)).await?;
        assert_ne!(result["isError"],true,"{result}");
        assert!(result.to_string().contains("Hive fixture"),"{result}");
        Ok::<(),anyhow::Error>(())
    }.await;
    mcp.close().await;
    outcome
}

#[tokio::test]
#[ignore = "requires uvx and network access to download the pinned upstream"]
async fn real_duckdb_session_queries_and_blocks_unapproved_files() -> Result<()> {
    if std::env::var("HIVE_TEST_UPSTREAMS").as_deref() != Ok("1") {
        return Ok(());
    }
    let entries: Vec<Value> =
        serde_json::from_str(include_str!("../../hiveway/integrations.json"))?;
    let item: config::Integration = serde_json::from_value(
        entries
            .into_iter()
            .find(|i| i["slug"] == "duckdb")
            .ok_or_else(|| anyhow::anyhow!("missing integration"))?,
    )?;
    let launch = item
        .local
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("missing launch"))?;
    let root = std::env::temp_dir().join(format!("hive-duck-test-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let config = config::Config {
        hive: "https://example.com/mcp".into(),
        key_file: root.with_extension("key"),
        roots: vec![root.clone()],
        enabled: BTreeMap::new(),
        shared: false,
    };
    let approval = config::Approval {
        command: launch.command.clone(),
        package: launch.package.clone(),
        project: None,
        database: None,
    };
    let (values, roots) = worker::values(&config, &approval, "duckdb")?;
    // Upstream defaults to read-only database files, so create its empty file.
    let path = values
        .get("duckdb_path")
        .ok_or_else(|| anyhow::anyhow!("missing database"))?;
    let status = tokio::process::Command::new("uvx")
        .args([
            "--python",
            "3.13",
            "--from",
            "mcp-server-motherduck==1.1.1",
            "python",
            "-c",
            "import duckdb,sys; duckdb.connect(sys.argv[1]).close()",
            path,
        ])
        .status()
        .await?;
    assert!(status.success());
    let csv = root.join("data with spaces.csv");
    std::fs::write(&csv, "value\n20\n22\n")?;
    let mut mcp = mcp::Mcp::start(launch, &values, &roots).await?;
    let outcome=async {
        let result=mcp.request("tools/call",json!({"name":"execute_query","arguments":{"sql":"SELECT 21 * 2 AS answer"}}),Duration::from_secs(30)).await?;
        assert_ne!(result["isError"],true,"{result}");assert!(result.to_string().contains("42"),"{result}");
        let local_sql=format!("SELECT SUM(value) AS total FROM read_csv_auto('{}')",platform::tool_path(&csv).replace('\'',"''"));
        let local=mcp.request("tools/call",json!({"name":"execute_query","arguments":{"sql":local_sql}}),Duration::from_secs(30)).await?;
        assert_ne!(local["isError"],true,"{local}");
        assert!(local.to_string().contains("42"),"{local}");
        let public=mcp.request("tools/call",json!({"name":"execute_query","arguments":{"sql":"SELECT count(*) AS n FROM read_csv_auto('https://duckdb.org/data/flights.csv')"}}),Duration::from_secs(30)).await?;
        assert_ne!(public["isError"],true,"{public}");
        let blocked=mcp.request("tools/call",json!({"name":"execute_query","arguments":{"sql":"SELECT * FROM read_text('/etc/passwd')"}}),Duration::from_secs(30)).await?;
        assert!(blocked["isError"]==true || blocked.to_string().contains("Error") || blocked.to_string().contains("error"),"{blocked}");
        assert!(!blocked.to_string().contains("root:x:"));
        Ok::<(),anyhow::Error>(())
    }.await;
    mcp.close().await;
    remove_fixture(&root).await?;
    outcome
}

#[tokio::test]
#[ignore = "set HIVE_TEST_UPSTREAMS=1; requires uvx and the pinned MarkItDown package"]
#[allow(clippy::too_many_lines)]
async fn real_worker_claims_executes_and_reports_once() -> Result<()> {
    use axum::{
        Json, Router,
        extract::State,
        routing::{get, post},
    };
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    struct Fixture {
        polls: AtomicUsize,
        completions: AtomicUsize,
        done: Mutex<Option<tokio::sync::oneshot::Sender<Value>>>,
    }
    async fn gateway(State(f): State<Arc<Fixture>>, Json(request): Json<Value>) -> Json<Value> {
        let params = &request["params"];
        let result = match params["name"].as_str().unwrap_or_default() {
            "register_mcp_connection" => {
                assert!(
                    params["arguments"]["tools"]
                        .as_array()
                        .is_some_and(|t| !t.is_empty())
                );
                json!({"id":"connection-fixture"})
            }
            "poll_mcp" => {
                if f.polls.fetch_add(1, Ordering::SeqCst) == 0 {
                    json!({"job":{"id":"job-fixture","lease":"lease-fixture","method":"tools/call","request":{"name":"convert_to_markdown","arguments":{"uri":"data:text/html,%3Ch1%3EWorker%20fixture%3C%2Fh1%3E"}}}})
                } else {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    json!({"job":null})
                }
            }
            "progress_mcp" => json!({"cancel_requested":false}),
            "complete_mcp" => {
                f.completions.fetch_add(1, Ordering::SeqCst);
                if let Ok(mut sender) = f.done.lock()
                    && let Some(sender) = sender.take()
                {
                    let _ = sender.send(params["arguments"]["result"].clone());
                }
                json!({"accepted":true})
            }
            _ => json!({}),
        };
        Json(
            json!({"jsonrpc":"2.0","id":request["id"],"result":{"content":[],"structuredContent":result,"isError":false}}),
        )
    }
    if std::env::var("HIVE_TEST_UPSTREAMS").as_deref() != Ok("1") {
        return Ok(());
    }
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let fixture = Arc::new(Fixture {
        polls: AtomicUsize::new(0),
        completions: AtomicUsize::new(0),
        done: Mutex::new(Some(sender)),
    });
    let catalog: Value = json!({"integrations":serde_json::from_str::<Value>(include_str!("../../hiveway/integrations.json"))?,"connectors":{"argument_bytes":131_072,"result_bytes":524_288,"poll_wait_seconds":1,"job_ttl_seconds":30}});
    let router = Router::new()
        .route("/mcp", post(gateway))
        .with_state(fixture.clone())
        .route(
            "/api/config",
            get(move || {
                let c = catalog.clone();
                async move { Json(c) }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(async move { axum::serve(listener, router).await });
    let dir = std::env::temp_dir().join(format!("hive-worker-test-{}", std::process::id()));
    let root = dir.join("approved");
    std::fs::create_dir_all(&root)?;
    let key = dir.join("key");
    std::fs::write(&key, "fixture-key")?;
    let config = config::Config {
        hive: format!("http://{address}/mcp"),
        key_file: key,
        roots: vec![root],
        enabled: BTreeMap::from([(
            "markitdown".into(),
            config::Approval {
                command: "uvx".into(),
                package: "markitdown-mcp".into(),
                project: None,
                database: None,
            },
        )]),
        shared: false,
    };
    let path = dir.join("config.json");
    config.write(&path)?;
    let worker = tokio::spawn(worker::serve(path, "markitdown".into()));
    let result = tokio::time::timeout(Duration::from_secs(360), receiver).await;
    worker.abort();
    let _ = worker.await;
    server.abort();
    let _ = server.await;
    remove_fixture(&dir).await?;
    let result = result??;
    assert_ne!(result["isError"], true, "{result}");
    assert!(result.to_string().contains("Worker fixture"), "{result}");
    assert_eq!(fixture.completions.load(Ordering::SeqCst), 1);
    Ok(())
}

#[tokio::test]
#[ignore = "set HIVE_TEST_UPSTREAMS=1; requires uvx, npx, ffmpeg, ffprobe and git"]
#[allow(clippy::too_many_lines)]
async fn real_filesystem_media_and_semantic_code_tools() -> Result<()> {
    if std::env::var("HIVE_TEST_UPSTREAMS").as_deref() != Ok("1") {
        return Ok(());
    }
    let root = std::env::temp_dir().join(format!("hive-local-tools-test-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    std::fs::write(
        root.join("fixture.py"),
        "def add(a, b):\n    return a + b\n",
    )?;
    assert!(
        tokio::process::Command::new("git")
            .args(["init", "--quiet"])
            .arg(&root)
            .status()
            .await?
            .success()
    );
    let entries: Vec<Value> =
        serde_json::from_str(include_str!("../../hiveway/integrations.json"))?;
    for slug in ["filesystem", "ffmpeg", "serena"] {
        let item: config::Integration = serde_json::from_value(
            entries
                .iter()
                .find(|i| i["slug"] == slug)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("missing integration"))?,
        )?;
        let mut launch = item
            .local
            .ok_or_else(|| anyhow::anyhow!("missing launch"))?;
        if slug == "serena" {
            let home = root.join(".test-home");
            std::fs::create_dir_all(&home)?;
            launch
                .env
                .insert("HOME".into(), home.to_string_lossy().into_owned());
            launch
                .env
                .insert("USERPROFILE".into(), home.to_string_lossy().into_owned());
            let cache = std::env::var("UV_CACHE_DIR")
                .unwrap_or_else(|_| platform::cache_dir().to_string_lossy().into_owned());
            launch.env.insert("UV_CACHE_DIR".into(), cache);
        }
        let config = config::Config {
            hive: "https://example.com/mcp".into(),
            key_file: root.with_extension("key"),
            roots: vec![root.clone()],
            enabled: BTreeMap::new(),
            shared: false,
        };
        let approval = config::Approval {
            command: launch.command.clone(),
            package: launch.package.clone(),
            project: Some(root.clone()),
            database: None,
        };
        launch.validate(&approval)?;
        let (values, roots) = worker::values(&config, &approval, slug)?;
        worker::prepare(&launch, &values).await?;
        let mut mcp = mcp::Mcp::start(&launch, &values, &roots).await?;
        let outcome=async {
            match slug {
                "filesystem"=>{
                    let path=root.join("note.txt").to_string_lossy().into_owned();
                    let written=mcp.request("tools/call",json!({"name":"write_file","arguments":{"path":path,"content":"Hive filesystem fixture"}}),Duration::from_secs(30)).await?;
                    assert_ne!(written["isError"],true,"{written}");
                    let read=mcp.request("tools/call",json!({"name":"read_text_file","arguments":{"path":path}}),Duration::from_secs(30)).await?;
                    assert!(read.to_string().contains("Hive filesystem fixture"),"{read}");
                },
                "ffmpeg"=>{
                    let path=root.join("tone.wav");
                    let status=tokio::process::Command::new("ffmpeg").args(["-loglevel","error","-f","lavfi","-i","sine=frequency=440:duration=0.1","-y"]).arg(&path).status().await?;
                    assert!(status.success());
                    let info=mcp.request("tools/call",json!({"name":"ffmpeg_get_info","arguments":{"file_path":path}}),Duration::from_secs(30)).await?;
                    assert_ne!(info["isError"],true,"{info}");assert!(info.to_string().contains("wav"),"{info}");
                },
                _=>{
                    assert!(mcp.tools().await?.iter().any(|t|t["name"]=="find_symbol"));
                    let found=mcp.request("tools/call",json!({"name":"find_symbol","arguments":{"name_path_pattern":"add","relative_path":"fixture.py","include_body":true}}),Duration::from_secs(120)).await?;
                    assert_ne!(found["isError"],true,"{found}");assert!(found.to_string().contains("return a + b"),"{found}");
                },
            }
            Ok::<(),anyhow::Error>(())
        }.await;
        mcp.close().await;
        outcome?;
    }
    remove_fixture(&root).await?;
    Ok(())
}

#[tokio::test]
#[ignore = "set HIVE_TEST_UPSTREAMS=1 and HIVE_TEST_PROXY to the supplied HTTPS-only proxy"]
async fn real_shared_duckdb_is_read_only_and_fetches_public_data() -> Result<()> {
    if std::env::var("HIVE_TEST_UPSTREAMS").as_deref() != Ok("1") {
        return Ok(());
    }
    let Ok(proxy) = std::env::var("HIVE_TEST_PROXY") else {
        return Ok(());
    };
    let entries: Vec<Value> =
        serde_json::from_str(include_str!("../../hiveway/integrations.json"))?;
    let item: config::Integration = serde_json::from_value(
        entries
            .into_iter()
            .find(|i| i["slug"] == "duckdb")
            .ok_or_else(|| anyhow::anyhow!("missing integration"))?,
    )?;
    let mut launch = item
        .shared_launch
        .ok_or_else(|| anyhow::anyhow!("missing shared launch"))?;
    let root = std::env::temp_dir().join(format!("hive-shared-duck-test-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let values = BTreeMap::from([
        ("workdir".into(), root.to_string_lossy().into_owned()),
        ("shared_proxy".into(), proxy.clone()),
        (
            "ca_bundle".into(),
            "/etc/ssl/certs/ca-certificates.crt".into(),
        ),
    ]);
    for arg in &mut launch.args {
        *arg = arg.replace(
            "SET lock_configuration=true",
            &format!(
                "SET http_proxy='{}'; SET lock_configuration=true",
                proxy.replace('\'', "''")
            ),
        );
    }
    worker::prepare(&launch, &values).await?;
    let mut mcp = mcp::Mcp::start(&launch, &values, &[]).await?;
    let outcome=async {
        let public=mcp.request("tools/call",json!({"name":"execute_query","arguments":{"sql":"SELECT count(*) AS n FROM read_csv_auto('https://duckdb.org/data/flights.csv')"}}),Duration::from_secs(60)).await?;
        assert_ne!(public["isError"],true,"{public}");let data:Value=serde_json::from_str(public["content"][0]["text"].as_str().ok_or_else(||anyhow::anyhow!("missing SQL result"))?)?; assert_eq!(data["rows"][0][0],3);
        for sql in ["SELECT * FROM read_text('/etc/passwd')","COPY (SELECT 1) TO '/tmp/hive-shared-must-not-write.csv'","SET enable_external_access=true","SELECT * FROM read_csv_auto('https://127.0.0.1/private.csv')"] {
            let blocked=mcp.request("tools/call",json!({"name":"execute_query","arguments":{"sql":sql}}),Duration::from_secs(30)).await?;
            assert_eq!(blocked["isError"],true,"{sql}: {blocked}");
        }
        Ok::<(),anyhow::Error>(())
    }.await;
    mcp.close().await;
    remove_fixture(&root).await?;
    outcome
}
