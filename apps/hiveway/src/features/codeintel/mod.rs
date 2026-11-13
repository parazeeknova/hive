//! Commit-addressed code intelligence for an account's repositories.
//!
//! Snapshots are keyed by `(repository, commit)` and file contents by their git
//! blob id, so indexing a new commit only uploads files that changed, and every
//! machine or CI job of the same account reuses what any of them uploaded.
//! Content is verified against its blob id (`sha1("blob <len>\0" + bytes)`), so
//! a cache entry always is exactly the committed file.
//!
//! Flow: `index_repository` sends the manifest (`git ls-tree`) and receives the
//! blob ids the server lacks, `upload_code_blobs` sends those, and the final
//! `index_repository` call with `finalize` builds the overview. Queries run
//! against the requested commit, or the latest ready snapshot.
pub mod overview;
pub mod symbols;

use crate::{
    features::{
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        memory,
    },
    state::AppState,
};
use axum::http::StatusCode;
use sea_orm::{ConnectionTrait, DbBackend, QueryResult, Statement, TransactionTrait};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};

fn statement(sql: &str, values: Vec<sea_orm::Value>) -> Statement {
    Statement::from_sql_and_values(DbBackend::Postgres, sql, values)
}
fn limits(state: &AppState) -> &crate::runtime::CodeIndex {
    &state.settings.runtime.code_index
}

/// Normalize a repository reference to `host/owner/name` (lower-case, no scheme,
/// credentials or `.git`). Accepts HTTPS and SSH remotes, `host/path` forms and
/// `local/<name>` for repositories without a remote.
/// # Errors
/// Rejects empty, oversized or path-traversing references.
pub fn repository_key(raw: &str) -> ApiResult<String> {
    let raw = raw.trim();
    let invalid = || ApiError::bad("invalid_repository");
    if raw.is_empty() || raw.len() > 512 {
        return Err(invalid());
    }
    let without_scheme = raw.split_once("://").map_or(raw, |(_, rest)| rest);
    let without_user = without_scheme
        .rsplit_once('@')
        .filter(|(user, _)| !user.contains('/'))
        .map_or(without_scheme, |(_, rest)| rest);
    // `git@host:owner/repo` uses a colon after the host.
    let joined = match without_user.split_once(':') {
        Some((host, rest))
            if !host.contains('/') && !rest.starts_with(|c: char| c.is_ascii_digit()) =>
        {
            format!("{host}/{rest}")
        }
        Some((host, rest)) if !host.contains('/') => {
            // `host:port/path`: drop the port.
            format!("{host}/{}", rest.split_once('/').map_or("", |(_, p)| p))
        }
        _ => without_user.to_owned(),
    };
    let lowered = joined.to_ascii_lowercase();
    let trimmed = lowered
        .trim_matches('/')
        .trim_end_matches(".git")
        .trim_matches('/');
    let segments: Vec<&str> = trimmed.split('/').collect();
    if segments.len() < 2
        || trimmed.len() > 200
        || segments.iter().any(|s| {
            s.is_empty()
                || *s == "."
                || *s == ".."
                || !s
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"-_.~".contains(&b))
        })
    {
        return Err(invalid());
    }
    Ok(trimmed.to_owned())
}
fn commit(value: &Value) -> ApiResult<String> {
    value
        .as_str()
        .map(str::to_ascii_lowercase)
        .filter(|c| c.len() == 40 && c.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(|| ApiError::bad("invalid_commit"))
}
fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 1024
        && !path.starts_with('/')
        && !path.contains('\0')
        && !path.chars().any(char::is_control)
        && path
            .split('/')
            .all(|s| !s.is_empty() && s != "." && s != "..")
}
/// Git's blob id for file content.
#[must_use]
pub fn blob_id(content: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(format!("blob {}\0", content.len()).as_bytes());
    hasher.update(content);
    hex::encode(hasher.finalize())
}

