//! Automatic workflows from repeated tool chains.
//!
//! Two sources feed the same counter:
//! - inline `run_workflow` definitions an account runs again and again;
//! - individual calls that form a chain, where an argument of one call is
//!   exactly the result (or a field of the result) of an earlier call within
//!   `chain_window_seconds`.
//!
//! A chain's signature ignores literal argument values. After the same
//! signature is seen `auto_publish_after` times, Hive publishes a private
//! workflow for that account: values that differed between runs become
//! inputs, values that never changed stay constants, and linked arguments
//! become `{{steps.<id>...}}` templates. Argument names that look like secrets
//! are never stored and always become inputs.
use super::{Definition, Step, limits, statement};
use crate::{
    features::common::{error::ApiResult, model::now},
    state::AppState,
};
use sea_orm::ConnectionTrait;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap, VecDeque},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

const MIN_LINK_CHARS: usize = 6;
const RECENT_CALLS: usize = 12;
const KEPT_TEXT: usize = 8192;
const SECRET_HINTS: &[&str] = &[
    "key",
    "token",
    "secret",
    "password",
    "authorization",
    "cookie",
    "credential",
];

fn secret(path: &str) -> bool {
    let lower = path.to_lowercase();
    SECRET_HINTS.iter().any(|hint| lower.contains(hint))
}
fn sensitive_schema(value: &Value, path: &str) -> bool {
    match value {
        Value::Object(map) => map.iter().any(|(key, value)| {
            (secret(path) && matches!(key.as_str(), "default" | "const" | "enum" | "examples"))
                || sensitive_schema(value, &format!("{path}.{key}"))
        }),
        Value::Array(items) => items.iter().any(|value| sensitive_schema(value, path)),
        _ => false,
    }
}

// --- Shape and literals ------------------------------------------------------

/// Replace literal leaves with `{{lit}}` markers; collect them by path.
fn strip(value: &Value, path: &str, literals: &mut BTreeMap<String, Value>) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| (k.clone(), strip(v, &format!("{path}.{k}"), literals)))
                .collect(),
        ),
        Value::String(s) if s.contains("{{") => value.clone(),
        // Small arrays of literals (e.g. a list of paths) are one literal.
        other => {
            literals.insert(path.to_owned(), other.clone());
            json!("{{lit}}")
        }
    }
}
/// The signature of a definition and its literal values.
fn shape(definition: &Definition) -> (String, Value, BTreeMap<String, Value>) {
    let mut literals = BTreeMap::new();
    let steps: Vec<Value> = definition
        .steps
        .iter()
        .map(|step| {
            json!({
                "id": step.id,
                "tool": step.tool,
                "arguments": strip(&step.arguments, &step.id, &mut literals),
                "when": step.when,
                "for_each": step.for_each,
                "after": step.after,
                "on_error": step.on_error,
            })
        })
        .collect();
    let template =
        json!({"inputs": definition.inputs, "steps": steps, "output": definition.output});
    let digest = Sha256::digest(template.to_string().as_bytes());
    (hex::encode(&digest[..16]), template, literals)
}
fn set_path(value: &mut Value, path: &[&str], replacement: Value) {
    let Some((first, rest)) = path.split_first() else {
        *value = replacement;
        return;
    };
    if let Some(child) = value.get_mut(*first) {
        set_path(child, rest, replacement);
    }
}
/// Build the definition to publish: varying literals become inputs.
fn published(
    template: &Value,
    literals: &Map<String, Value>,
    varying: &[String],
) -> Option<Definition> {
    let mut steps: Vec<Value> = template["steps"].as_array()?.clone();
    let mut inputs = template
        .get("inputs")
        .cloned()
        .unwrap_or_else(|| json!({"type":"object"}));
    let mut properties = inputs["properties"]
        .as_object()
        .cloned()
        .unwrap_or_default();
    let mut required = inputs["required"].as_array().cloned().unwrap_or_default();
    for (path, value) in literals {
        let mut parts = path.split('.');
        let step_id = parts.next()?;
        let rest: Vec<&str> = parts.collect();
        let step = steps.iter_mut().find(|s| s["id"] == step_id)?;
        let tool = step["tool"].as_str().unwrap_or_default().to_owned();
        let arguments = step.get_mut("arguments")?;
        if varying.contains(path) || value.is_null() {
            let mut name: String = format!("{step_id}_{}", rest.join("_"))
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() {
                        c.to_ascii_lowercase()
                    } else {
                        '_'
                    }
                })
                .take(48)
                .collect();
            while properties.contains_key(&name) {
                name.push('_');
            }
            let kind = match value {
                Value::Number(_) => "number",
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                _ => "string",
            };
            properties.insert(
                name.clone(),
                json!({"type": kind, "description": format!("`{}` for {tool}", rest.join("."))}),
            );
            required.push(json!(name));
            set_path(arguments, &rest, json!(format!("{{{{inputs.{name}}}}}")));
        } else {
            set_path(arguments, &rest, value.clone());
        }
    }
    inputs["properties"] = json!(properties);
    inputs["required"] = json!(required);
    let definition = json!({
        "inputs": inputs,
        "steps": steps.into_iter().map(|mut s| {
            if let Some(map) = s.as_object_mut() {
                map.retain(|_, v| !v.is_null() && v != &json!([]));
            }
            s
        }).collect::<Vec<_>>(),
        "output": template["output"],
    });
    let mut definition: Definition = serde_json::from_value(definition).ok()?;
    if definition.output.as_ref().is_some_and(Value::is_null) {
        definition.output = None;
    }
    Some(definition)
}

