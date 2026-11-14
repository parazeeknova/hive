//! Step argument templates.
//!
//! A string that is exactly one `{{ expression }}` becomes the referenced JSON
//! value with its type; any other string interpolates each expression as text.
//! Expressions are a path with optional filters:
//!
//! ```text
//! {{ inputs.url }}
//! {{ steps.fetch.text | truncate: 4000 }}
//! {{ steps.query.json.rows | pluck: 0 | join: ", " }}
//! {{ item.path }}  {{ index }}          (inside for_each steps)
//! ```
//!
//! Roots: `inputs`, `steps.<id>` (with `text`, `json`, `ok`, `result`, `items`),
//! `item` and `index`. A missing path is `null`; `default:` supplies a fallback.
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

/// Values an expression can read.
pub struct Scope<'a> {
    pub inputs: &'a Value,
    pub steps: &'a Map<String, Value>,
    pub item: Option<&'a Value>,
    pub index: Option<usize>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TemplateError(pub String);
impl std::fmt::Display for TemplateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
fn fail(message: impl Into<String>) -> TemplateError {
    TemplateError(message.into())
}

const MAX_EXPRESSION: usize = 512;
const FILTERS: &[&str] = &[
    "default",
    "truncate",
    "truncate_bytes",
    "json",
    "sql",
    "first",
    "last",
    "length",
    "keys",
    "join",
    "lines",
    "lower",
    "upper",
    "trim",
    "pluck",
    "number",
    "string",
    "split",
];

