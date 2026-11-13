//! Lightweight, dependency-free source analysis: language detection, top-level
//! symbol extraction and identifier-aware search text. Heuristics are line based
//! on purpose; they are fast, bounded and good enough to answer "where is X
//! defined" across many languages without shipping a parser per language.

/// Language for a repository path, by file name first and extension second.
#[must_use]
pub fn language(path: &str) -> &'static str {
    let file = path.rsplit('/').next().unwrap_or(path);
    let lower = file.to_ascii_lowercase();
    match lower.as_str() {
        "dockerfile" | "containerfile" => return "dockerfile",
        "makefile" | "gnumakefile" => return "make",
        "justfile" => return "just",
        "cmakelists.txt" => return "cmake",
        "gemfile" | "rakefile" => return "ruby",
        _ => {}
    }
    if lower.starts_with("dockerfile.") {
        return "dockerfile";
    }
    let extension = lower.rsplit_once('.').map_or("", |(_, ext)| ext);
    match extension {
        "rs" => "rust",
        "ts" | "mts" | "cts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "jsx",
        "py" | "pyi" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "scala" => "scala",
        "swift" => "swift",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "c" | "h" => "c",
        "cc" | "cpp" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
        "m" | "mm" => "objc",
        "ex" | "exs" => "elixir",
        "erl" | "hrl" => "erlang",
        "hs" => "haskell",
        "ml" | "mli" => "ocaml",
        "clj" | "cljs" | "cljc" => "clojure",
        "dart" => "dart",
        "lua" => "lua",
        "r" => "r",
        "jl" => "julia",
        "zig" => "zig",
        "nim" => "nim",
        "sol" => "solidity",
        "sh" | "bash" | "zsh" | "fish" => "shell",
        "ps1" => "powershell",
        "sql" => "sql",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "vue" => "vue",
        "svelte" => "svelte",
        "astro" => "astro",
        "html" | "htm" => "html",
        "css" | "scss" | "sass" | "less" => "css",
        "md" | "mdx" | "markdown" => "markdown",
        "json" | "jsonc" => "json",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "xml" => "xml",
        "nix" => "nix",
        "tf" | "hcl" => "hcl",
        _ => "text",
    }
}
/// Languages that count as source code in overviews (not config or prose).
#[must_use]
pub fn is_code(language: &str) -> bool {
    !matches!(
        language,
        "text"
            | "markdown"
            | "json"
            | "yaml"
            | "toml"
            | "xml"
            | "html"
            | "css"
            | "make"
            | "just"
            | "dockerfile"
            | "cmake"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub kind: &'static str,
    pub line: i32,
    pub signature: String,
}

fn identifier(rest: &str) -> Option<String> {
    let name: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
        .collect();
    (!name.is_empty() && !name.chars().next().is_some_and(char::is_numeric)).then_some(name)
}
fn strip_any<'a>(mut line: &'a str, prefixes: &[&str]) -> &'a str {
    loop {
        let before = line;
        for prefix in prefixes {
            if let Some(rest) = line.strip_prefix(prefix) {
                line = rest.trim_start();
            }
        }
        if line.len() == before.len() {
            return line;
        }
    }
}
fn keyword<'a>(
    line: &'a str,
    keywords: &[(&str, &'static str)],
) -> Option<(&'a str, &'static str)> {
    keywords.iter().find_map(|(keyword, kind)| {
        line.strip_prefix(keyword)
            .filter(|rest| rest.starts_with([' ', '\t', '<', '(']) || keyword.ends_with('!'))
            .map(|rest| (rest, *kind))
    })
}

/// Skip a leading `<...>` generic parameter list, including nested brackets.
fn skip_generics(rest: &str) -> &str {
    let rest = rest.trim_start();
    if !rest.starts_with('<') {
        return rest;
    }
    let mut depth = 0usize;
    for (index, c) in rest.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return rest[index + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    ""
}
fn rust(line: &str, indented: bool) -> Option<(String, &'static str)> {
    let rest = strip_any(
        line,
        &[
            "pub(crate) ",
            "pub(super) ",
            "pub ",
            "async ",
            "unsafe ",
            "extern \"C\" ",
            "default ",
        ],
    );
    // `const fn` is a function; a bare `const` is a constant.
    let rest = match rest.strip_prefix("const ") {
        Some(after) if strip_any(after, &["async ", "unsafe "]).starts_with("fn ") => {
            strip_any(after, &["async ", "unsafe "])
        }
        _ => rest,
    };
    if let Some(rest) = rest.strip_prefix("fn ") {
        return identifier(rest).map(|n| (n, if indented { "method" } else { "function" }));
    }
    if let Some(rest) = rest
        .strip_prefix("impl")
        .filter(|r| r.starts_with([' ', '<']))
    {
        let rest = skip_generics(rest);
        let target = rest.rsplit(" for ").next().unwrap_or(rest);
        let target = target.split(['<', '{', ' ']).find(|s| !s.is_empty())?;
        let name = target.rsplit("::").next().unwrap_or(target);
        return identifier(name).map(|n| (n, "impl"));
    }
    let (rest, kind) = keyword(
        rest,
        &[
            ("struct", "struct"),
            ("enum", "enum"),
            ("trait", "trait"),
            ("type", "type"),
            ("mod", "module"),
            ("union", "union"),
            ("const", "constant"),
            ("static", "constant"),
            ("macro_rules!", "macro"),
        ],
    )?;
    let rest = rest.trim_start().trim_start_matches("mut ");
    identifier(rest).map(|n| (n, kind))
}
fn script(line: &str, indented: bool) -> Option<(String, &'static str)> {
    let rest = strip_any(
        line,
        &["export ", "default ", "declare ", "async ", "abstract "],
    );
    if let Some(rest) = rest.strip_prefix("function") {
        let rest = rest.trim_start_matches('*');
        return identifier(rest).map(|n| (n, "function"));
    }
    if let Some((rest, kind)) = keyword(
        rest,
        &[
            ("class", "class"),
            ("interface", "interface"),
            ("enum", "enum"),
            ("namespace", "module"),
            ("type", "type"),
        ],
    ) {
        return identifier(rest).map(|n| (n, kind));
    }
    if indented {
        return None;
    }
    let (rest, _) = keyword(
        rest,
        &[
            ("const", "constant"),
            ("let", "variable"),
            ("var", "variable"),
        ],
    )?;
    let name = identifier(rest)?;
    let after = rest.trim_start()[name.len()..].trim_start();
    let assigned = after.starts_with('=') || after.starts_with(':');
    let arrow = after.contains("=>") || after.contains("function");
    assigned.then_some((name, if arrow { "function" } else { "constant" }))
}
fn python(line: &str, indented: bool) -> Option<(String, &'static str)> {
    let rest = line.strip_prefix("async ").unwrap_or(line);
    if let Some(rest) = rest.strip_prefix("def ") {
        return identifier(rest).map(|n| (n, if indented { "method" } else { "function" }));
    }
    rest.strip_prefix("class ")
        .and_then(identifier)
        .map(|n| (n, "class"))
}
fn go(line: &str) -> Option<(String, &'static str)> {
    if let Some(rest) = line.strip_prefix("func ") {
        if let Some(receiver) = rest.strip_prefix('(') {
            let after = receiver.split_once(')')?.1;
            return identifier(after).map(|n| (n, "method"));
        }
        return identifier(rest).map(|n| (n, "function"));
    }
    if let Some(rest) = line.strip_prefix("type ") {
        let name = identifier(rest)?;
        let kind = if rest.contains(" interface") {
            "interface"
        } else if rest.contains(" struct") {
            "struct"
        } else {
            "type"
        };
        return Some((name, kind));
    }
    None
}
fn jvm(line: &str) -> Option<(String, &'static str)> {
    let rest = strip_any(
        line,
        &[
            "public ",
            "private ",
            "protected ",
            "internal ",
            "static ",
            "final ",
            "abstract ",
            "sealed ",
            "open ",
            "data ",
            "partial ",
            "export ",
            "inline ",
            "override ",
            "suspend ",
            "@objc ",
            "fileprivate ",
        ],
    );
    if let Some((rest, kind)) = keyword(
        rest,
        &[
            ("class", "class"),
            ("interface", "interface"),
            ("enum", "enum"),
            ("record", "record"),
            ("struct", "struct"),
            ("object", "object"),
            ("protocol", "interface"),
            ("trait", "trait"),
            ("extension", "impl"),
        ],
    ) {
        return identifier(rest).map(|n| (n, kind));
    }
    rest.strip_prefix("fun ")
        .or_else(|| rest.strip_prefix("func "))
        .or_else(|| rest.strip_prefix("def "))
        .and_then(|rest| identifier(rest.trim_start_matches(['<', '>'])))
        .map(|n| (n, "function"))
}
fn ruby_like(line: &str, indented: bool) -> Option<(String, &'static str)> {
    if let Some(rest) = line
        .strip_prefix("def ")
        .or_else(|| line.strip_prefix("defp "))
    {
        let rest = rest.trim_start_matches("self.");
        return identifier(rest).map(|n| (n, if indented { "method" } else { "function" }));
    }
    let (rest, kind) = keyword(
        line,
        &[
            ("class", "class"),
            ("module", "module"),
            ("defmodule", "module"),
        ],
    )?;
    let name: String = rest
        .trim_start()
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':' || *c == '.')
        .collect();
    (!name.is_empty()).then_some((name, kind))
}
fn php(line: &str) -> Option<(String, &'static str)> {
    let rest = strip_any(
        line,
        &[
            "public ",
            "private ",
            "protected ",
            "static ",
            "final ",
            "abstract ",
        ],
    );
    if let Some(rest) = rest.strip_prefix("function ") {
        return identifier(rest.trim_start_matches('&')).map(|n| (n, "function"));
    }
    let (rest, kind) = keyword(
        rest,
        &[
            ("class", "class"),
            ("interface", "interface"),
            ("trait", "trait"),
            ("enum", "enum"),
        ],
    )?;
    identifier(rest).map(|n| (n, kind))
}
fn c_like(line: &str, indented: bool) -> Option<(String, &'static str)> {
    if let Some(rest) = line.strip_prefix("#define ") {
        return identifier(rest).map(|n| (n, "macro"));
    }
    let rest = strip_any(
        line,
        &["typedef ", "static ", "inline ", "extern ", "template<"],
    );
    if let Some((rest, kind)) = keyword(
        rest,
        &[
            ("struct", "struct"),
            ("class", "class"),
            ("enum", "enum"),
            ("union", "union"),
            ("namespace", "module"),
        ],
    ) {
        let rest = rest.trim_start().trim_start_matches("class ");
        let name = identifier(rest)?;
        return Some((name, kind));
    }
    if indented || line.ends_with(';') {
        return None;
    }
    // `return_type name(args) {` at column zero.
    let (head, _) = line.split_once('(')?;
    let name = head
        .split_whitespace()
        .last()?
        .trim_start_matches(['*', '&']);
    let name = name.rsplit("::").next().unwrap_or(name);
    let control = ["if", "for", "while", "switch", "return", "sizeof", "else"];
    (head.split_whitespace().count() >= 2 && !control.contains(&name))
        .then(|| identifier(name))
        .flatten()
        .map(|n| (n, "function"))
}
fn shell(line: &str) -> Option<(String, &'static str)> {
    if let Some(rest) = line.strip_prefix("function ") {
        return identifier(rest).map(|n| (n, "function"));
    }
    let (name, rest) = line.split_once("()")?;
    rest.trim_start()
        .starts_with('{')
        .then(|| identifier(name).filter(|n| n.len() == name.trim().len()))
        .flatten()
        .map(|n| (n, "function"))
}

/// Extract up to `max` definitions with 1-based line numbers.
#[must_use]
pub fn extract(language: &str, content: &str, max: usize) -> Vec<Symbol> {
    let mut symbols = Vec::new();
    for (index, raw) in content.lines().enumerate() {
        if symbols.len() >= max {
            break;
        }
        if raw.len() > 1000 {
            continue;
        }
        let indented = raw.starts_with([' ', '\t']);
        let line = raw.trim();
        if line.is_empty()
            || line.starts_with("//")
            || line.starts_with('#') && language != "c" && language != "cpp"
        {
            continue;
        }
        let found = match language {
            "rust" => rust(line, indented),
            "typescript" | "tsx" | "javascript" | "jsx" | "vue" | "svelte" | "astro" => {
                script(line, indented)
            }
            "python" => python(line, indented),
            "go" => go(line),
            "java" | "kotlin" | "scala" | "swift" | "csharp" | "dart" => jvm(line),
            "ruby" | "elixir" => ruby_like(line, indented),
            "php" => php(line),
            "c" | "cpp" | "objc" => c_like(line, indented),
            "shell" => shell(line),
            _ => None,
        };
        if let Some((name, kind)) = found
            && name.len() <= 200
        {
            symbols.push(Symbol {
                name,
                kind,
                line: i32::try_from(index + 1).unwrap_or(i32::MAX),
                signature: line.chars().take(200).collect(),
            });
        }
    }
    symbols
}

/// Split `getUserName`, `get_user_name` and `HTTPServer` into lower-case words.
#[must_use]
pub fn split_identifier(word: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = word.chars().collect();
    for (index, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                parts.push(std::mem::take(&mut current));
            }
            continue;
        }
        let previous = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let next = chars.get(index + 1).copied();
        let boundary = c.is_uppercase()
            && previous.is_some_and(|p| {
                p.is_lowercase()
                    || p.is_numeric()
                    || (p.is_uppercase() && next.is_some_and(char::is_lowercase))
            });
        if boundary && !current.is_empty() {
            parts.push(std::mem::take(&mut current));
        }
        current.extend(c.to_lowercase());
    }
    if !current.is_empty() {
        parts.push(current);
    }
    parts
}
/// Words to index: every identifier whole (lower-cased) plus its parts, bounded.
#[must_use]
pub fn search_text(content: &str, max_bytes: usize) -> String {
    let mut out = String::with_capacity(content.len().min(max_bytes));
    for word in content.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if word.is_empty() || word.len() > 80 {
            continue;
        }
        let parts = split_identifier(word);
        let whole: String = word
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>()
            .to_lowercase();
        let extra = if parts.len() > 1 {
            parts.join(" ")
        } else {
            String::new()
        };
        if out.len() + whole.len() + extra.len() + 2 > max_bytes {
            break;
        }
        out.push_str(&whole);
        out.push(' ');
        if !extra.is_empty() {
            out.push_str(&extra);
            out.push(' ');
        }
    }
    out
}
/// Query terms in the same normalized space as [`search_text`].
#[must_use]
pub fn query_terms(query: &str) -> Vec<String> {
    let mut terms = Vec::new();
    for word in query.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
        if word.is_empty() {
            continue;
        }
        let parts = split_identifier(word);
        if parts.len() > 1 {
            terms.extend(parts);
        } else {
            terms.push(word.to_lowercase());
        }
    }
    terms.sort();
    terms.dedup();
    terms.truncate(16);
    terms
}