/// Resolve the snapshot a query runs against: the requested commit, or the
/// most recent ready snapshot (falling back to a pending one).
async fn snapshot(
    state: &AppState,
    c: &Credential,
    args: &Value,
) -> ApiResult<(String, String, String)> {
    let repo = repository_key(args["repo"].as_str().unwrap_or_default())?;
    let row = if args["commit"].is_string() {
        let commit = commit(&args["commit"])?;
        state
            .postgres
            .query_one_raw(statement(
                "SELECT commit_sha,status FROM code_snapshots WHERE owner=$1 AND repo=$2 AND commit_sha=$3",
                vec![c.wallet.clone().into(), repo.clone().into(), commit.into()],
            ))
            .await?
    } else {
        state
            .postgres
            .query_one_raw(statement(
                "SELECT commit_sha,status FROM code_snapshots WHERE owner=$1 AND repo=$2 ORDER BY (status='ready') DESC, updated_at DESC LIMIT 1",
                vec![c.wallet.clone().into(), repo.clone().into()],
            ))
            .await?
    };
    let row = row.ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "no_code_index_for_repository_run_sync_code_index",
        )
    })?;
    Ok((
        repo,
        row.try_get("", "commit_sha")?,
        row.try_get("", "status")?,
    ))
}

type Manifest = (Vec<String>, Vec<String>, Vec<i32>);
/// Validate one manifest chunk into parallel path, blob id and size columns.
fn manifest(limits: &crate::runtime::CodeIndex, files: &Value) -> ApiResult<Manifest> {
    let files = match files {
        Value::Null => return Ok((Vec::new(), Vec::new(), Vec::new())),
        Value::Array(items) if items.len() <= limits.manifest_files_per_call => items,
        _ => return Err(ApiError::bad("code_manifest_too_large_send_in_chunks")),
    };
    let (mut paths, mut shas, mut sizes) = (Vec::new(), Vec::new(), Vec::new());
    for file in files {
        let path = file["path"]
            .as_str()
            .filter(|p| valid_path(p))
            .ok_or_else(|| ApiError::bad("invalid_code_path"))?;
        let sha = commit_like(&file["sha"]).ok_or_else(|| ApiError::bad("invalid_blob_id"))?;
        let bytes = file["bytes"]
            .as_i64()
            .filter(|b| (0..=i64::try_from(limits.file_bytes).unwrap_or(i64::MAX)).contains(b))
            .ok_or_else(|| ApiError::bad("code_file_too_large_skip_it"))?;
        paths.push(path.to_owned());
        shas.push(sha);
        sizes.push(i32::try_from(bytes).unwrap_or(i32::MAX));
    }
    Ok((paths, shas, sizes))
}
async fn index_repository(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let repo = repository_key(args["repo"].as_str().unwrap_or_default())?;
    let commit = commit(&args["commit"])?;
    let reference = args["ref"]
        .as_str()
        .filter(|r| r.len() <= 200 && !r.chars().any(char::is_control))
        .map(str::to_owned);
    let (paths, shas, sizes) = manifest(limits, &args["files"])?;
    let at = now();
    let txn = state.postgres.begin().await?;
    txn.execute_raw(statement(
        "INSERT INTO code_snapshots(owner,repo,commit_sha,ref_name,status,created_at,updated_at) VALUES($1,$2,$3,$4,'pending',$5,$5) ON CONFLICT(owner,repo,commit_sha) DO UPDATE SET ref_name=COALESCE(EXCLUDED.ref_name, code_snapshots.ref_name), updated_at=EXCLUDED.updated_at",
        vec![c.wallet.clone().into(), repo.clone().into(), commit.clone().into(), reference.into(), at.into()],
    ))
    .await?;
    if args["reset"] == true {
        txn.execute_raw(statement(
            "DELETE FROM code_files WHERE owner=$1 AND repo=$2 AND commit_sha=$3",
            vec![
                c.wallet.clone().into(),
                repo.clone().into(),
                commit.clone().into(),
            ],
        ))
        .await?;
    }
    if !paths.is_empty() {
        txn.execute_raw(statement(
            "INSERT INTO code_files(owner,repo,commit_sha,path,sha,bytes) SELECT $1,$2,$3,p,s,b FROM unnest($4::text[],$5::text[],$6::int[]) AS t(p,s,b) ON CONFLICT(owner,repo,commit_sha,path) DO UPDATE SET sha=EXCLUDED.sha, bytes=EXCLUDED.bytes",
            vec![
                c.wallet.clone().into(),
                repo.clone().into(),
                commit.clone().into(),
                paths.into(),
                shas.clone().into(),
                sizes.into(),
            ],
        ))
        .await?;
    }
    let count: i64 = txn
        .query_one_raw(statement(
            "SELECT count(*) AS count FROM code_files WHERE owner=$1 AND repo=$2 AND commit_sha=$3",
            vec![
                c.wallet.clone().into(),
                repo.clone().into(),
                commit.clone().into(),
            ],
        ))
        .await?
        .ok_or_else(|| ApiError::unavailable("code_index"))?
        .try_get("", "count")?;
    if count > limits.files_per_snapshot {
        return Err(ApiError::bad("code_snapshot_file_limit"));
    }
    let missing: Vec<String> = txn
        .query_all_raw(statement(
            "SELECT DISTINCT s.sha FROM unnest($1::text[]) AS s(sha) WHERE NOT EXISTS (SELECT 1 FROM code_blobs b WHERE b.owner=$2 AND b.sha=s.sha)",
            vec![shas.into(), c.wallet.clone().into()],
        ))
        .await?
        .iter()
        .map(|row| row.try_get("", "sha"))
        .collect::<Result<_, _>>()?;
    txn.commit().await?;
    let mut ready = false;
    let mut summary = Value::Null;
    if args["finalize"] == true {
        summary = finalize(state, c, &repo, &commit).await?;
        ready = true;
    }
    Ok(json!({
        "repo": repo,
        "commit": commit,
        "files": count,
        "missing": missing,
        "status": if ready { "ready" } else { "pending" },
        "summary": summary,
        "upload_with": "upload_code_blobs",
        "blobs_per_upload": limits.blobs_per_upload,
    }))
}
fn commit_like(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_ascii_lowercase)
        .filter(|c| c.len() == 40 && c.bytes().all(|b| b.is_ascii_hexdigit()))
}
/// Compute the overview, mark the snapshot ready and prune old snapshots.
async fn finalize(state: &AppState, c: &Credential, repo: &str, commit: &str) -> ApiResult<Value> {
    let rows = state
        .postgres
        .query_all_raw(statement(
            "SELECT f.path, f.bytes::bigint AS bytes, b.language, CASE WHEN f.path ~ '(^|/)(package\\.json|Cargo\\.toml|pyproject\\.toml|requirements\\.txt|go\\.mod|Makefile|justfile|composer\\.json|Gemfile|pom\\.xml|build\\.gradle(\\.kts)?|deno\\.json|mix\\.exs|Package\\.swift)$' THEN b.content END AS content FROM code_files f LEFT JOIN code_blobs b ON b.owner=f.owner AND b.sha=f.sha WHERE f.owner=$1 AND f.repo=$2 AND f.commit_sha=$3",
            vec![c.wallet.clone().into(), repo.into(), commit.into()],
        ))
        .await?;
    let mut owned = Vec::with_capacity(rows.len());
    let mut manifests = Vec::new();
    for row in &rows {
        let path: String = row.try_get("", "path")?;
        let bytes: i64 = row.try_get("", "bytes")?;
        let language: Option<String> = row.try_get("", "language")?;
        if let Some(content) = row.try_get::<Option<String>>("", "content")?
            && overview::manifest(&path)
        {
            manifests.push((path.clone(), content));
        }
        owned.push((path, bytes, language));
    }
    let files: Vec<overview::File<'_>> = owned
        .iter()
        .map(|(path, bytes, language)| overview::File {
            path,
            bytes: *bytes,
            language: language
                .as_deref()
                .unwrap_or_else(|| symbols::language(path)),
        })
        .collect();
    let summary = overview::summarize(&files, &manifests);
    state
        .postgres
        .execute_raw(statement(
            "UPDATE code_snapshots SET status='ready', files=$4, bytes=$5, summary=$6, updated_at=$7 WHERE owner=$1 AND repo=$2 AND commit_sha=$3",
            vec![
                c.wallet.clone().into(),
                repo.into(),
                commit.into(),
                i32::try_from(files.len()).unwrap_or(i32::MAX).into(),
                summary["bytes"].as_i64().unwrap_or(0).into(),
                summary.clone().into(),
                now().into(),
            ],
        ))
        .await?;
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM code_snapshots WHERE owner=$1 AND repo=$2 AND commit_sha IN (SELECT commit_sha FROM code_snapshots WHERE owner=$1 AND repo=$2 ORDER BY updated_at DESC OFFSET $3)",
            vec![c.wallet.clone().into(), repo.into(), limits(state).snapshots_per_repo.into()],
        ))
        .await?;
    Ok(summary)
}

