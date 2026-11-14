//! Bounded syntax-tree retrieval. Edges describe source syntax, not compiler resolution.
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    ops::ControlFlow,
    time::{Duration, Instant},
};
use tree_sitter::{Language, Node, ParseOptions, Parser, Tree};

fn grammar(language: &str) -> Option<Language> {
    Some(match language {
        "rust" => tree_sitter_rust::LANGUAGE.into(),
        "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "tsx" => tree_sitter_typescript::LANGUAGE_TSX.into(),
        "javascript" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
        "python" => tree_sitter_python::LANGUAGE.into(),
        _ => return None,
    })
}

fn parse(language: &str, source: &str) -> Result<Tree, &'static str> {
    let grammar = grammar(language).ok_or("code_context_language_unsupported")?;
    if source.len() > 1_048_576 {
        return Err("code_context_source_too_large");
    }
    let mut parser = Parser::new();
    parser
        .set_language(&grammar)
        .map_err(|_| "code_context_parser_unavailable")?;
    let start = Instant::now();
    let mut progress = |_: &tree_sitter::ParseState| {
        if start.elapsed() > Duration::from_millis(100) {
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    };
    parser
        .parse_with_options(
            &mut |offset, _| source.as_bytes().get(offset..).unwrap_or_default(),
            None,
            Some(ParseOptions::new().progress_callback(&mut progress)),
        )
        .ok_or("code_context_parse_budget_exceeded")
}

fn visit<'a>(
    root: Node<'a>,
    mut visitor: impl FnMut(Node<'a>) -> bool,
) -> Result<(), &'static str> {
    let mut cursor = root.walk();
    let start = Instant::now();
    for count in 0..200_000 {
        if count % 256 == 0 && start.elapsed() > Duration::from_millis(100) {
            return Err("code_context_traversal_budget_exceeded");
        }
        if cursor.node().is_named() && !visitor(cursor.node()) {
            return Ok(());
        }
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return Ok(());
            }
        }
    }
    Err("code_context_traversal_budget_exceeded")
}

fn text<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    source.get(node.byte_range()).unwrap_or_default()
}

fn has_ancestor(node: Node<'_>, kinds: &[&str]) -> bool {
    let mut parent = node.parent();
    // Bound nesting even in malformed or adversarial source.
    for _ in 0..128 {
        let Some(node) = parent else {
            break;
        };
        if kinds.contains(&node.kind()) {
            return true;
        }
        parent = node.parent();
    }
    false
}

fn definition(node: Node<'_>, source: &str) -> Option<(String, &'static str)> {
    let kind = match node.kind() {
        "function_item"
        | "function_signature_item"
        | "function_declaration"
        | "generator_function_declaration"
        | "function_definition" => {
            if has_ancestor(
                node,
                &["impl_item", "trait_item", "class_body", "class_definition"],
            ) {
                "method"
            } else {
                "function"
            }
        }
        "method_definition" | "method_signature" => "method",
        "struct_item" => "struct",
        "enum_item" | "enum_declaration" => "enum",
        "trait_item" => "trait",
        "type_item" | "type_alias_declaration" => "type",
        "interface_declaration" => "interface",
        "class_declaration" | "class_definition" => "class",
        "const_item" | "static_item" => "constant",
        "macro_definition" => "macro",
        "variable_declarator" => {
            if has_ancestor(
                node,
                &[
                    "function_declaration",
                    "arrow_function",
                    "function_expression",
                    "method_definition",
                ],
            ) {
                return None;
            }
            match node.child_by_field_name("value").map(|n| n.kind()) {
                Some("arrow_function" | "function_expression" | "generator_function") => "function",
                _ => "constant",
            }
        }
        _ => return None,
    };
    let name = node.child_by_field_name("name")?;
    let name = text(name, source);
    if name.is_empty() || name.len() > 200 {
        return None;
    }
    Some((name.to_owned(), kind))
}

/// Parser-backed definitions for supported valid source; malformed/other languages
/// keep the existing explicitly heuristic indexer.
pub(super) fn definitions(
    language: &str,
    source: &str,
    max: usize,
) -> Option<Vec<super::symbols::Symbol>> {
    let tree = parse(language, source).ok()?;
    if tree.root_node().has_error() {
        return None;
    }
    let mut result = Vec::new();
    visit(tree.root_node(), |node| {
        if result.len() >= max {
            return false;
        }
        if let Some((name, kind)) = definition(node, source) {
            result.push(super::symbols::Symbol {
                name,
                kind,
                line: i32::try_from(node.start_position().row + 1).unwrap_or(i32::MAX),
                signature: text(node, source)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .chars()
                    .take(200)
                    .collect(),
            });
        }
        true
    })
    .ok()?;
    Some(result)
}

fn location(node: Node<'_>, name: &str, kind: &str) -> Value {
    json!({"name":name,"kind":kind,"start_line":node.start_position().row+1,"end_line":node.end_position().row+1})
}