#[cfg(test)]
mod tests {
    use super::*;
    fn names(language: &str, source: &str) -> Vec<(String, &'static str)> {
        extract(language, source, 100)
            .into_iter()
            .map(|s| (s.name, s.kind))
            .collect()
    }
    #[test]
    fn languages_come_from_names_then_extensions() {
        assert_eq!(language("src/main.rs"), "rust");
        assert_eq!(language("docker/Dockerfile"), "dockerfile");
        assert_eq!(language("Dockerfile.dev"), "dockerfile");
        assert_eq!(language("web/App.TSX"), "tsx");
        assert_eq!(language("README"), "text");
        assert!(is_code("go") && !is_code("yaml"));
    }
    #[test]
    fn extracts_definitions_across_languages() {
        assert_eq!(
            names(
                "rust",
                "pub async fn serve() {}\npub(crate) struct State;\nimpl<T: Into<u8>> Display for Wrapper<T> {\n    fn fmt(&self) {}\n}\nimpl<T> Store<T> {\nmacro_rules! hive {}\nconst LIMIT: u8 = 1;\npub const fn size() -> usize {"
            ),
            vec![
                ("serve".into(), "function"),
                ("State".into(), "struct"),
                ("Wrapper".into(), "impl"),
                ("fmt".into(), "method"),
                ("Store".into(), "impl"),
                ("hive".into(), "macro"),
                ("LIMIT".into(), "constant"),
                ("size".into(), "function"),
            ]
        );
        assert_eq!(
            names(
                "typescript",
                "export default async function main() {}\nexport const handler = async () => {}\nexport interface Config {}\ntype Id = string;\n  const local = 1;"
            ),
            vec![
                ("main".into(), "function"),
                ("handler".into(), "function"),
                ("Config".into(), "interface"),
                ("Id".into(), "type"),
            ]
        );
        assert_eq!(
            names(
                "python",
                "class Repo:\n    async def load(self):\n        pass\ndef main():\n    pass"
            ),
            vec![
                ("Repo".into(), "class"),
                ("load".into(), "method"),
                ("main".into(), "function")
            ]
        );
        assert_eq!(
            names(
                "go",
                "func (s *Server) Start() error {\ntype Handler interface {\nfunc main() {"
            ),
            vec![
                ("Start".into(), "method"),
                ("Handler".into(), "interface"),
                ("main".into(), "function")
            ]
        );
        assert_eq!(
            names(
                "c",
                "#define MAX 4\nstatic int parse(char *s) {\n  if (x) {"
            ),
            vec![("MAX".into(), "macro"), ("parse".into(), "function")]
        );
        assert_eq!(
            names("shell", "deploy() {\nfunction build {"),
            vec![("deploy".into(), "function"), ("build".into(), "function")]
        );
    }
    #[test]
    fn identifiers_split_for_search() {
        assert_eq!(split_identifier("getUserName"), vec!["get", "user", "name"]);
        assert_eq!(split_identifier("HTTPServer2"), vec!["http", "server2"]);
        assert_eq!(
            split_identifier("snake_case_id"),
            vec!["snake", "case", "id"]
        );
        let text = search_text("fn parseConfig(path: &str)", 1000);
        assert!(text.contains("parseconfig") && text.contains("parse config"));
        assert_eq!(
            query_terms("parseConfig path"),
            vec!["config", "parse", "path"]
        );
        assert!(search_text(&"word ".repeat(1000), 50).len() <= 50);
    }
}