async fn upload_blobs(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let blobs = args["blobs"]
        .as_array()
        .filter(|b| !b.is_empty() && b.len() <= limits.blobs_per_upload)
        .ok_or_else(|| ApiError::bad("invalid_code_blobs"))?;
    let mut prepared = Vec::with_capacity(blobs.len());
    let mut incoming = 0i64;
    for blob in blobs {
        let content = blob["content"]
            .as_str()
            .filter(|content| content.len() <= limits.file_bytes)
            .ok_or_else(|| ApiError::bad("invalid_code_blob_content"))?;
        let sha = commit_like(&blob["sha"]).ok_or_else(|| ApiError::bad("invalid_blob_id"))?;
        if blob_id(content.as_bytes()) != sha {
            return Err(ApiError::bad("blob_content_does_not_match_its_id"));
        }
        let path = blob["path"]
            .as_str()
            .filter(|p| valid_path(p))
            .unwrap_or_default();
        let language = symbols::language(path);
        incoming += i64::try_from(content.len()).unwrap_or(i64::MAX);
        prepared.push((sha, content, language));
    }
    let used: i64 = state
        .postgres
        .query_one_raw(statement(
            "SELECT COALESCE(sum(bytes),0)::bigint AS used FROM code_blobs WHERE owner=$1",
            vec![c.wallet.clone().into()],
        ))
        .await?
        .ok_or_else(|| ApiError::unavailable("code_index"))?
        .try_get("", "used")?;
    if used + incoming > limits.owner_storage_bytes {
        return Err(ApiError::bad("code_index_storage_full_delete_old_indexes"));
    }
    let at = now();
    let mut stored = 0;
    for (sha, content, language) in prepared {
        let txn = state.postgres.begin().await?;
        let inserted = txn
            .execute_raw(statement(
                "INSERT INTO code_blobs(owner,sha,bytes,language,content,search,created_at) VALUES($1,$2,$3,$4,$5,to_tsvector('simple',$6),$7) ON CONFLICT(owner,sha) DO NOTHING",
                vec![
                    c.wallet.clone().into(),
                    sha.clone().into(),
                    i32::try_from(content.len()).unwrap_or(i32::MAX).into(),
                    language.into(),
                    content.to_owned().into(),
                    symbols::search_text(content, 512 * 1024).into(),
                    at.into(),
                ],
            ))
            .await?;
        if inserted.rows_affected() == 1 {
            stored += 1;
            let found = symbols::extract(language, content, limits.symbols_per_file);
            if !found.is_empty() {
                let (mut names, mut lower, mut kinds, mut lines, mut signatures) =
                    (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
                for symbol in found {
                    lower.push(symbol.name.to_lowercase());
                    names.push(symbol.name);
                    kinds.push(symbol.kind.to_owned());
                    lines.push(symbol.line);
                    signatures.push(symbol.signature);
                }
                txn.execute_raw(statement(
                    "INSERT INTO code_symbols(owner,sha,name,lname,kind,line,signature) SELECT $1,$2,n,l,k,ln,s FROM unnest($3::text[],$4::text[],$5::text[],$6::int[],$7::text[]) AS t(n,l,k,ln,s) ON CONFLICT DO NOTHING",
                    vec![
                        c.wallet.clone().into(),
                        sha.into(),
                        names.into(),
                        lower.into(),
                        kinds.into(),
                        lines.into(),
                        signatures.into(),
                    ],
                ))
                .await?;
            }
        }
        txn.commit().await?;
    }
    Ok(json!({"stored":stored,"received":blobs.len()}))
}

fn like_prefix(value: &str) -> String {
    let mut escaped = value
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_");
    escaped.push('%');
    escaped
}
/// Matching lines with 1-based numbers, for search results and references.
fn matching_lines(content: &str, matches: impl Fn(&str) -> bool, max: usize) -> Vec<Value> {
    content
        .lines()
        .enumerate()
        .filter(|(_, line)| matches(line))
        .take(max)
        .map(|(index, line)| json!({"line":index + 1,"text":line.trim().chars().take(240).collect::<String>()}))
        .collect()
}
fn word_match(line: &str, word: &str) -> bool {
    let bytes = line.as_bytes();
    let boundary = |b: u8| !(b.is_ascii_alphanumeric() || b == b'_' || b == b'$');
    line.match_indices(word).any(|(start, _)| {
        let end = start + word.len();
        (start == 0 || boundary(bytes[start - 1])) && (end == bytes.len() || boundary(bytes[end]))
    })
}

async fn code_search(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let (repo, commit, status) = snapshot(state, c, args).await?;
    let query = args["query"]
        .as_str()
        .map(str::trim)
        .filter(|q| !q.is_empty() && q.len() <= 400)
        .ok_or_else(|| ApiError::bad("invalid_code_query"))?;
    let limit = args["limit"]
        .as_u64()
        .unwrap_or(limits.search_results)
        .clamp(1, limits.search_results);
    let prefix = args["path_prefix"]
        .as_str()
        .filter(|p| p.len() <= 512)
        .unwrap_or_default();
    let language = args["language"]
        .as_str()
        .filter(|l| l.len() <= 32)
        .unwrap_or_default();
    let literal = args["mode"] == "literal";
    let terms = symbols::query_terms(query);
    if !literal && terms.is_empty() {
        return Err(ApiError::bad("invalid_code_query"));
    }
    let fetch = i64::try_from(limit * 2).unwrap_or(40);
    let common = "f.owner=$1 AND f.repo=$2 AND f.commit_sha=$3 AND ($4='' OR f.path LIKE $5) AND ($6='' OR b.language=$6)";
    let base: Vec<sea_orm::Value> = vec![
        c.wallet.clone().into(),
        repo.clone().into(),
        commit.clone().into(),
        prefix.to_owned().into(),
        like_prefix(prefix).into(),
        language.to_owned().into(),
        fetch.into(),
    ];
    let rows: Vec<QueryResult> = if literal {
        let mut values = base;
        values.push(query.to_lowercase().into());
        state.postgres.query_all_raw(statement(
            &format!("SELECT f.path, b.language, b.content, 0::real AS rank FROM code_files f JOIN code_blobs b ON b.owner=f.owner AND b.sha=f.sha WHERE {common} AND strpos(lower(b.content), $8) > 0 ORDER BY f.path LIMIT $7"),
            values,
        )).await?
    } else {
        let mut values = base;
        values.push(terms.join(" & ").into());
        state.postgres.query_all_raw(statement(
            &format!("SELECT f.path, b.language, b.content, ts_rank(b.search, to_tsquery('simple',$8)) AS rank FROM code_files f JOIN code_blobs b ON b.owner=f.owner AND b.sha=f.sha WHERE {common} AND b.search @@ to_tsquery('simple',$8) ORDER BY rank DESC, f.path LIMIT $7"),
            values,
        )).await?
    };
    let needle = query.to_lowercase();
    let mut results = Vec::new();
    for row in &rows {
        let path: String = row.try_get("", "path")?;
        let content: String = row.try_get("", "content")?;
        let rank: f32 = row.try_get("", "rank")?;
        let lower_path = path.to_lowercase();
        let path_hits = terms
            .iter()
            .filter(|t| lower_path.contains(t.as_str()))
            .count();
        let snippets = if literal {
            matching_lines(
                &content,
                |line| line.to_lowercase().contains(&needle),
                limits.snippets_per_file,
            )
        } else {
            matching_lines(
                &content,
                |line| {
                    let line = line.to_lowercase();
                    terms.iter().any(|t| line.contains(t.as_str()))
                },
                limits.snippets_per_file,
            )
        };
        #[allow(clippy::cast_precision_loss)]
        let score = f64::from(rank) + path_hits as f64 * 0.5;
        results.push((score, json!({"path":path,"language":row.try_get::<String>("", "language")?,"snippets":snippets})));
    }
    results.sort_by(|a, b| b.0.total_cmp(&a.0));
    results.truncate(usize::try_from(limit).unwrap_or(20));
    Ok(json!({
        "repo": repo,
        "commit": commit,
        "index_status": status,
        "results": results.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
        "read_with": "read_code",
    }))
}

async fn find_symbol(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let (repo, commit, _) = snapshot(state, c, args).await?;
    let name = args["name"]
        .as_str()
        .map(str::trim)
        .filter(|n| !n.is_empty() && n.len() <= 200)
        .ok_or_else(|| ApiError::bad("invalid_symbol_name"))?
        .to_lowercase();
    let exact = args["exact"].as_bool().unwrap_or(false);
    let kind = args["kind"]
        .as_str()
        .filter(|k| k.len() <= 32)
        .unwrap_or_default();
    let limit = i64::try_from(args["limit"].as_u64().unwrap_or(20).clamp(1, 100)).unwrap_or(20);
    let rows = state.postgres.query_all_raw(statement(
        "SELECT f.path, s.name, s.kind, s.line, s.signature FROM code_symbols s JOIN code_files f ON f.owner=s.owner AND f.sha=s.sha WHERE s.owner=$1 AND f.repo=$2 AND f.commit_sha=$3 AND (s.lname=$4 OR (NOT $5 AND s.lname LIKE $6)) AND ($7='' OR s.kind=$7) ORDER BY (s.lname=$4) DESC, length(s.name), f.path, s.line LIMIT $8",
        vec![
            c.wallet.clone().into(),
            repo.clone().into(),
            commit.clone().into(),
            name.clone().into(),
            exact.into(),
            like_prefix(&name).into(),
            kind.to_owned().into(),
            limit.into(),
        ],
    )).await?;
    let symbols = rows
        .iter()
        .map(|row| {
            Ok(json!({
                "path": row.try_get::<String>("", "path")?,
                "name": row.try_get::<String>("", "name")?,
                "kind": row.try_get::<String>("", "kind")?,
                "line": row.try_get::<i32>("", "line")?,
                "signature": row.try_get::<String>("", "signature")?,
            }))
        })
        .collect::<ApiResult<Vec<_>>>()?;
    Ok(json!({"repo":repo,"commit":commit,"symbols":symbols}))
}

async fn find_references(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let (repo, commit, _) = snapshot(state, c, args).await?;
    let symbol = args["symbol"]
        .as_str()
        .map(str::trim)
        .filter(|s| {
            !s.is_empty()
                && s.len() <= 200
                && s.chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        })
        .ok_or_else(|| ApiError::bad("invalid_symbol_name"))?;
    let limit = usize::try_from(args["limit"].as_u64().unwrap_or(50).clamp(1, 200)).unwrap_or(50);
    let whole: String = symbol
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase();
    let rows = state.postgres.query_all_raw(statement(
        "SELECT f.path, b.content FROM code_files f JOIN code_blobs b ON b.owner=f.owner AND b.sha=f.sha WHERE f.owner=$1 AND f.repo=$2 AND f.commit_sha=$3 AND b.search @@ to_tsquery('simple',$4) ORDER BY f.path LIMIT $5",
        vec![
            c.wallet.clone().into(),
            repo.clone().into(),
            commit.clone().into(),
            whole.into(),
            i64::try_from(limits.search_results * 5).unwrap_or(100).into(),
        ],
    )).await?;
    let mut references = Vec::new();
    'files: for row in &rows {
        let path: String = row.try_get("", "path")?;
        let content: String = row.try_get("", "content")?;
        for hit in matching_lines(&content, |line| word_match(line, symbol), limit) {
            references.push(json!({"path":path,"line":hit["line"],"text":hit["text"]}));
            if references.len() >= limit {
                break 'files;
            }
        }
    }
    Ok(
        json!({"repo":repo,"commit":commit,"symbol":symbol,"references":references,"truncated":references.len() >= limit}),
    )
}

