//! `search_tools`: rank every tool the caller can reach for a short intent.
//!
//! The score adds four bounded signals, each weighted from configuration:
//! - lexical: BM25 over stemmed name, description and operator guidance tokens;
//! - semantic: cosine similarity of embeddings, when an embeddings provider is configured;
//! - usage: how often this account (and, weaker, everyone) called the tool successfully;
//! - stack: overlap with the caller's declared or indexed project stack.
//!
//! Operator-registered entry points (for example a provider's search tool) get
//! a small boost when they are otherwise relevant.
use super::embeddings;
use crate::{
    features::{
        codeintel::{self, symbols::split_identifier},
        common::{
            error::{ApiError, ApiResult},
            model::{Credential, now},
        },
        integrations,
    },
    state::AppState,
};
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// Light suffix stemming so `templates`/`template` and `searching`/`search` meet.
fn stem(word: &str) -> String {
    let word = word.to_lowercase();
    let len = word.len();
    if len > 4 && word.ends_with("ies") {
        return format!("{}y", &word[..len - 3]);
    }
    for (suffix, min) in [("ing", 6), ("ed", 5)] {
        if len >= min && word.ends_with(suffix) {
            return word[..len - suffix.len()].to_owned();
        }
    }
    if len > 4
        && ["ses", "xes", "ches", "shes"]
            .iter()
            .any(|s| word.ends_with(s))
    {
        return word[..len - 2].to_owned();
    }
    if len > 3 && word.ends_with('s') && !word.ends_with("ss") {
        return word[..len - 1].to_owned();
    }
    word
}
fn tokens(text: &str) -> Vec<String> {
    text.split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-'))
        .flat_map(|word| {
            let parts = split_identifier(word);
            if parts.is_empty() {
                vec![word.to_lowercase()]
            } else {
                parts
            }
        })
        .filter(|t| t.len() > 1)
        .map(|t| stem(&t))
        .collect()
}
/// Intent terms: stemmed, de-duplicated and without configured stopwords.
pub(super) fn terms(state: &AppState, query: &str) -> Vec<String> {
    let config = &state.settings.runtime.discovery;
    let mut seen = HashSet::new();
    tokens(query)
        .into_iter()
        .filter(|t| !config.stopwords.iter().any(|s| stem(s) == *t))
        .filter(|t| seen.insert(t.clone()))
        .take(config.max_query_terms)
        .collect()
}
/// The searchable name part, without Hive routing prefixes.
fn display_name(name: &str) -> &str {
    name.strip_prefix("mcp_").map_or(name, |rest| {
        rest.split_once("__").map_or(rest, |(_, tool)| tool)
    })
}
fn provider_context(name: &str) -> String {
    integrations::registry()
        .iter()
        .filter(|p| {
            name.starts_with(&format!("mcp_{}__", p.slug)) && integrations::is_search_entry(name)
        })
        .map(|p| {
            format!(
                "{} {} {}",
                p.name,
                p.description,
                p.discovery_keywords.join(" ")
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn search_description(tool: &Value) -> String {
    let text = tool["description"].as_str().unwrap_or_default();
    let metadata = text
        .split_once("untrusted data: ")
        .and_then(|(_, raw)| serde_json::from_str::<Value>(raw).ok());
    let text = metadata
        .as_ref()
        .and_then(|v| v["description"].as_str())
        .unwrap_or(text);
    text.lines()
        .next()
        .unwrap_or_default()
        .chars()
        .take(500)
        .collect()
}
fn intent_score(name: &str, terms: &[String]) -> f64 {
    if !integrations::is_search_entry(name) {
        return 0.0;
    }
    integrations::registry()
        .iter()
        .filter(|p| name.starts_with(&format!("mcp_{}__", p.slug)))
        .map(|p| {
            let keywords: HashSet<_> = p
                .discovery_keywords
                .iter()
                .flat_map(|v| tokens(v))
                .collect();
            let hits = terms.iter().filter(|v| keywords.contains(*v)).count();
            if hits == 0 {
                0.0
            } else {
                1.0 + f64::from(u32::try_from(hits.min(4)).unwrap_or(4)) * 0.25
            }
        })
        .sum()
}
/// Only return workflows whose routed dependencies are currently reachable. Native steps
/// still enforce their own repository prerequisites at execution time.
fn reachable(tool: &Value, names: &HashSet<String>) -> bool {
    tool["_meta"]["dev.hive/requiredTools"]
        .as_array()
        .is_none_or(|steps| {
            steps
                .iter()
                .filter_map(Value::as_str)
                .all(|name| names.contains(name))
        })
}

struct Document {
    weights: HashMap<String, f64>,
    length: f64,
}
fn document(tool: &Value) -> Document {
    let name = tool["name"].as_str().unwrap_or_default();
    let mut weights: HashMap<String, f64> = HashMap::new();
    let mut length = 0.0;
    let fields = [
        (format!("{} {}", display_name(name), name), 3.0),
        (search_description(tool), 1.0),
        (provider_context(name), 0.5),
    ];
    for (text, weight) in fields {
        for token in tokens(&text) {
            *weights.entry(token).or_default() += weight;
            length += weight;
        }
    }
    Document { weights, length }
}
/// BM25 per document over the candidate set, normalized to `[0, 1]`.
fn bm25(documents: &[Document], terms: &[String]) -> Vec<f64> {
    const K1: f64 = 1.2;
    const B: f64 = 0.75;
    #[allow(clippy::cast_precision_loss)]
    let count = documents.len().max(1) as f64;
    let average = documents.iter().map(|d| d.length).sum::<f64>() / count;
    let scores: Vec<f64> = documents
        .iter()
        .map(|doc| {
            terms
                .iter()
                .map(|term| {
                    let frequency = doc.weights.get(term).copied().unwrap_or(0.0);
                    if frequency == 0.0 {
                        return 0.0;
                    }
                    #[allow(clippy::cast_precision_loss)]
                    let containing = documents
                        .iter()
                        .filter(|d| d.weights.contains_key(term))
                        .count() as f64;
                    let idf = ((count - containing + 0.5) / (containing + 0.5) + 1.0).ln();
                    idf * frequency * (K1 + 1.0)
                        / (frequency + K1 * (1.0 - B + B * doc.length / average.max(1.0)))
                })
                .sum()
        })
        .collect();
    let max = scores.iter().copied().fold(0.0, f64::max);
    scores
        .into_iter()
        .map(|s| if max > 0.0 { s / max } else { 0.0 })
        .collect()
}
fn embedding_text(tool: &Value) -> String {
    let name = tool["name"].as_str().unwrap_or_default();
    let description: String = tool["description"]
        .as_str()
        .unwrap_or_default()
        .chars()
        .take(1200)
        .collect();
    format!(
        "{} ({}): {description}",
        split_identifier(display_name(name)).join(" "),
        name
    )
}

/// Record one call for ranking. Best effort: ranking data must never fail a call.
pub async fn record_usage(state: &AppState, owner: &str, tool: &str, failed: bool) {
    if !super::discoverable(tool) {
        return;
    }
    let outcome = state.postgres.execute_raw(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "INSERT INTO tool_usage(owner,tool,calls,failures,last_used_at) VALUES($1,$2,1,$3,$4),('*',$2,1,$3,$4) ON CONFLICT(owner,tool) DO UPDATE SET calls=tool_usage.calls+1, failures=tool_usage.failures+EXCLUDED.failures, last_used_at=EXCLUDED.last_used_at",
        vec![owner.into(), tool.into(), i64::from(failed).into(), now().into()],
    )).await;
    if let Err(error) = outcome {
        tracing::warn!(%error, "Tool usage was not recorded");
    }
}
/// `ln(1+successes)` for the caller and for everyone, squashed into `[0, 1)`.
async fn usage(state: &AppState, owner: &str, names: &[String]) -> ApiResult<HashMap<String, f64>> {
    let rows = state
        .postgres
        .query_all_raw(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT owner, tool, calls, failures FROM tool_usage WHERE tool=ANY($1) AND owner IN ($2,'*')",
            vec![names.to_vec().into(), owner.into()],
        ))
        .await?;
    let mut raw: HashMap<String, f64> = HashMap::new();
    for row in rows {
        let who: String = row.try_get("", "owner")?;
        let tool: String = row.try_get("", "tool")?;
        let calls: i64 = row.try_get("", "calls")?;
        let failures: i64 = row.try_get("", "failures")?;
        #[allow(clippy::cast_precision_loss)]
        let successes = (calls - failures).max(0) as f64;
        let weight = if who == "*" { 0.4 } else { 1.0 };
        *raw.entry(tool).or_default() += weight * successes.ln_1p();
    }
    Ok(raw
        .into_iter()
        .map(|(tool, value)| (tool, value / (1.0 + value)))
        .collect())
}
fn stack_overlap(tool: &Value, stack: &[String]) -> f64 {
    if stack.is_empty() {
        return 0.0;
    }
    let text = format!(
        "{} {}",
        tool["name"].as_str().unwrap_or_default(),
        tool["description"].as_str().unwrap_or_default()
    )
    .to_lowercase();
    #[allow(clippy::cast_precision_loss)]
    let hits = stack.iter().filter(|s| text.contains(s.as_str())).count() as f64;
    #[allow(clippy::cast_precision_loss)]
    let total = stack.len().min(4) as f64;
    (hits / total).min(1.0)
}
async fn caller_stack(
    state: &AppState,
    credential: &Credential,
    args: &Value,
) -> ApiResult<Vec<String>> {
    let mut stack: Vec<String> = args["stack"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|s| s.as_str())
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty() && s.len() <= 40)
        .take(12)
        .collect();
    if let Some(repo) = args["repo"].as_str()
        && credential.scopes.iter().any(|s| s == "code:read")
    {
        stack.extend(codeintel::stack(state, &credential.wallet, repo).await?);
    }
    stack.sort();
    stack.dedup();
    stack.truncate(16);
    Ok(stack)
}

async fn candidates(
    state: &AppState,
    credential: &Credential,
    query: &str,
) -> ApiResult<Vec<Value>> {
    let mut tools = super::tools::tools(state, credential, Some(query), true).await?["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let dependencies: Vec<uuid::Uuid> = tools
        .iter()
        .flat_map(|tool| {
            tool["_meta"]["dev.hive/requiredTools"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .filter_map(|name| {
            name.as_str()
                .and_then(|name| name.strip_prefix("hive_"))
                .and_then(|id| id.parse().ok())
        })
        .collect();
    let mut callable =
        super::tools::tools_with_dependencies(state, credential, Some(query), true, &dependencies)
            .await?["tools"]
            .as_array()
            .cloned()
            .unwrap_or_default();
    let available = callable_names(&mut callable);
    tools.retain(|tool| {
        tool["name"]
            .as_str()
            .is_some_and(|name| super::discoverable(name) && available.contains(name))
            && tool["_meta"]["dev.hive/requiresBinding"] != true
    });
    Ok(tools)
}

/// Execution routes include callable-only native tools and connection-bound catalog
/// wrappers. Discovery visibility must not remove them from a workflow's dependencies.
fn callable_names(tools: &mut Vec<Value>) -> HashSet<String> {
    loop {
        let available: HashSet<String> = tools
            .iter()
            .filter_map(|tool| tool["name"].as_str().map(str::to_owned))
            .collect();
        let previous = tools.len();
        tools.retain(|tool| reachable(tool, &available));
        if tools.len() == previous {
            return available;
        }
    }
}

async fn semantic_scores(state: &AppState, query: &str, tools: &[Value]) -> Vec<f64> {
    if state.settings.embeddings.is_some() && !tools.is_empty() {
        let mut texts = vec![format!("query: {query}")];
        texts.extend(tools.iter().map(embedding_text));
        match embeddings::vectors(state, &texts).await {
            Some(vectors) => vectors[1..]
                .iter()
                .map(|v| embeddings::similarity(&vectors[0], v))
                .collect(),
            None => vec![0.0; tools.len()],
        }
    } else {
        vec![0.0; tools.len()]
    }
}

pub(super) async fn search(
    state: &AppState,
    credential: &Credential,
    args: &Value,
) -> ApiResult<Value> {
    let config = &state.settings.runtime.discovery;
    let query = args["query"]
        .as_str()
        .filter(|q| !q.trim().is_empty() && q.len() <= 200)
        .ok_or_else(|| ApiError::bad("invalid_tool_query"))?;
    let limit = args["limit"].as_u64().unwrap_or(config.default_results);
    if !(1..=config.max_results).contains(&limit) {
        return Err(ApiError::bad("invalid_tool_limit"));
    }
    let terms = terms(state, query);
    if terms.is_empty() {
        return Err(ApiError::bad("tool_query_too_general"));
    }
    let tools = candidates(state, credential, query).await?;
    let documents: Vec<Document> = tools.iter().map(document).collect();
    let lexical = bm25(&documents, &terms);
    let semantic = semantic_scores(state, query, &tools).await;
    let names: Vec<String> = tools
        .iter()
        .filter_map(|t| t["name"].as_str().map(str::to_owned))
        .collect();
    let usage = usage(state, &credential.wallet, &names).await?;
    let stack = caller_stack(state, credential, args).await?;
    let mut ranked: Vec<(f64, Value, Value)> = Vec::new();
    for (index, tool) in tools.into_iter().enumerate() {
        let name = tool["name"].as_str().unwrap_or_default().to_owned();
        let intent = intent_score(&name, &terms);
        let semantic_hit = semantic[index] >= config.semantic_floor;
        if lexical[index] <= 0.0 && !semantic_hit && intent == 0.0 {
            continue;
        }
        let used = usage.get(&name).copied().unwrap_or(0.0);
        let overlap = stack_overlap(&tool, &stack);
        let entry = if integrations::is_search_entry(&name) {
            0.25
        } else {
            0.0
        };
        let semantic_score = if semantic_hit { semantic[index] } else { 0.0 };
        let score = config.lexical_weight * lexical[index]
            + config.semantic_weight * semantic_score
            + config.usage_weight * used
            + config.stack_weight * overlap
            + entry
            + intent;
        let signals = json!({
            "lexical": (lexical[index] * 1000.0).round() / 1000.0,
            "semantic": (semantic_score * 1000.0).round() / 1000.0,
            "usage": (used * 1000.0).round() / 1000.0,
            "stack": (overlap * 1000.0).round() / 1000.0,
        });
        ranked.push((score, tool, signals));
    }
    ranked.sort_by(|a, b| {
        b.0.total_cmp(&a.0)
            .then_with(|| a.1["name"].as_str().cmp(&b.1["name"].as_str()))
    });
    ranked.truncate(usize::try_from(limit).unwrap_or(6));
    let mut bytes = 0;
    ranked.retain(|(_, tool, _)| {
        let size = tool.to_string().len();
        if bytes + size > config.response_bytes {
            return false;
        }
        bytes += size;
        true
    });
    let ranking: Vec<Value> = ranked
        .iter()
        .map(|(score, tool, signals)| json!({"name":tool["name"],"score":(score * 1000.0).round() / 1000.0,"signals":signals}))
        .collect();
    let tools: Vec<Value> = ranked.into_iter().map(|(_, tool, _)| tool).collect();
    let setup = setup_hints(&terms, &names);
    let guidance: Vec<_> = integrations::registry().iter().filter(|item| {
        tools.iter().any(|tool| tool["name"].as_str().is_some_and(|name|name.starts_with(&format!("mcp_{}__",item.slug)) || name == format!("hive_{}",item.id.simple())))
    }).map(|item| json!({"integration":item.name,"source":"Hive operator-maintained workflow","guidance":item.guidance})).collect();
    Ok(json!({
        "tools": tools,
        "ranking": ranking,
        "stack": stack,
        "semantic": state.settings.embeddings.is_some(),
        "workflows": guidance,
        "available_to_enable": setup,
        "call_with": "call_tool",
        "multi_step": "To combine several of these tools in one call, use run_workflow with steps that pass results forward ({{steps.<id>.text}}), or call a workflow_<name> tool.",
        "connection_setup": "Integrations in available_to_enable are not connected for this account yet: Hive Local ones run on the user's own computer (`hive-local enable <slug>`), bring-your-own-key ones need the user's API key in the Hive dashboard. Tell the user how to enable them instead of guessing their output. No other user's credentials are shared.",
        "empty_results": "Refine the query once; if no tool fits, continue with the user's task normally.",
    }))
}

/// Relevant integrations the caller has not connected, with how to enable them.
fn setup_hints(terms: &[String], names: &[String]) -> Vec<Value> {
    let connected: HashSet<&str> = names
        .iter()
        .filter_map(|name| {
            name.strip_prefix("mcp_")?
                .split_once("__")
                .map(|(slug, _)| slug)
        })
        .collect();
    let mut matches: Vec<(usize, &integrations::Integration)> = integrations::registry()
        .iter()
        .filter(|item| {
            (item.local.is_some() || item.byok) && !connected.contains(item.slug.as_str())
        })
        .filter_map(|item| {
            let text: HashSet<String> = tokens(&format!(
                "{} {} {} {}",
                item.slug,
                item.name,
                item.description,
                item.tools.join(" ")
            ))
            .into_iter()
            .collect();
            let hits = terms.iter().filter(|term| text.contains(*term)).count();
            (hits > 0).then_some((hits, item))
        })
        .collect();
    matches.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.slug.cmp(&b.1.slug)));
    matches
        .into_iter()
        .take(3)
        .map(|(_, item)| {
            let how = if item.local.is_some() {
                format!(
                    "Runs on the user's own computer through Hive Local: install hive-local, run `hive-local enable {}` and keep `hive-local run` running. Its tools then appear here as mcp_{}__<tool>.",
                    item.slug, item.slug
                )
            } else {
                format!(
                    "Needs the user's own {} API key: save it in the Hive dashboard (Integrations).",
                    item.name
                )
            };
            json!({
                "integration": item.name,
                "slug": item.slug,
                "description": item.description,
                "tools": item.tools.iter().take(10).map(|t| format!("mcp_{}__{t}", item.slug)).collect::<Vec<_>>(),
                "how_to_enable": how,
                "shared_status": if item.shared_owner.is_some() { "offline" } else { "not_configured" },
                "operator_action": item.shared_owner.as_ref().map(|_| "The operator-shared service is unavailable. The deployment operator must start or reconnect its isolated shared helper; Hive Local is an optional personal alternative."),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn callable_only_dependencies_survive_discovery_filtering() {
        let workflow = json!({"name":"workflow_chain","_meta":{"dev.hive/requiredTools":["scaffold_project","get_task","search_capabilities","hive_binding"]}});
        let mut tools = vec![
            json!({"name":"scaffold_project"}),
            json!({"name":"get_task"}),
            json!({"name":"search_capabilities"}),
            json!({"name":"hive_binding","_meta":{"dev.hive/requiresBinding":true}}),
            workflow.clone(),
        ];
        assert!(callable_names(&mut tools).contains("workflow_chain"));
        tools.retain(|v| v["name"] != "get_task");
        assert!(!callable_names(&mut tools).contains("workflow_chain"));
    }
    #[test]
    fn setup_hints_suggest_unconnected_local_integrations() {
        let terms = vec![stem("video"), stem("trim")];
        let hints = setup_hints(&terms, &[]);
        assert!(hints.iter().any(|h| h["slug"] == "ffmpeg"), "{hints:?}");
        let connected = vec!["mcp_ffmpeg__ffmpeg_trim".to_owned()];
        assert!(
            !setup_hints(&terms, &connected)
                .iter()
                .any(|h| h["slug"] == "ffmpeg")
        );
    }
    #[test]
    fn stemming_joins_common_inflections() {
        assert_eq!(stem("templates"), "template");
        assert_eq!(stem("searching"), "search");
        assert_eq!(stem("libraries"), "library");
        assert_eq!(stem("class"), "class");
        assert_eq!(stem("matches"), "match");
        assert_eq!(
            tokens("mcp_templr__search_templates"),
            vec!["mcp", "templr", "search", "template"]
        );
    }
    #[test]
    fn bm25_prefers_name_matches_and_normalizes() {
        let docs: Vec<Document> = [
            json!({"name":"mcp_templr__search_templates","description":"Search website templates by business type"}),
            json!({"name":"mcp_files__read_file","description":"Read a file; templates are files too"}),
            json!({"name":"mcp_files__write_file","description":"Write a file"}),
        ]
        .iter()
        .map(document)
        .collect();
        let scores = bm25(&docs, &["template".to_owned(), "search".to_owned()]);
        assert!((scores[0] - 1.0).abs() < 1e-9);
        assert!(scores[0] > scores[1]);
        assert_eq!(scores[2], 0.0);
    }
    #[test]
    fn stack_overlap_is_bounded() {
        let tool = json!({"name":"hive_x","description":"Next.js and React scaffolding"});
        assert!(stack_overlap(&tool, &["react".into(), "next.js".into()]) > 0.9);
        assert_eq!(stack_overlap(&tool, &[]), 0.0);
        assert_eq!(stack_overlap(&tool, &["django".into()]), 0.0);
    }
}