/// One `{{ ... }}` occurrence.
struct Expression {
    root: String,
    path: Vec<String>,
    filters: Vec<(String, Option<String>)>,
}
/// Split `text` into literal and expression parts.
fn parts(text: &str) -> Result<Vec<Result<&str, &str>>, TemplateError> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        if start > 0 {
            out.push(Ok(&rest[..start]));
        }
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .ok_or_else(|| fail("unclosed {{ in template"))?;
        out.push(Err(after[..end].trim()));
        rest = &after[end + 2..];
    }
    if !rest.is_empty() {
        out.push(Ok(rest));
    }
    Ok(out)
}
/// Split on `|` outside quotes.
fn split_pipes(text: &str) -> Vec<String> {
    let mut out = vec![String::new()];
    let mut quote: Option<char> = None;
    for ch in text.chars() {
        match (quote, ch) {
            (None, '"' | '\'') => {
                quote = Some(ch);
                if let Some(last) = out.last_mut() {
                    last.push(ch);
                }
            }
            (Some(q), c) if c == q => {
                quote = None;
                if let Some(last) = out.last_mut() {
                    last.push(ch);
                }
            }
            (None, '|') => out.push(String::new()),
            _ => {
                if let Some(last) = out.last_mut() {
                    last.push(ch);
                }
            }
        }
    }
    out.into_iter().map(|s| s.trim().to_owned()).collect()
}
fn parse(text: &str) -> Result<Expression, TemplateError> {
    if text.is_empty() || text.len() > MAX_EXPRESSION {
        return Err(fail("empty or oversized expression"));
    }
    let mut pieces = split_pipes(text).into_iter();
    let path = pieces.next().unwrap_or_default();
    let mut segments = path.split('.').map(str::trim);
    let root = segments.next().unwrap_or_default().to_owned();
    if !matches!(root.as_str(), "inputs" | "steps" | "item" | "index") {
        return Err(fail(format!(
            "unknown template root `{root}`; use inputs, steps, item or index"
        )));
    }
    let path: Vec<String> = segments.map(str::to_owned).collect();
    if path.iter().any(|s| {
        s.is_empty()
            || !s
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    }) {
        return Err(fail(format!("invalid path in `{text}`")));
    }
    if root == "steps" && path.is_empty() {
        return Err(fail("steps needs a step id, e.g. steps.fetch.text"));
    }
    let mut filters = Vec::new();
    for piece in pieces {
        let (name, arg) = match piece.split_once(':') {
            Some((name, arg)) => (name.trim().to_owned(), Some(arg.trim().to_owned())),
            None => (piece, None),
        };
        if !FILTERS.contains(&name.as_str()) {
            return Err(fail(format!("unknown filter `{name}`")));
        }
        filters.push((name, arg));
    }
    Ok(Expression {
        root,
        path,
        filters,
    })
}
/// Every expression inside a JSON value, for validation and dependency analysis.
fn walk_strings<'v>(value: &'v Value, out: &mut Vec<&'v str>) {
    match value {
        Value::String(s) => out.push(s),
        Value::Array(items) => items.iter().for_each(|v| walk_strings(v, out)),
        Value::Object(map) => map.values().for_each(|v| walk_strings(v, out)),
        _ => {}
    }
}
/// What a template reads: the step ids it references and whether it uses `item`.
pub struct References {
    pub steps: BTreeSet<String>,
    pub uses_item: bool,
    pub inputs: BTreeSet<String>,
}
/// Validate every expression in `value` and collect its references.
/// # Errors
/// Rejects malformed expressions, unknown roots and filters.
pub fn references(value: &Value) -> Result<References, TemplateError> {
    let mut strings = Vec::new();
    walk_strings(value, &mut strings);
    let mut refs = References {
        steps: BTreeSet::new(),
        uses_item: false,
        inputs: BTreeSet::new(),
    };
    for text in strings {
        for part in parts(text)? {
            let Err(expression) = part else { continue };
            let parsed = parse(expression)?;
            match parsed.root.as_str() {
                "steps" => {
                    refs.steps.insert(parsed.path[0].clone());
                }
                "item" | "index" => refs.uses_item = true,
                _ => {
                    if let Some(first) = parsed.path.first() {
                        refs.inputs.insert(first.clone());
                    }
                }
            }
        }
    }
    Ok(refs)
}
fn lookup<'v>(value: &'v Value, path: &[String]) -> Option<&'v Value> {
    let mut current = value;
    for segment in path {
        current = match current {
            Value::Object(map) => map.get(segment)?,
            Value::Array(items) => items.get(segment.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(current)
}
fn resolve(expression: &Expression, scope: &Scope<'_>) -> Value {
    let found = match expression.root.as_str() {
        "inputs" => lookup(scope.inputs, &expression.path).cloned(),
        "steps" => scope
            .steps
            .get(&expression.path[0])
            .and_then(|step| lookup(step, &expression.path[1..]))
            .cloned(),
        "item" => scope
            .item
            .and_then(|item| lookup(item, &expression.path))
            .cloned(),
        _ => scope.index.map(|i| json!(i)),
    };
    found.unwrap_or(Value::Null)
}
/// Text form of a value: strings as-is, everything else as compact JSON.
#[must_use]
pub fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
fn literal(arg: &str) -> Value {
    let trimmed = arg.trim();
    if let Some(inner) = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| {
            trimmed
                .strip_prefix('\'')
                .and_then(|s| s.strip_suffix('\''))
        })
    {
        return Value::String(inner.to_owned());
    }
    serde_json::from_str(trimmed).unwrap_or_else(|_| Value::String(trimmed.to_owned()))
}
fn number_arg(arg: Option<&String>) -> Result<usize, TemplateError> {
    arg.and_then(|a| a.trim().parse().ok())
        .ok_or_else(|| fail("filter needs a whole-number argument"))
}
fn truncate_bytes(mut s: String, limit: usize) -> String {
    if s.len() > limit {
        let marker = if limit >= "…".len() { "…" } else { "" };
        let mut end = limit - marker.len();
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
        s.push_str(marker);
    }
    s
}
fn apply(value: Value, name: &str, arg: Option<&String>) -> Result<Value, TemplateError> {
    Ok(match name {
        "default" => {
            let empty = value.is_null() || value.as_str().is_some_and(str::is_empty);
            if empty {
                arg.map_or(Value::Null, |a| literal(a))
            } else {
                value
            }
        }
        "truncate" => {
            let limit = number_arg(arg)?;
            let s = text(&value);
            if s.chars().count() > limit {
                Value::String(s.chars().take(limit).collect::<String>() + "…")
            } else {
                Value::String(s)
            }
        }
        "truncate_bytes" => Value::String(truncate_bytes(text(&value), number_arg(arg)?)),
        "json" => Value::String(value.to_string()),
        "sql" => Value::String(text(&value).replace('\'', "''")),
        "first" => match value {
            Value::Array(items) => items.into_iter().next().unwrap_or(Value::Null),
            other => other,
        },
        "last" => match value {
            Value::Array(items) => items.into_iter().last().unwrap_or(Value::Null),
            other => other,
        },
        "length" => json!(match &value {
            Value::Array(items) => items.len(),
            Value::Object(map) => map.len(),
            Value::String(s) => s.chars().count(),
            Value::Null => 0,
            _ => 1,
        }),
        "keys" => match value {
            Value::Object(map) => Value::Array(map.keys().cloned().map(Value::String).collect()),
            _ => json!([]),
        },
        "join" => {
            let separator = arg.map_or_else(|| ", ".to_owned(), |a| text(&literal(a)));
            match value {
                Value::Array(items) => {
                    Value::String(items.iter().map(text).collect::<Vec<_>>().join(&separator))
                }
                other => Value::String(text(&other)),
            }
        }
        "lines" => {
            let limit = number_arg(arg)?;
            Value::String(
                text(&value)
                    .lines()
                    .take(limit)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        }
        "lower" => Value::String(text(&value).to_lowercase()),
        "upper" => Value::String(text(&value).to_uppercase()),
        "trim" => Value::String(text(&value).trim().to_owned()),
        "string" => Value::String(text(&value)),
        "split" => {
            let separator = arg.map_or_else(|| "\n".to_owned(), |a| text(&literal(a)));
            Value::Array(
                text(&value)
                    .split(separator.as_str())
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| Value::String(s.to_owned()))
                    .collect(),
            )
        }
        "number" => {
            let s = text(&value);
            s.trim()
                .parse::<i64>()
                .map(|n| json!(n))
                .or_else(|_| s.trim().parse::<f64>().map(|n| json!(n)))
                .unwrap_or(Value::Null)
        }
        "pluck" => {
            let key = arg.ok_or_else(|| fail("pluck needs a field name or index"))?;
            let path = vec![text(&literal(key))];
            match value {
                Value::Array(items) => Value::Array(
                    items
                        .iter()
                        .map(|item| lookup(item, &path).cloned().unwrap_or(Value::Null))
                        .collect(),
                ),
                other => lookup(&other, &path).cloned().unwrap_or(Value::Null),
            }
        }
        _ => return Err(fail(format!("unknown filter `{name}`"))),
    })
}
fn evaluate(text: &str, scope: &Scope<'_>) -> Result<Value, TemplateError> {
    let expression = parse(text)?;
    let mut value = resolve(&expression, scope);
    for (name, arg) in &expression.filters {
        value = apply(value, name, arg.as_ref())?;
    }
    Ok(value)
}
fn render_string(text: &str, scope: &Scope<'_>) -> Result<Value, TemplateError> {
    let parts = parts(text)?;
    if let [Err(expression)] = parts.as_slice() {
        return evaluate(expression, scope);
    }
    let mut out = String::new();
    for part in parts {
        match part {
            Ok(literal) => out.push_str(literal),
            Err(expression) => out.push_str(&self::text(&evaluate(expression, scope)?)),
        }
    }
    Ok(Value::String(out))
}
/// Render every string in `value` against `scope`.
/// # Errors
/// Rejects malformed expressions and filter misuse.
pub fn render(value: &Value, scope: &Scope<'_>) -> Result<Value, TemplateError> {
    Ok(match value {
        Value::String(s) if s.contains("{{") => render_string(s, scope)?,
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|v| render(v, scope))
                .collect::<Result<_, _>>()?,
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(k, v)| Ok((k.clone(), render(v, scope)?)))
                .collect::<Result<_, TemplateError>>()?,
        ),
        other => other.clone(),
    })
}
/// Whether a rendered `when` value allows the step to run.
#[must_use]
pub fn truthy(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|n| n != 0.0),
        Value::String(s) => !s.is_empty() && s != "false" && s != "0",
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope_with<'a>(inputs: &'a Value, steps: &'a Map<String, Value>) -> Scope<'a> {
        Scope {
            inputs,
            steps,
            item: None,
            index: None,
        }
    }
    #[test]
    fn whole_expressions_keep_types_and_strings_interpolate() -> Result<(), TemplateError> {
        let inputs = json!({"url":"https://example.com/a.csv","limit":5});
        let mut steps = Map::new();
        steps.insert(
            "q".into(),
            json!({"text":"hello","json":{"rows":[[1,"a"],[2,"b"]],"columns":["n","s"]}}),
        );
        let scope = scope_with(&inputs, &steps);
        assert_eq!(render(&json!("{{ inputs.limit }}"), &scope)?, json!(5));
        assert_eq!(
            render(
                &json!("SELECT * FROM read_csv_auto('{{inputs.url|sql}}') LIMIT {{inputs.limit}}"),
                &scope
            )?,
            json!("SELECT * FROM read_csv_auto('https://example.com/a.csv') LIMIT 5")
        );
        assert_eq!(
            render(
                &json!("{{ steps.q.json.rows | pluck: 1 | join: '/' }}"),
                &scope
            )?,
            json!("a/b")
        );
        assert_eq!(
            render(
                &json!({"n":"{{steps.q.json.rows|length}}","missing":"{{steps.none.text|default:\"x\"}}"}),
                &scope
            )?,
            json!({"n":2,"missing":"x"})
        );
        assert_eq!(
            render(&json!("{{steps.q.text|truncate:2|upper}}"), &scope)?,
            json!("HE…")
        );
        Ok(())
    }
    #[test]
    fn sql_filter_escapes_quotes() -> Result<(), TemplateError> {
        let inputs = json!({"v":"it's'; DROP TABLE x; --"});
        let steps = Map::new();
        let scope = scope_with(&inputs, &steps);
        assert_eq!(
            render(&json!("'{{inputs.v|sql}}'"), &scope)?,
            json!("'it''s''; DROP TABLE x; --'")
        );
        Ok(())
    }
    #[test]
    fn byte_truncation_and_research_note_respect_utf8_limits() -> Result<(), TemplateError> {
        for limit in 0..12 {
            let result = apply(
                json!("界🦀abcdef"),
                "truncate_bytes",
                Some(&limit.to_string()),
            )?;
            assert!(result.as_str().is_some_and(|s| s.len() <= limit));
        }
        assert_eq!(
            apply(json!("fits"), "truncate_bytes", Some(&"4".into()))?,
            "fits"
        );
        let recipe: Value = serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/workflows/implementation-research-brief.json"
        )))
        .map_err(|_| fail("invalid research example"))?;
        let save = recipe["steps"]
            .as_array()
            .and_then(|steps| steps.iter().find(|step| step["id"] == "save"))
            .ok_or_else(|| fail("missing save step"))?;
        let inputs = json!({"task":"🦀".repeat(1000),"library_name":"界".repeat(600),"repo":"local/byte-budget"});
        let steps = Map::from_iter([
            ("docs".into(), json!({"text":"界".repeat(3000)})),
            ("fetch".into(), json!({"text":"🦀".repeat(2000)})),
        ]);
        let args = render(&save["arguments"], &scope_with(&inputs, &steps))?;
        assert!(args["body"].as_str().is_some_and(|s| s.len() <= 8192));
        assert!(args["title"].as_str().is_some_and(|s| s.len() <= 160));
        let inputs = json!({"task":"Question","library_name":"Library","repo":"local/byte-budget"});
        let steps = Map::from_iter([
            ("docs".into(), json!({"text":"Docs"})),
            ("fetch".into(), json!({"text":"Web"})),
        ]);
        let args = render(&save["arguments"], &scope_with(&inputs, &steps))?;
        assert_eq!(
            args["body"],
            "Question: Question\n\nDocumentation:\nDocs\n\nWeb evidence:\nWeb"
        );
        Ok(())
    }
    #[test]
    fn references_and_errors() {
        let refs =
            references(&json!({"a":"{{steps.fetch.text}} {{inputs.url}}","b":["{{item.path}}"]}));
        let Ok(refs) = refs else {
            panic!("valid template rejected")
        };
        assert!(refs.steps.contains("fetch"));
        assert!(refs.inputs.contains("url"));
        assert!(refs.uses_item);
        assert!(references(&json!("{{ env.HOME }}")).is_err());
        assert!(references(&json!("{{ steps }}")).is_err());
        assert!(references(&json!("{{ inputs.a | exec }}")).is_err());
        assert!(references(&json!("{{ inputs.a")).is_err());
        assert!(truthy(&json!("x")) && !truthy(&json!("")) && !truthy(&json!([])));
    }
}