async fn read_code(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let limits = limits(state);
    let (repo, commit, _) = snapshot(state, c, args).await?;
    let path = args["path"]
        .as_str()
        .filter(|p| valid_path(p))
        .ok_or_else(|| ApiError::bad("invalid_code_path"))?;
    let row = state.postgres.query_one_raw(statement(
        "SELECT b.content, b.language FROM code_files f JOIN code_blobs b ON b.owner=f.owner AND b.sha=f.sha WHERE f.owner=$1 AND f.repo=$2 AND f.commit_sha=$3 AND f.path=$4",
        vec![c.wallet.clone().into(), repo.clone().into(), commit.clone().into(), path.into()],
    )).await?.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "code_file_not_indexed"))?;
    let content: String = row.try_get("", "content")?;
    let lines: Vec<&str> = content.lines().collect();
    let start = usize::try_from(args["start_line"].as_u64().unwrap_or(1).max(1)).unwrap_or(1);
    let requested_end =
        usize::try_from(args["end_line"].as_u64().unwrap_or(u64::MAX)).unwrap_or(usize::MAX);
    let end = requested_end
        .min(start.saturating_add(limits.read_lines) - 1)
        .min(lines.len());
    let width = end.to_string().len();
    let text = lines
        .iter()
        .enumerate()
        .skip(start - 1)
        .take(end.saturating_sub(start - 1))
        .map(|(index, line)| format!("{:>width$} | {line}", index + 1))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(json!({
        "repo": repo,
        "commit": commit,
        "path": path,
        "language": row.try_get::<String>("", "language")?,
        "start_line": start,
        "end_line": end,
        "total_lines": lines.len(),
        "truncated": end < lines.len().min(requested_end),
        "text": text,
    }))
}

