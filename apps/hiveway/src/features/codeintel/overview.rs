//! Repository overview computed once per snapshot: languages, layout, manifests,
//! detected stack, build/test commands and entry points. Agents read this
//! instead of rediscovering the same facts with dozens of file reads.
use super::symbols::is_code;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

/// One indexed file: path, size and detected language.
pub struct File<'a> {
    pub path: &'a str,
    pub bytes: i64,
    pub language: &'a str,
}

/// Paths whose contents feed stack and command detection.
#[must_use]
pub fn manifest(path: &str) -> bool {
    let file = path.rsplit('/').next().unwrap_or(path);
    matches!(
        file,
        "package.json"
            | "Cargo.toml"
            | "pyproject.toml"
            | "requirements.txt"
            | "go.mod"
            | "Makefile"
            | "justfile"
            | "composer.json"
            | "Gemfile"
            | "pom.xml"
            | "build.gradle"
            | "build.gradle.kts"
            | "deno.json"
            | "mix.exs"
            | "Package.swift"
    ) && path.split('/').count() <= 3
        && !path.contains("node_modules/")
}
fn tooling(path: &str) -> Option<&'static str> {
    let file = path.rsplit('/').next().unwrap_or(path);
    if path.starts_with(".github/workflows/") {
        return Some("github-actions");
    }
    match file {
        "Dockerfile"
        | "compose.yaml"
        | "compose.yml"
        | "docker-compose.yml"
        | "docker-compose.yaml" => Some("docker"),
        "flake.nix" | "devenv.nix" | "shell.nix" => Some("nix"),
        "nx.json" => Some("nx"),
        "turbo.json" => Some("turborepo"),
        "lerna.json" => Some("lerna"),
        "bun.lock" | "bun.lockb" | "bunfig.toml" => Some("bun"),
        "pnpm-lock.yaml" | "pnpm-workspace.yaml" => Some("pnpm"),
        "yarn.lock" => Some("yarn"),
        "tsconfig.json" => Some("typescript"),
        "terraform.tf" | "main.tf" => Some("terraform"),
        ".gitlab-ci.yml" => Some("gitlab-ci"),
        _ if file.starts_with("Dockerfile") => Some("docker"),
        _ => None,
    }
}
const FRAMEWORKS: &[(&str, &str)] = &[
    ("next", "next.js"),
    ("react", "react"),
    ("vue", "vue"),
    ("nuxt", "nuxt"),
    ("svelte", "svelte"),
    ("@sveltejs/kit", "sveltekit"),
    ("@angular/core", "angular"),
    ("express", "express"),
    ("fastify", "fastify"),
    ("@nestjs/core", "nestjs"),
    ("hono", "hono"),
    ("vite", "vite"),
    ("tailwindcss", "tailwind"),
    ("@prisma/client", "prisma"),
    ("prisma", "prisma"),
    ("drizzle-orm", "drizzle"),
    ("typeorm", "typeorm"),
    ("jest", "jest"),
    ("vitest", "vitest"),
    ("@playwright/test", "playwright"),
    ("electron", "electron"),
    ("react-native", "react-native"),
    ("expo", "expo"),
    ("astro", "astro"),
    ("@remix-run/react", "remix"),
    ("solid-js", "solid"),
    ("@trpc/server", "trpc"),
    ("graphql", "graphql"),
    ("@modelcontextprotocol/sdk", "mcp"),
    ("@modelcontextprotocol/server", "mcp"),
    ("axum", "axum"),
    ("actix-web", "actix-web"),
    ("rocket", "rocket"),
    ("warp", "warp"),
    ("tokio", "tokio"),
    ("sea-orm", "sea-orm"),
    ("diesel", "diesel"),
    ("sqlx", "sqlx"),
    ("tauri", "tauri"),
    ("bevy", "bevy"),
    ("leptos", "leptos"),
    ("yew", "yew"),
    ("wasm-bindgen", "wasm"),
    ("clap", "clap"),
    ("django", "django"),
    ("flask", "flask"),
    ("fastapi", "fastapi"),
    ("pydantic", "pydantic"),
    ("sqlalchemy", "sqlalchemy"),
    ("pytest", "pytest"),
    ("pandas", "pandas"),
    ("numpy", "numpy"),
    ("torch", "pytorch"),
    ("tensorflow", "tensorflow"),
    ("langchain", "langchain"),
    ("github.com/gin-gonic/gin", "gin"),
    ("github.com/labstack/echo", "echo"),
    ("github.com/gofiber/fiber", "fiber"),
    ("gorm.io/gorm", "gorm"),
    ("github.com/spf13/cobra", "cobra"),
    ("rails", "rails"),
    ("laravel/framework", "laravel"),
    ("phoenix", "phoenix"),
];
fn framework(dependency: &str) -> Option<&'static str> {
    let dependency = dependency.trim().to_ascii_lowercase();
    FRAMEWORKS.iter().find_map(|(name, label)| {
        (dependency == *name || dependency.starts_with(&format!("{name}/v"))).then_some(*label)
    })
}