/// Count one occurrence of `definition` for `owner`; publish when it repeats enough.
/// # Errors
/// Returns database errors.
#[allow(clippy::too_many_lines)]
pub async fn record(state: &AppState, owner: &str, definition: &Definition) -> ApiResult<()> {
    let limits = limits(state);
    if limits.auto_publish_after == 0 || definition.steps.len() < 2 {
        return Ok(());
    }
    // A schema can itself embed a credential as a default or enum. Do not
    // persist such recipes in the learning store; manual publication remains
    // an explicit caller action.
    if sensitive_schema(&definition.inputs, "inputs")
        || super::plan(state, &serde_json::to_value(definition)?, true).is_err()
    {
        return Ok(());
    }
    if definition
        .steps
        .iter()
        .any(|step| !super::step_tool_allowed(&step.tool, true))
    {
        return Ok(());
    }
    let (signature, template, literals) = shape(definition);
    let stored: Map<String, Value> = literals
        .iter()
        .map(|(path, value)| {
            let kept = if secret(path) || value.to_string().len() > 4096 {
                Value::Null
            } else {
                value.clone()
            };
            (path.clone(), kept)
        })
        .collect();
    let txn = sea_orm::TransactionTrait::begin(&state.postgres).await?;
    txn.execute_raw(statement(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 722))",
        vec![owner.into()],
    ))
    .await?;
    let row = sea_orm::ConnectionTrait::query_one_raw(
        &txn,
        statement(
            "INSERT INTO workflow_patterns(owner,signature,template,literals,seen,last_seen) SELECT $1,$2,$3,$4,1,$5 WHERE EXISTS(SELECT 1 FROM workflow_patterns WHERE owner=$1 AND signature=$2) OR (SELECT count(*) FROM workflow_patterns WHERE owner=$1)<$6 ON CONFLICT(owner,signature) DO UPDATE SET seen=workflow_patterns.seen+1, last_seen=EXCLUDED.last_seen RETURNING seen, literals AS previous, varying, workflow_id",
            vec![owner.into(), signature.clone().into(), template.clone().into(), Value::Object(stored.clone()).into(), now().into(), i64::try_from(limits.workflows_per_owner.saturating_mul(10)).unwrap_or(i64::MAX).into()],
        ),
    )
    .await?;
    let Some(row) = row else {
        txn.commit().await?;
        return Ok(());
    };
    let seen: i32 = row.try_get("", "seen")?;
    let previous: Value = row.try_get("", "previous")?;
    let mut varying: Vec<String> =
        serde_json::from_value(row.try_get::<Value>("", "varying")?).unwrap_or_default();
    let published_already: Option<uuid::Uuid> = row.try_get("", "workflow_id")?;
    for (path, value) in &stored {
        let changed = previous.get(path) != Some(value) || value.is_null();
        if seen > 1 && changed && !varying.contains(path) {
            varying.push(path.clone());
        }
    }
    sea_orm::ConnectionTrait::execute_raw(
        &txn,
        statement(
            "UPDATE workflow_patterns SET varying=$3, literals=$4 WHERE owner=$1 AND signature=$2",
            vec![
                owner.into(),
                signature.clone().into(),
                json!(varying).into(),
                Value::Object(stored.clone()).into(),
            ],
        ),
    )
    .await?;
    txn.commit().await?;
    let threshold = i32::try_from(limits.auto_publish_after).unwrap_or(i32::MAX);
    if seen < threshold {
        return Ok(());
    }
    let Some(definition) = published(&template, &stored, &varying) else {
        return Ok(());
    };
    let value = serde_json::to_value(&definition)?;
    if super::plan(state, &value, true).is_err() {
        return Ok(());
    }
    if let Some(id) = published_already {
        state.postgres.execute_raw(statement(
            "UPDATE hive_workflows SET definition=$3,updated_at=$4 WHERE id=$1 AND owner=$2 AND active AND origin='auto' AND definition IS DISTINCT FROM $3",
            vec![id.into(), owner.into(), value.into(), now().into()],
        )).await?;
        return Ok(());
    }
    let chain = super::engine::definition_text(&definition);
    let name: String = format!("Auto: {chain}").chars().take(80).collect();
    let description = format!(
        "Published automatically for you after you ran this chain {seen} times: {chain}. Values that changed between runs are inputs; everything else is fixed. Delete it with delete_workflow if it is not useful."
    );
    let stored_workflow = super::insert(
        state,
        super::NewWorkflow {
            owner,
            name: &name,
            description: &description,
            definition: &value,
            visibility: "private",
            origin: "auto",
            signature: Some(&signature),
        },
    )
    .await;
    if let Ok(workflow) = stored_workflow {
        state
            .postgres
            .execute_raw(statement(
                "UPDATE workflow_patterns SET workflow_id=$3 WHERE owner=$1 AND signature=$2",
                vec![owner.into(), signature.into(), workflow.id.into()],
            ))
            .await?;
        tracing::info!(owner, workflow = %workflow.id, "Published an automatic workflow");
    }
    Ok(())
}