async fn repo_overview(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let (repo, commit, status) = snapshot(state, c, args).await?;
    let row = state.postgres.query_one_raw(statement(
        "SELECT summary, ref_name, files, updated_at FROM code_snapshots WHERE owner=$1 AND repo=$2 AND commit_sha=$3",
        vec![c.wallet.clone().into(), repo.clone().into(), commit.clone().into()],
    )).await?.ok_or_else(|| ApiError::new(StatusCode::NOT_FOUND, "no_code_index_for_repository"))?;
    let recent = state.postgres.query_all_raw(statement(
        "SELECT commit_sha, ref_name, status, updated_at FROM code_snapshots WHERE owner=$1 AND repo=$2 ORDER BY updated_at DESC LIMIT 5",
        vec![c.wallet.clone().into(), repo.clone().into()],
    )).await?;
    let memories = memory::recall_for(
        state,
        &c.wallet,
        &[repo.clone(), memory::GLOBAL.to_owned()],
        None,
        &[],
        &[],
        10,
    )
    .await?;
    Ok(json!({
        "repo": repo,
        "commit": commit,
        "ref": row.try_get::<Option<String>>("", "ref_name")?,
        "index_status": status,
        "indexed_at": row.try_get::<i64>("", "updated_at")?,
        "summary": row.try_get::<Value>("", "summary")?,
        "recent_snapshots": recent.iter().map(|r| Ok(json!({
            "commit": r.try_get::<String>("", "commit_sha")?,
            "ref": r.try_get::<Option<String>>("", "ref_name")?,
            "status": r.try_get::<String>("", "status")?,
            "updated_at": r.try_get::<i64>("", "updated_at")?,
        }))).collect::<ApiResult<Vec<_>>>()?,
        "memories": memories,
    }))
}