#[derive(Default)]
struct Detected {
    frameworks: BTreeSet<&'static str>,
    commands: BTreeMap<&'static str, String>,
    packages: Vec<Value>,
}
fn package_json(path: &str, content: &str, manager: &str, found: &mut Detected) {
    let Ok(value) = serde_json::from_str::<Value>(content) else {
        return;
    };
    for section in ["dependencies", "devDependencies", "peerDependencies"] {
        for name in value[section]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(k, _)| k)
        {
            found.frameworks.extend(framework(name));
        }
    }
    let scripts: Vec<String> = value["scripts"]
        .as_object()
        .into_iter()
        .flatten()
        .map(|(k, _)| k.clone())
        .take(40)
        .collect();
    if !path.contains('/') {
        for (key, label) in [
            ("build", "build"),
            ("test", "test"),
            ("lint", "lint"),
            ("check", "check"),
            ("typecheck", "typecheck"),
            ("check-types", "typecheck"),
            ("dev", "dev"),
            ("start", "start"),
            ("format", "format"),
        ] {
            if scripts.iter().any(|s| s == key) {
                found
                    .commands
                    .entry(label)
                    .or_insert_with(|| format!("{manager} run {key}"));
            }
        }
    }
    found
        .packages
        .push(json!({"path":path,"name":value["name"],"scripts":scripts}));
}
fn cargo(path: &str, content: &str, found: &mut Detected) {
    let Ok(value) = content.parse::<toml::Table>() else {
        return;
    };
    for section in ["dependencies", "dev-dependencies", "workspace"] {
        let table = if section == "workspace" {
            value.get("workspace").and_then(|w| w.get("dependencies"))
        } else {
            value.get(section)
        };
        for name in table
            .and_then(toml::Value::as_table)
            .into_iter()
            .flatten()
            .map(|(k, _)| k)
        {
            found.frameworks.extend(framework(name));
        }
    }
    if !path.contains('/') {
        found
            .commands
            .entry("build")
            .or_insert_with(|| "cargo build".into());
        found
            .commands
            .entry("test")
            .or_insert_with(|| "cargo test".into());
        found
            .commands
            .entry("lint")
            .or_insert_with(|| "cargo clippy --all-targets".into());
    }
    let members: Vec<String> = value
        .get("workspace")
        .and_then(|w| w.get("members"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|m| m.as_str().map(str::to_owned))
        .take(40)
        .collect();
    let name = value
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str);
    found
        .packages
        .push(json!({"path":path,"name":name,"workspace_members":members}));
}
fn pyproject(path: &str, content: &str, found: &mut Detected) {
    let Ok(value) = content.parse::<toml::Table>() else {
        return;
    };
    let dependencies = value
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str);
    for dependency in dependencies {
        let name = dependency
            .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .next()
            .unwrap_or_default();
        found.frameworks.extend(framework(name));
    }
    let name = value
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(toml::Value::as_str);
    found.packages.push(json!({"path":path,"name":name}));
}
fn requirements(content: &str, found: &mut Detected) {
    for line in content.lines().take(500) {
        let name = line
            .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
            .next()
            .unwrap_or_default();
        found.frameworks.extend(framework(name));
    }
}
fn go_mod(path: &str, content: &str, found: &mut Detected) {
    let module = content
        .lines()
        .find_map(|l| l.strip_prefix("module "))
        .map(str::trim);
    for line in content.lines().take(1000) {
        let dependency = line
            .trim()
            .trim_start_matches("require ")
            .split_whitespace()
            .next()
            .unwrap_or_default();
        found.frameworks.extend(framework(dependency));
    }
    if !path.contains('/') {
        found
            .commands
            .entry("build")
            .or_insert_with(|| "go build ./...".into());
        found
            .commands
            .entry("test")
            .or_insert_with(|| "go test ./...".into());
        found
            .commands
            .entry("lint")
            .or_insert_with(|| "go vet ./...".into());
    }
    found.packages.push(json!({"path":path,"name":module}));
}
fn targets(content: &str, make: bool) -> Vec<String> {
    content
        .lines()
        .filter(|line| !line.starts_with([' ', '\t', '#', '.']))
        .filter_map(|line| {
            let (name, rest) = line.split_once(':')?;
            let name = name.split_whitespace().next()?;
            let valid = !name.is_empty()
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                && (!make || !rest.starts_with('='));
            valid.then(|| name.to_owned())
        })
        .take(40)
        .collect()
}
fn task_runner(path: &str, content: &str, make: bool, found: &mut Detected) {
    let names = targets(content, make);
    if !path.contains('/') {
        let tool = if make { "make" } else { "just" };
        for key in ["build", "test", "lint", "check", "dev", "format"] {
            if names.iter().any(|n| n == key) {
                found
                    .commands
                    .entry(key)
                    .or_insert_with(|| format!("{tool} {key}"));
            }
        }
    }
    found.packages.push(json!({"path":path,"targets":names}));
}

