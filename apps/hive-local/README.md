# Hive Local

A Rust background worker for the hosted Hive MCP endpoint. Your agent connects to `https://YOUR-HIVE/mcp`. This worker opens no listening port: it claims your account's queued jobs, runs enabled stdio MCPs, and reports their results to Hive. Independent integrations run concurrently. The executable has no database, WebAssembly engine, Node or Python runtime embedded in it.

Install the helper and the runners required by the integrations you enable:

```sh
cargo install --locked --path apps/hive-local
# Python integrations use uvx; filesystem uses npx. FFmpeg also needs ffmpeg/ffprobe.
hive-local init --hive https://YOUR-HIVE/mcp \
  --key-file /home/YOU/.config/hive/connector-key \
  --root /home/YOU/projects
hive-local enable filesystem
hive-local enable markitdown
hive-local enable duckdb
hive-local enable ffmpeg
hive-local enable serena --project /home/YOU/projects/app
hive-local run
```

Use a dedicated Hive API key with `capabilities:publish` to register and serve connections. Your agent uses its own key or OAuth grant with invoke, memory and code permissions as needed. Keep the helper configuration and Hive key outside every approved folder. All commands accept `--config` with an absolute path. `list` shows enabled integrations; `disable SLUG` stops serving one after an active call finishes. Results requested by an agent pass through Hive, including file contents returned by a tool. Inputs and local files are not bulk uploaded.

| OS | Default configuration |
| --- | --- |
| Windows | `%APPDATA%\hive-local\config.json` (falls back to `%USERPROFILE%\AppData\Roaming`) |
| macOS | `~/Library/Application Support/hive-local/config.json` |
| Linux | `$XDG_CONFIG_HOME/hive-local/config.json`, or `~/.config/hive-local/config.json` |

In PowerShell, use native paths such as `--root "$env:USERPROFILE\projects"` and PowerShell's backtick for multiline commands. See the [project README](../../.github/README.md#enable-tools-on-your-own-computer) for a complete Windows example. Python integrations select Python 3.13 through uvx so they do not accidentally resolve against a newer system Python without compatible upstream wheels. uv can download that interpreter automatically.

The first enable approves a runner/package identity. Every minute the helper reloads configuration, fetches the catalog and refreshes tool snapshots. Catalog version bumps restart an integration between calls and use its newly pinned package version. A different package identity needs `enable` again. Changed tool metadata invalidates Hive's connected-result cache. Removed integrations stop being served. No uncertain tool execution or completion is automatically replayed. Updating the helper executable itself uses your normal package/deployment process.

Outputs go under the first approved folder's `.hive-local/output`. Paths are canonicalized, including parents of new outputs, to reject traversal and symlink escapes. Filesystem also enforces its own allowed roots. DuckDB defaults to a read-only file; use `--database /approved/data.duckdb` when enabling it. Its locked configuration permits files only under approved roots and disables subsequent extension loading. This release supports local DuckDB files and analytical reads; automatic PostgreSQL/MySQL attachment is not configured. Serena uses its explicitly approved project. Local upstreams run with your OS identity; these argument checks are not a replacement for OS isolation of untrusted upstream packages.

For automatic startup on Linux, install the supplied user service after building:

```sh
mkdir -p ~/.config/systemd/user
cp apps/hive-local/hive-local.service ~/.config/systemd/user/
systemctl --user daemon-reload
systemctl --user enable --now hive-local
```

For macOS, initialize the helper, then run `sh apps/hive-local/autostart/install-macos.sh`. The installer requires Python 3 and creates a per-user LaunchAgent; it also accepts executable and config paths as its first two arguments.

For Windows, initialize the helper, then run `powershell -File apps/hive-local/autostart/install-windows.ps1`. It registers a per-user Task Scheduler task at sign-in. `-Executable` and `-Config` override the defaults. Neither installer is run automatically by `init` or `enable`.

Windows runner launch, path normalization and process-tree cleanup are handled natively. The operator shared-service sandbox remains Linux-container infrastructure and can be run through Docker Desktop on Windows/macOS.

The existing TypeScript Hive Local MCP proxy remains available for previews, provider connections and code-index upload. Both bridges use the same hosted gateway, connection queue, memory, code index and discovery. Give independently managed connections distinct names; two workers must not register the same name.

## Shared services

The shared MarkItDown and DuckDB launchers are intended for an operator deployment, using `docker/hive-local/compose.yaml`. Copy `shared.example.json`, set the real hosted Hive URL, and mount a connector key owned by the catalog's `shared_owner`. Set `HIVE_SHARED_CONFIG` and `HIVE_SHARED_KEY` to those absolute file paths, then:

```sh
docker compose -f docker/hive-local/compose.yaml up --build -d
```

The worker has an internal-only network. Its only egress is a private HTTPS CONNECT proxy that rejects private, loopback, link-local, reserved and metadata destinations, including redirected requests. Do not attach another external network to the worker. The container has a read-only root, temporary workspace, resource limits and no host data mounts. Each shared call starts a new upstream session. Shared DuckDB uses a read-only empty database and locked configuration; the HTTP extension is bootstrapped over HTTPS before lockdown. Shared MarkItDown accepts HTTPS and data URLs; local file URIs require the user's own helper. Shared services need this one-time operator deployment; catalog publication alone does not provision running upstreams.

Mount the shared connector key owned by root with mode `0600`. The Rust supervisor reads it; package runners and upstreams drop to uid/gid 10001 and cannot read that key. The supervisor capabilities exist only to drop child privileges and terminate its upstream processes between calls. Desktop mode uses your normal OS identity.

## Verification

```sh
cargo test --locked --workspace --all-targets --all-features
HIVE_TEST_UPSTREAMS=1 cargo test -p hive-local -- --include-ignored
HIVE_TEST_DATABASE_URL=postgres://... cargo test -p hiveway --test workflows -- --ignored
```

External upstream tests additionally require `HIVE_TEST_UPSTREAMS=1` so a normal CI run with `--include-ignored` does not download packages or require local runners. The shared sandbox test also needs `HIVE_TEST_PROXY` pointing at the supplied proxy. Database tests create isolated schemas and use only disposable test data.