async fn list_indexes(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let repo = args["repo"].as_str().map(repository_key).transpose()?;
    let rows = state.postgres.query_all_raw(statement(
        "SELECT repo, commit_sha, ref_name, status, files, bytes, updated_at FROM code_snapshots WHERE owner=$1 AND ($2::text IS NULL OR repo=$2) ORDER BY updated_at DESC LIMIT 50",
        vec![c.wallet.clone().into(), repo.into()],
    )).await?;
    let used: i64 = state
        .postgres
        .query_one_raw(statement(
            "SELECT COALESCE(sum(bytes),0)::bigint AS used FROM code_blobs WHERE owner=$1",
            vec![c.wallet.clone().into()],
        ))
        .await?
        .ok_or_else(|| ApiError::unavailable("code_index"))?
        .try_get("", "used")?;
    Ok(json!({
        "snapshots": rows.iter().map(|r| Ok(json!({
            "repo": r.try_get::<String>("", "repo")?,
            "commit": r.try_get::<String>("", "commit_sha")?,
            "ref": r.try_get::<Option<String>>("", "ref_name")?,
            "status": r.try_get::<String>("", "status")?,
            "files": r.try_get::<i32>("", "files")?,
            "bytes": r.try_get::<i64>("", "bytes")?,
            "updated_at": r.try_get::<i64>("", "updated_at")?,
        }))).collect::<ApiResult<Vec<_>>>()?,
        "storage": {"used_bytes": used, "limit_bytes": limits(state).owner_storage_bytes},
    }))
}