// --- Chains of individual calls ----------------------------------------------

struct Observed {
    seq: u64,
    at: Instant,
    tool: String,
    args: Value,
    text: String,
    json: Value,
    /// Argument path → (earlier call's seq, template suffix after `steps.<id>.`).
    links: Vec<(String, u64, String)>,
}
#[derive(Default)]
struct Recent {
    seq: u64,
    owners: HashMap<String, VecDeque<Observed>>,
}
fn recent() -> &'static Mutex<Recent> {
    static RECENT: OnceLock<Mutex<Recent>> = OnceLock::new();
    RECENT.get_or_init(Mutex::default)
}
/// Find `needle` inside `value` (depth-limited), returning its dotted path.
fn find_path(value: &Value, needle: &Value, path: &mut Vec<String>, budget: &mut usize) -> bool {
    if *budget == 0 {
        return false;
    }
    *budget -= 1;
    if value == needle {
        return true;
    }
    if path.len() >= 5 {
        return false;
    }
    match value {
        Value::Object(map) => map.iter().any(|(k, v)| {
            path.push(k.clone());
            let found = find_path(v, needle, path, budget);
            if !found {
                path.pop();
            }
            found
        }),
        Value::Array(items) => items.iter().enumerate().take(50).any(|(i, v)| {
            path.push(i.to_string());
            let found = find_path(v, needle, path, budget);
            if !found {
                path.pop();
            }
            found
        }),
        _ => false,
    }
}
fn leaves<'v>(value: &'v Value, path: String, out: &mut Vec<(String, &'v Value)>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                leaves(
                    v,
                    if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    },
                    out,
                );
            }
        }
        other => out.push((path, other)),
    }
}
/// Observe one top-level tool call. Builds a chain when this call consumed an
/// earlier call's result and records it as a pattern.
#[allow(clippy::too_many_lines)]
pub async fn observe(state: &AppState, owner: &str, tool: &str, args: &Value, result: &Value) {
    let limits = limits(state);
    if limits.auto_publish_after == 0
        || tool.starts_with(super::PREFIX)
        || !super::step_tool_allowed(tool, true)
        || !args.is_object()
    {
        return;
    }
    let window = Duration::from_secs(limits.chain_window_seconds);
    let view = super::engine::view(result, KEPT_TEXT);
    let text = view["text"].as_str().unwrap_or_default().trim().to_owned();
    let definition = {
        let Ok(mut recent) = recent().lock() else {
            return;
        };
        if recent.owners.len() >= limits.tracked_owners && !recent.owners.contains_key(owner) {
            // Forget the least recently active account to stay bounded.
            if let Some(oldest) = recent
                .owners
                .iter()
                .min_by_key(|(_, calls)| calls.back().map(|c| c.at))
                .map(|(owner, _)| owner.clone())
            {
                recent.owners.remove(&oldest);
            }
        }
        recent.seq += 1;
        let seq = recent.seq;
        let calls = recent.owners.entry(owner.to_owned()).or_default();
        calls.retain(|call| call.at.elapsed() < window);
        let mut links = Vec::new();
        let mut arg_leaves = Vec::new();
        leaves(args, String::new(), &mut arg_leaves);
        for (path, value) in arg_leaves {
            let candidate = match value {
                Value::String(s) if s.trim().chars().count() >= MIN_LINK_CHARS => value,
                _ => continue,
            };
            for earlier in calls.iter().rev() {
                if earlier.text == candidate.as_str().unwrap_or_default().trim() {
                    links.push((path.clone(), earlier.seq, "text".to_owned()));
                    break;
                }
                let mut found = Vec::new();
                let mut budget = 2000;
                if !earlier.json.is_null()
                    && find_path(&earlier.json, candidate, &mut found, &mut budget)
                {
                    links.push((
                        path.clone(),
                        earlier.seq,
                        format!("json.{}", found.join(".")),
                    ));
                    break;
                }
            }
        }
        let chain = if links.is_empty() {
            None
        } else {
            // The chain is every earlier call this one depends on, transitively.
            let mut needed: Vec<u64> = links.iter().map(|(_, s, _)| *s).collect();
            let mut members = Vec::new();
            while let Some(seq) = needed.pop() {
                if members.contains(&seq) {
                    continue;
                }
                if let Some(call) = calls.iter().find(|c| c.seq == seq) {
                    members.push(seq);
                    needed.extend(call.links.iter().map(|(_, s, _)| *s));
                }
            }
            members.sort_unstable();
            let ids: HashMap<u64, String> = members
                .iter()
                .enumerate()
                .map(|(i, seq)| (*seq, format!("s{}", i + 1)))
                .collect();
            let mut steps = Vec::new();
            let mut build =
                |tool: &str, args: &Value, links: &[(String, u64, String)], id: String| {
                    let mut arguments = args.clone();
                    for (path, seq, suffix) in links {
                        if let Some(source) = ids.get(seq) {
                            let parts: Vec<&str> = path.split('.').collect();
                            set_path(
                                &mut arguments,
                                &parts,
                                json!(format!("{{{{steps.{source}.{suffix}}}}}")),
                            );
                        }
                    }
                    steps.push(Step {
                        id,
                        tool: tool.to_owned(),
                        arguments,
                        when: None,
                        for_each: None,
                        after: Vec::new(),
                        on_error: super::OnError::Stop,
                        description: None,
                    });
                };
            for seq in &members {
                if let Some(call) = calls.iter().find(|c| c.seq == *seq) {
                    build(&call.tool, &call.args, &call.links, ids[seq].clone());
                }
            }
            build(tool, args, &links, format!("s{}", members.len() + 1));
            Some(Definition {
                inputs: json!({"type":"object","properties":{}}),
                steps,
                output: None,
            })
        };
        let kept_args = if args.to_string().len() <= KEPT_TEXT {
            args.clone()
        } else {
            json!({})
        };
        calls.push_back(Observed {
            seq,
            at: Instant::now(),
            tool: tool.to_owned(),
            args: kept_args,
            text,
            json: view["json"].clone(),
            links,
        });
        while calls.len() > RECENT_CALLS {
            calls.pop_front();
        }
        chain
    };
    if let Some(definition) = definition
        && let Err(error) = record(state, owner, &definition).await
    {
        tracing::warn!(error = error.code, "Tool chain was not recorded");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schemas_with_secret_defaults_are_not_learned() {
        assert!(sensitive_schema(
            &json!({"properties":{"api_key":{"type":"string","default":"secret-fixture"}}}),
            "inputs"
        ));
        assert!(!sensitive_schema(
            &json!({"properties":{"query":{"type":"string","default":"fixture"}}}),
            "inputs"
        ));
    }
    fn definition(url: &str, path: &str) -> Definition {
        let value = json!({
            "steps": [
                {"id":"s1","tool":"mcp_markitdown__convert_to_markdown","arguments":{"uri":url}},
                {"id":"s2","tool":"mcp_filesystem__write_file","arguments":{"path":path,"content":"{{steps.s1.text}}","api_key":"sk-123"}}
            ]
        });
        let parsed: Result<Definition, _> = serde_json::from_value(value);
        let Ok(parsed) = parsed else {
            panic!("fixture definition is invalid")
        };
        parsed
    }
    #[test]
    fn signatures_ignore_literals_and_publish_lifts_varying_ones() {
        let (a, template, first) = shape(&definition("https://a.example", "/tmp/a.md"));
        let (b, _, second) = shape(&definition("https://b.example", "/tmp/a.md"));
        assert_eq!(a, b, "literal values do not change the chain signature");
        assert_ne!(first.get("s1.uri"), second.get("s1.uri"));
        let mut stored: Map<String, Value> = second.into_iter().collect();
        stored.insert("s2.api_key".into(), Value::Null);
        let published = published(&template, &stored, &["s1.uri".to_owned()]);
        let Some(published) = published else {
            panic!("publishable chain rejected")
        };
        let props = &published.inputs["properties"];
        assert!(props.get("s1_uri").is_some(), "{props}");
        assert!(
            props.get("s2_api_key").is_some(),
            "secrets always become inputs"
        );
        assert!(
            props.get("s2_path").is_none(),
            "unchanged values stay constant"
        );
        assert_eq!(published.steps[1].arguments["path"], "/tmp/a.md");
        assert_eq!(published.steps[1].arguments["content"], "{{steps.s1.text}}");
        assert_eq!(published.steps[0].arguments["uri"], "{{inputs.s1_uri}}");
    }
    #[test]
    fn find_path_locates_nested_values() {
        let value = json!({"results":[{"id":"abc123"},{"id":"zzz999"}]});
        let mut path = Vec::new();
        let mut budget = 100;
        assert!(find_path(&value, &json!("zzz999"), &mut path, &mut budget));
        assert_eq!(path, vec!["results", "1", "id"]);
    }
}
