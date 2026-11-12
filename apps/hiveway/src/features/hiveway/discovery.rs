//! Bounded on-demand discovery across private connections and public user artifacts.
use crate::{
    features::common::{
        error::{ApiError, ApiResult},
        model::Credential,
    },
    state::AppState,
};
use serde_json::{Value, json};
pub(super) fn terms(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| {
            s.len() > 1
                && ![
                    "find", "tool", "tools", "for", "the", "please", "need", "help", "with", "can",
                    "you", "and", "some",
                ]
                .contains(s)
        })
        .take(12)
        .map(str::to_owned)
        .collect()
}
fn score(tool: &Value, terms: &[String]) -> usize {
    let name = tool["name"].as_str().unwrap_or_default();
    let description = tool["description"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase();
    let mut score = terms
        .iter()
        .map(|term| usize::from(name.contains(term)) * 4 + usize::from(description.contains(term)))
        .sum();
    for provider in crate::features::integrations::registry() {
        if name.starts_with(&format!("mcp_{}__", provider.slug)) {
            let context = format!("{} {}", provider.description, provider.guidance).to_lowercase();
            score += terms
                .iter()
                .filter(|term| context.contains(term.as_str()))
                .count()
                * 2;
        }
    }
    if score > 0
        && (name.contains("search_templates")
            || name.contains("resolve-library-id")
            || name.ends_with("__search"))
    {
        score += 3;
    }
    score
}
pub(super) async fn search(
    state: &AppState,
    credential: &Credential,
    args: &Value,
) -> ApiResult<Value> {
    let query = args["query"]
        .as_str()
        .filter(|q| !q.trim().is_empty() && q.len() <= 200)
        .ok_or_else(|| ApiError::bad("invalid_tool_query"))?;
    let limit = args["limit"].as_u64().unwrap_or(6);
    if !(1..=8).contains(&limit) {
        return Err(ApiError::bad("invalid_tool_limit"));
    }
    let terms = terms(query);
    if terms.is_empty() {
        return Err(ApiError::bad("tool_query_too_general"));
    }
    let mut tools = super::tools(state, credential, Some(query), true).await?["tools"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    tools.retain(|tool| {
        tool["name"].as_str().is_some_and(super::discoverable) && score(tool, &terms) > 0
    });
    tools.sort_by(|a, b| {
        score(b, &terms)
            .cmp(&score(a, &terms))
            .then_with(|| a["name"].as_str().cmp(&b["name"].as_str()))
    });
    tools.truncate(usize::try_from(limit).unwrap_or(6));
    let mut bytes = 0;
    tools.retain(|tool| {
        bytes += tool.to_string().len();
        bytes <= 32_768
    });
    let guidance: Vec<_> = crate::features::integrations::registry().iter().filter(|item| {
        tools.iter().any(|tool| tool["name"].as_str().is_some_and(|name|name.starts_with(&format!("mcp_{}__",item.slug)) || name == format!("hive_{}",item.id.simple())))
    }).map(|item| json!({"integration":item.name,"source":"Hive operator-maintained workflow","guidance":item.guidance})).collect();
    Ok(
        json!({"tools":tools,"workflows":guidance,"call_with":"call_tool","connection_setup":"If the integration has no connected tools, start Hive Local with your own provider keys. Catalog artifacts bind to your own connection_id; no other user's credentials are shared.","empty_results":"Refine the query once; if no tool fits, continue with the user's task normally."}),
    )
}
#[cfg(test)]
mod tests {
    #[test]
    fn intent_search_prefers_real_template_search() {
        let terms = super::terms("please find clothing landing page inspiration");
        let tool = serde_json::json!({"name":"mcp_templr__search_templates","description":"Search website templates"});
        assert!(super::score(&tool, &terms) > 0);
        assert!(!terms.iter().any(|term| term == "please"));
    }
}