async fn delete_index(state: &AppState, c: &Credential, args: &Value) -> ApiResult<Value> {
    let repo = repository_key(args["repo"].as_str().unwrap_or_default())?;
    let commit = args["commit"]
        .is_string()
        .then(|| commit(&args["commit"]))
        .transpose()?;
    let deleted = state
        .postgres
        .execute_raw(statement(
            "DELETE FROM code_snapshots WHERE owner=$1 AND repo=$2 AND ($3::text IS NULL OR commit_sha=$3)",
            vec![c.wallet.clone().into(), repo.into(), commit.into()],
        ))
        .await?;
    Ok(json!({"deleted_snapshots":deleted.rows_affected()}))
}

/// The stack detected in the latest ready snapshot, for discovery ranking.
/// # Errors
/// Returns database errors.
pub async fn stack(state: &AppState, owner: &str, repo: &str) -> ApiResult<Vec<String>> {
    let Ok(repo) = repository_key(repo) else {
        return Ok(Vec::new());
    };
    let row = state
        .postgres
        .query_one_raw(statement(
            "SELECT summary FROM code_snapshots WHERE owner=$1 AND repo=$2 AND status='ready' ORDER BY updated_at DESC LIMIT 1",
            vec![owner.into(), repo.into()],
        ))
        .await?;
    let summary = row
        .map(|row| row.try_get::<Value>("", "summary"))
        .transpose()?
        .unwrap_or(Value::Null);
    Ok(summary["stack"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str().map(str::to_owned))
        .collect())
}