/// Retrieve a whole definition and its syntax-level dependencies without returning
/// the entire indexed file. `line` disambiguates duplicate method/function names.
pub(super) fn context(
    language: &str,
    source: &str,
    name: &str,
    line: Option<usize>,
    max_lines: usize,
) -> Result<Value, &'static str> {
    let tree = parse(language, source)?;
    let root = tree.root_node();
    let mut matches = Vec::new();
    let mut imports = Vec::new();
    visit(root, |node| {
        if let Some((found, kind)) = definition(node, source)
            && found == name
            && line.is_none_or(|line| node.start_position().row + 1 == line)
            && matches.len() < 20
        {
            matches.push((node, kind));
        }
        if matches!(
            node.kind(),
            "use_declaration" | "import_statement" | "import_from_statement"
        ) && imports.len() < 20
        {
            imports.push(json!({"line":node.start_position().row+1,"text":text(node, source).chars().take(300).collect::<String>()}));
        }
        true
    })?;
    if matches.is_empty() {
        return Err("code_context_symbol_not_found");
    }
    if matches.len() > 1 {
        return Ok(
            json!({"analysis":"syntax","ambiguous":true,"matches":matches.iter().map(|(n,k)|location(*n,name,k)).collect::<Vec<_>>(),"notice":"Pass start_line to select a definition; locations are in this file only."}),
        );
    }
    let (selected, kind) = matches[0];
    let mut edges = Vec::new();
    let mut seen = HashSet::new();
    let mut truncated = false;
    visit(selected, |node| {
        let (edge, target) = match node.kind() {
            "call_expression" | "call" => ("calls", node.child_by_field_name("function")),
            "new_expression" => ("constructs", node.child_by_field_name("constructor")),
            "macro_invocation" => ("macro", node.child_by_field_name("macro")),
            "type_identifier" => ("type", Some(node)),
            "type" if language == "python" => ("type", Some(node)),
            _ => return true,
        };
        if let Some(target) = target {
            let target = text(target, source);
            if target.len() > 200 || target.is_empty() {
                return true;
            }
            if seen.insert((edge, target.to_owned())) {
                if edges.len() == 40 {
                    truncated = true;
                    return false;
                }
                edges.push(json!({"kind":edge,"target":target,"line":node.start_position().row+1}));
            }
        }
        true
    })?;
    let source = text(selected, source);
    let mut snippet = String::new();
    for (index, line) in source.lines().enumerate() {
        if index >= max_lines || snippet.len() + line.len() + 1 > 12_000 {
            truncated = true;
            break;
        }
        if !snippet.is_empty() {
            snippet.push('\n');
        }
        snippet.push_str(line);
    }
    Ok(json!({
        "analysis":"syntax","has_parse_errors":root.has_error(),
        "symbol":location(selected,name,kind),"source":snippet,
        "dependencies":edges,"imports":imports,"truncated":truncated,
        "notice":"Parser-derived syntax edges, not compiler-resolved types or references. Imports are file-level; calls may include nested definitions. Use find_references for repository-wide identifier matches or a connected language server for semantic resolution."
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retrieves_multiline_bodies_and_dependencies_without_comment_matches()
    -> Result<(), &'static str> {
        let source = "use crate::Config;\n// fn fake() { evil(); }\npub fn run(\n config: Config,\n) -> Output {\n // ignored_call();\n let message = \"fake_call()\";\n make(config)\n}\nfn other() {}";
        let result = context("rust", source, "run", None, 100)?;
        assert_eq!(result["symbol"]["start_line"], 3);
        assert_eq!(result["symbol"]["end_line"], 9);
        assert!(result["source"].as_str().unwrap_or_default().ends_with('}'));
        assert!(
            !result["source"]
                .as_str()
                .unwrap_or_default()
                .contains("fn other")
        );
        let deps = result["dependencies"].to_string();
        assert!(deps.contains("make") && deps.contains("Config") && deps.contains("Output"));
        assert!(!deps.contains("ignored_call") && !deps.contains("fake_call"));
        assert_eq!(result["imports"][0]["text"], "use crate::Config;");
        Ok(())
    }
    #[test]
    fn supports_script_python_and_duplicate_method_disambiguation() -> Result<(), &'static str> {
        for (lang, source, name) in [
            (
                "typescript",
                "import { load } from './db';\nexport async function run(x: Input): Promise<Output> { return load(x); }",
                "run",
            ),
            (
                "tsx",
                "export const View = () => <main>Hello</main>;",
                "View",
            ),
            (
                "javascript",
                "export function run(x) { return load(x); }",
                "run",
            ),
            (
                "python",
                "from db import load\ndef run(x: Input) -> Output:\n    return load(x)\n",
                "run",
            ),
        ] {
            let result = context(lang, source, name, None, 100)?;
            assert_eq!(result["analysis"], "syntax", "{lang}: {result}");
            assert_eq!(result["has_parse_errors"], false, "{lang}");
            if lang == "python" {
                let dependencies = result["dependencies"].to_string();
                assert!(dependencies.contains("Input") && dependencies.contains("Output"));
            }
            assert_eq!(
                definitions(lang, source, 10).ok_or("missing definitions")?[0].name,
                name
            );
        }
        let source = "class A:\n def run(self):\n  return first()\nclass B:\n def run(self):\n  return second()";
        assert_eq!(
            context("python", source, "run", None, 100)?["ambiguous"],
            true
        );
        let selected = context("python", source, "run", Some(5), 100)?;
        assert!(
            selected["source"]
                .as_str()
                .unwrap_or_default()
                .contains("second()")
        );
        assert!(
            !selected["source"]
                .as_str()
                .unwrap_or_default()
                .contains("first()")
        );
        Ok(())
    }
    #[test]
    fn bounds_source_and_declares_unsupported_languages() -> Result<(), &'static str> {
        let result = context(
            "rust",
            "fn run() {\n first();\n second();\n}",
            "run",
            None,
            1,
        )?;
        assert_eq!(result["truncated"], true);
        assert_eq!(result["source"], "fn run() {");
        assert!(matches!(
            context("go", "", "run", None, 10),
            Err("code_context_language_unsupported")
        ));
        assert!(context("rust", &" ".repeat(1_048_577), "run", None, 10).is_err());
        Ok(())
    }
}