/// Build the overview from file metadata and manifest contents.
#[must_use]
pub fn summarize(files: &[File<'_>], manifests: &[(String, String)]) -> Value {
    let mut languages: BTreeMap<&str, (i64, i64)> = BTreeMap::new();
    let mut directories: BTreeMap<&str, i64> = BTreeMap::new();
    let mut tools: BTreeSet<&'static str> = BTreeSet::new();
    let mut entry_points: Vec<&str> = Vec::new();
    let mut bytes = 0i64;
    for file in files {
        bytes += file.bytes;
        let entry = languages.entry(file.language).or_default();
        entry.0 += 1;
        entry.1 += file.bytes;
        if let Some((top, _)) = file.path.split_once('/') {
            *directories.entry(top).or_default() += 1;
        }
        tools.extend(tooling(file.path));
        let name = file.path.rsplit('/').next().unwrap_or(file.path);
        let depth = file.path.split('/').count();
        if depth <= 4
            && matches!(
                name,
                "main.rs"
                    | "lib.rs"
                    | "main.go"
                    | "main.py"
                    | "app.py"
                    | "manage.py"
                    | "__main__.py"
                    | "index.ts"
                    | "index.js"
                    | "server.ts"
                    | "server.js"
                    | "main.ts"
                    | "main.tsx"
                    | "App.tsx"
                    | "app.ts"
                    | "Program.cs"
                    | "Main.java"
                    | "main.swift"
            )
            && !file.path.contains("node_modules/")
            && !file.path.contains("/test")
        {
            entry_points.push(file.path);
        }
    }
    entry_points.sort_by_key(|p| (p.split('/').count(), p.len()));
    entry_points.truncate(12);
    let manager = if tools.contains("bun") {
        "bun"
    } else if tools.contains("pnpm") {
        "pnpm"
    } else if tools.contains("yarn") {
        "yarn"
    } else {
        "npm"
    };
    let mut found = Detected::default();
    let mut ordered: Vec<&(String, String)> = manifests.iter().collect();
    ordered.sort_by_key(|(path, _)| (path.split('/').count(), path.clone()));
    for (path, content) in ordered.into_iter().take(40) {
        match path.rsplit('/').next().unwrap_or(path) {
            "package.json" => package_json(path, content, manager, &mut found),
            "Cargo.toml" => cargo(path, content, &mut found),
            "pyproject.toml" => pyproject(path, content, &mut found),
            "requirements.txt" => requirements(content, &mut found),
            "go.mod" => go_mod(path, content, &mut found),
            "Makefile" => task_runner(path, content, true, &mut found),
            "justfile" => task_runner(path, content, false, &mut found),
            _ => found.packages.push(json!({"path":path})),
        }
    }
    let mut code: Vec<(&str, i64)> = languages
        .iter()
        .filter(|(language, _)| is_code(language))
        .map(|(language, (_, bytes))| (*language, *bytes))
        .collect();
    code.sort_by_key(|entry| std::cmp::Reverse(entry.1));
    let mut stack: Vec<String> = code.iter().take(5).map(|(l, _)| (*l).to_owned()).collect();
    stack.extend(found.frameworks.iter().map(|f| (*f).to_owned()));
    stack.extend(tools.iter().map(|t| (*t).to_owned()));
    stack.dedup();
    let mut top: Vec<(&str, i64)> = directories.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    top.truncate(30);
    json!({
        "files": files.len(),
        "bytes": bytes,
        "languages": languages.iter().map(|(l, (files, bytes))| json!({"language":l,"files":files,"bytes":bytes})).collect::<Vec<_>>(),
        "top_level": top.iter().map(|(d, files)| json!({"path":d,"files":files})).collect::<Vec<_>>(),
        "stack": stack,
        "commands": found.commands,
        "entry_points": entry_points,
        "manifests": found.packages,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overview_detects_stack_commands_and_entry_points() {
        let files = [
            File {
                path: "package.json",
                bytes: 100,
                language: "json",
            },
            File {
                path: "bun.lock",
                bytes: 10,
                language: "text",
            },
            File {
                path: "apps/api/Cargo.toml",
                bytes: 50,
                language: "toml",
            },
            File {
                path: "apps/api/src/main.rs",
                bytes: 4000,
                language: "rust",
            },
            File {
                path: "apps/web/app/page.tsx",
                bytes: 900,
                language: "tsx",
            },
            File {
                path: ".github/workflows/ci.yml",
                bytes: 300,
                language: "yaml",
            },
            File {
                path: "Makefile",
                bytes: 40,
                language: "make",
            },
        ];
        let manifests = vec![
            ("package.json".into(), r#"{"name":"mono","scripts":{"test":"bun test","dev":"x"},"devDependencies":{"next":"16","tailwindcss":"4"}}"#.into()),
            ("apps/api/Cargo.toml".into(), "[package]\nname=\"api\"\n[dependencies]\naxum=\"0.8\"\nsea-orm={version=\"2\"}\n".into()),
            ("Makefile".into(), "build:\n\tcargo build\nVAR := 1\nlint: build\n".into()),
        ];
        let summary = summarize(&files, &manifests);
        let stack: Vec<&str> = summary["stack"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .collect();
        for expected in [
            "rust",
            "tsx",
            "next.js",
            "tailwind",
            "axum",
            "sea-orm",
            "bun",
            "github-actions",
        ] {
            assert!(
                stack.contains(&expected),
                "{expected} missing from {stack:?}"
            );
        }
        assert_eq!(summary["commands"]["test"], "bun run test");
        assert_eq!(summary["commands"]["build"], "make build");
        assert_eq!(summary["commands"]["lint"], "make lint");
        assert_eq!(summary["entry_points"][0], "apps/api/src/main.rs");
        assert_eq!(summary["top_level"][0]["path"], "apps");
        assert!(manifest("apps/api/Cargo.toml") && !manifest("node_modules/x/package.json"));
    }
}