/// Code intelligence tools and the scope each requires.
pub const TOOLS: &[(&str, &str)] = &[
    ("index_repository", "code:index"),
    ("upload_code_blobs", "code:index"),
    ("delete_code_index", "code:index"),
    ("code_search", "code:read"),
    ("find_symbol", "code:read"),
    ("find_references", "code:read"),
    ("read_code", "code:read"),
    ("repo_overview", "code:read"),
    ("list_code_indexes", "code:read"),
];
/// Dispatch a code intelligence tool. Scopes are checked by the gateway.
/// # Errors
/// Rejects invalid input and database failures.
pub async fn call(state: &AppState, c: &Credential, name: &str, args: &Value) -> ApiResult<Value> {
    match name {
        "index_repository" => index_repository(state, c, args).await,
        "upload_code_blobs" => upload_blobs(state, c, args).await,
        "delete_code_index" => delete_index(state, c, args).await,
        "code_search" => code_search(state, c, args).await,
        "find_symbol" => find_symbol(state, c, args).await,
        "find_references" => find_references(state, c, args).await,
        "read_code" => read_code(state, c, args).await,
        "repo_overview" => repo_overview(state, c, args).await,
        "list_code_indexes" => list_indexes(state, c, args).await,
        _ => Err(ApiError::bad("unknown_code_operation")),
    }
}

/// Garbage-collect unreferenced blobs and abandoned pending snapshots.
/// # Errors
/// Returns database errors.
pub async fn cleanup(state: &AppState) -> ApiResult<()> {
    let at = now();
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM code_blobs WHERE (owner, sha) IN (SELECT b.owner, b.sha FROM code_blobs b WHERE b.created_at<$1 AND NOT EXISTS (SELECT 1 FROM code_files f WHERE f.owner=b.owner AND f.sha=b.sha) LIMIT 1000)",
            vec![(at - 3600).into()],
        ))
        .await?;
    state
        .postgres
        .execute_raw(statement(
            "DELETE FROM code_snapshots WHERE status='pending' AND updated_at<$1",
            vec![(at - 7 * 86_400).into()],
        ))
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repository_references_normalize_to_one_key() -> ApiResult<()> {
        for raw in [
            "https://github.com/Acme/App.git",
            "git@github.com:acme/app.git",
            "ssh://git@github.com/acme/app",
            "github.com/acme/app/",
            "https://token@github.com/acme/app",
            "ssh://git@github.com:22/acme/app.git",
        ] {
            assert_eq!(repository_key(raw)?, "github.com/acme/app", "{raw}");
        }
        assert_eq!(repository_key("local/my-tool")?, "local/my-tool");
        for bad in [
            "",
            "app",
            "github.com/../etc",
            "github.com/a b/c",
            "github.com//x",
        ] {
            assert!(repository_key(bad).is_err(), "{bad}");
        }
        Ok(())
    }
    #[test]
    fn blob_ids_match_git() {
        // `printf 'hello\n' | git hash-object --stdin`
        assert_eq!(
            blob_id(b"hello\n"),
            "ce013625030ba8dba906f756967f9e9ca394464a"
        );
        assert_eq!(blob_id(b""), "e69de29bb2d1d6434b8b29ae775ad8c2e48c5391");
    }
    #[test]
    fn paths_and_matches_are_safe() {
        assert!(valid_path("src/main.rs"));
        for bad in ["/etc/passwd", "../x", "a//b", "a/./b", ""] {
            assert!(!valid_path(bad), "{bad}");
        }
        assert!(word_match("let user = getUser();", "getUser"));
        assert!(!word_match("let user = getUserName();", "getUser"));
        assert_eq!(like_prefix("a_b%"), "a\\_b\\%%");
        let lines = matching_lines("a\nfoo\nbar foo", |l| l.contains("foo"), 5);
        assert_eq!(lines[0]["line"], 2);
        assert_eq!(lines.len(), 2);
    }
}
