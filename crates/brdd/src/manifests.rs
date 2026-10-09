//! Version and dependency facts from common manifests.

use std::path::Path;

use serde::{Deserialize, Serialize};

const MAX_DEPENDENCIES: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// `runtime` | `dev` | `build`
    pub kind: String,
    /// Manifest the entry came from.
    pub source: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifestFacts {
    pub manifests: Vec<String>,
    pub version: Option<String>,
    pub version_source: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub description: Option<String>,
}

/// Manifests / markers looked for in the project root.
pub const KNOWN_MANIFESTS: &[&str] = &[
    "Cargo.toml",
    "package.json",
    "pyproject.toml",
    "requirements.txt",
    "setup.py",
    "go.mod",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "Gemfile",
    "composer.json",
    "Makefile",
    "Dockerfile",
    "docker-compose.yml",
    "compose.yaml",
    "VERSION",
];

pub fn collect(root: &Path) -> ManifestFacts {
    let mut facts = ManifestFacts::default();
    for name in KNOWN_MANIFESTS {
        if root.join(name).is_file() {
            facts.manifests.push((*name).to_string());
        }
    }

    // Order = priority for the version: the first manifest that declares one wins.
    if let Some(text) = read(root, "Cargo.toml") {
        cargo(&text, &mut facts);
    }
    if let Some(text) = read(root, "package.json") {
        package_json(&text, &mut facts);
    }
    if let Some(text) = read(root, "pyproject.toml") {
        pyproject(&text, &mut facts);
    }
    if let Some(text) = read(root, "requirements.txt") {
        requirements(&text, &mut facts);
    }
    if let Some(text) = read(root, "go.mod") {
        go_mod(&text, &mut facts);
    }
    if let Some(text) = read(root, "pom.xml") {
        pom(&text, &mut facts);
    }
    for gradle in ["build.gradle", "build.gradle.kts"] {
        if let Some(text) = read(root, gradle) {
            gradle_version(&text, gradle, &mut facts);
        }
    }
    if let Some(text) = read(root, "VERSION") {
        let v = text.trim();
        if !v.is_empty() && v.len() < 40 {
            set_version(&mut facts, v, "VERSION");
        }
    }

    facts.dependencies.truncate(MAX_DEPENDENCIES);
    facts
}

fn read(root: &Path, name: &str) -> Option<String> {
    let path = root.join(name);
    let meta = std::fs::metadata(&path).ok()?;
    if !meta.is_file() || meta.len() > 2 * 1024 * 1024 {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn set_version(facts: &mut ManifestFacts, version: &str, source: &str) {
    if facts.version.is_none() && !version.trim().is_empty() {
        facts.version = Some(version.trim().to_string());
        facts.version_source = Some(source.to_string());
    }
}

fn push_dep(facts: &mut ManifestFacts, name: &str, version: Option<String>, kind: &str, source: &str) {
    let name = name.trim();
    if name.is_empty() || facts.dependencies.iter().any(|d| d.name == name && d.source == source) {
        return;
    }
    facts.dependencies.push(Dependency {
        name: name.to_string(),
        version: version.filter(|v| !v.trim().is_empty()),
        kind: kind.to_string(),
        source: source.to_string(),
    });
}

fn toml_dep_version(v: &toml::Value) -> Option<String> {
    match v {
        toml::Value::String(s) => Some(s.clone()),
        toml::Value::Table(t) => t
            .get("version")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| t.get("workspace").and_then(|w| w.as_bool()).filter(|w| *w).map(|_| "workspace".into())),
        _ => None,
    }
}

fn cargo(text: &str, facts: &mut ManifestFacts) {
    let Ok(doc) = text.parse::<toml::Table>() else { return };
    let package = doc.get("package").and_then(|p| p.as_table());
    let workspace_pkg = doc
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.as_table());

    let version = package
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .or_else(|| workspace_pkg.and_then(|p| p.get("version")).and_then(|v| v.as_str()));
    if let Some(v) = version {
        set_version(facts, v, "Cargo.toml");
    }
    if facts.description.is_none() {
        facts.description = package
            .and_then(|p| p.get("description"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
    }

    for (section, kind) in [
        ("dependencies", "runtime"),
        ("dev-dependencies", "dev"),
        ("build-dependencies", "build"),
    ] {
        if let Some(table) = doc.get(section).and_then(|d| d.as_table()) {
            for (name, value) in table {
                push_dep(facts, name, toml_dep_version(value), kind, "Cargo.toml");
            }
        }
    }
    if let Some(table) = doc
        .get("workspace")
        .and_then(|w| w.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        for (name, value) in table {
            push_dep(facts, name, toml_dep_version(value), "runtime", "Cargo.toml");
        }
    }
}

fn package_json(text: &str, facts: &mut ManifestFacts) {
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(text) else { return };
    if let Some(v) = doc.get("version").and_then(|v| v.as_str()) {
        set_version(facts, v, "package.json");
    }
    if facts.description.is_none() {
        facts.description = doc.get("description").and_then(|v| v.as_str()).map(str::to_string);
    }
    for (section, kind) in [
        ("dependencies", "runtime"),
        ("devDependencies", "dev"),
        ("peerDependencies", "runtime"),
    ] {
        if let Some(obj) = doc.get(section).and_then(|d| d.as_object()) {
            for (name, value) in obj {
                push_dep(facts, name, value.as_str().map(str::to_string), kind, "package.json");
            }
        }
    }
}

fn split_requirement(spec: &str) -> (String, Option<String>) {
    let spec = spec.split(';').next().unwrap_or(spec).trim();
    let idx = spec.find(|c: char| "=<>!~ [".contains(c));
    match idx {
        Some(i) => {
            let name = spec[..i].trim().to_string();
            let rest = spec[i..].trim_start_matches(|c: char| c == '[' || c.is_whitespace());
            let version = rest
                .split(']')
                .next_back()
                .map(|v| v.trim().trim_start_matches(|c: char| "=<>!~".contains(c)).trim().to_string())
                .filter(|v| !v.is_empty());
            (name, version)
        }
        None => (spec.to_string(), None),
    }
}

fn pyproject(text: &str, facts: &mut ManifestFacts) {
    let Ok(doc) = text.parse::<toml::Table>() else { return };
    let project = doc.get("project").and_then(|p| p.as_table());
    let poetry = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.as_table());

    if let Some(v) = project
        .and_then(|p| p.get("version"))
        .or_else(|| poetry.and_then(|p| p.get("version")))
        .and_then(|v| v.as_str())
    {
        set_version(facts, v, "pyproject.toml");
    }
    if facts.description.is_none() {
        facts.description = project
            .and_then(|p| p.get("description"))
            .or_else(|| poetry.and_then(|p| p.get("description")))
            .and_then(|v| v.as_str())
            .map(str::to_string);
    }
    if let Some(list) = project.and_then(|p| p.get("dependencies")).and_then(|d| d.as_array()) {
        for spec in list.iter().filter_map(|v| v.as_str()) {
            let (name, version) = split_requirement(spec);
            push_dep(facts, &name, version, "runtime", "pyproject.toml");
        }
    }
    if let Some(table) = poetry.and_then(|p| p.get("dependencies")).and_then(|d| d.as_table()) {
        for (name, value) in table.iter().filter(|(n, _)| n.as_str() != "python") {
            push_dep(facts, name, toml_dep_version(value), "runtime", "pyproject.toml");
        }
    }
}

fn requirements(text: &str, facts: &mut ManifestFacts) {
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() || line.starts_with('-') {
            continue;
        }
        let (name, version) = split_requirement(line);
        push_dep(facts, &name, version, "runtime", "requirements.txt");
    }
}

fn go_mod(text: &str, facts: &mut ManifestFacts) {
    let mut in_block = false;
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("").trim();
        if line.starts_with("require (") {
            in_block = true;
            continue;
        }
        if in_block && line == ")" {
            in_block = false;
            continue;
        }
        let spec = if in_block {
            Some(line)
        } else {
            line.strip_prefix("require ")
        };
        if let Some(spec) = spec {
            let mut parts = spec.split_whitespace();
            if let Some(name) = parts.next() {
                push_dep(facts, name, parts.next().map(str::to_string), "runtime", "go.mod");
            }
        }
    }
}

fn strip_block(text: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        match rest[start..].find(&close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

fn pom(text: &str, facts: &mut ManifestFacts) {
    let body = strip_block(&strip_block(&strip_block(text, "parent"), "dependencies"), "build");
    if let Some(start) = body.find("<version>") {
        let rest = &body[start + "<version>".len()..];
        if let Some(end) = rest.find("</version>") {
            set_version(facts, &rest[..end], "pom.xml");
        }
    }
    let mut rest = text;
    while let Some(start) = rest.find("<artifactId>") {
        let after = &rest[start + "<artifactId>".len()..];
        let Some(end) = after.find("</artifactId>") else { break };
        let in_deps = text[..text.len() - rest.len() + start].rfind("<dependency>")
            > text[..text.len() - rest.len() + start].rfind("</dependency>");
        if in_deps {
            push_dep(facts, &after[..end], None, "runtime", "pom.xml");
        }
        rest = &after[end..];
    }
}

fn gradle_version(text: &str, source: &str, facts: &mut ManifestFacts) {
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version") {
            let v = rest
                .trim_start_matches(|c: char| c == ' ' || c == '=')
                .trim_matches(|c: char| c == '"' || c == '\'' || c.is_whitespace());
            if !v.is_empty() && !v.contains(' ') {
                set_version(facts, v, source);
                return;
            }
        }
    }
}

/// Well-known frameworks / libraries worth naming in the summary.
pub fn infer_stack(deps: &[Dependency]) -> Vec<String> {
    const KNOWN: &[(&str, &str)] = &[
        ("tokio", "Tokio"),
        ("axum", "Axum"),
        ("actix-web", "Actix Web"),
        ("rocket", "Rocket"),
        ("tauri", "Tauri"),
        ("sqlx", "SQLx"),
        ("diesel", "Diesel"),
        ("bevy", "Bevy"),
        ("leptos", "Leptos"),
        ("react", "React"),
        ("next", "Next.js"),
        ("vue", "Vue"),
        ("svelte", "Svelte"),
        ("@angular/core", "Angular"),
        ("vite", "Vite"),
        ("express", "Express"),
        ("@nestjs/core", "NestJS"),
        ("electron", "Electron"),
        ("typescript", "TypeScript"),
        ("tailwindcss", "Tailwind CSS"),
        ("django", "Django"),
        ("flask", "Flask"),
        ("fastapi", "FastAPI"),
        ("torch", "PyTorch"),
        ("tensorflow", "TensorFlow"),
        ("pandas", "pandas"),
        ("numpy", "NumPy"),
        ("aiogram", "aiogram"),
        ("github.com/gin-gonic/gin", "Gin"),
        ("spring-boot-starter", "Spring Boot"),
    ];
    let mut out: Vec<String> = Vec::new();
    for (needle, label) in KNOWN {
        let hit = deps.iter().any(|d| {
            let n = d.name.to_ascii_lowercase();
            n == *needle || (needle.ends_with("-starter") && n.starts_with(needle))
        });
        if hit && !out.iter().any(|o| o == label) {
            out.push((*label).to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_workspace_version_and_deps() {
        let text = r#"
[workspace.package]
version = "0.3.1"
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
[dependencies]
axum = "0.8"
[dev-dependencies]
tempfile = "3"
"#;
        let mut f = ManifestFacts::default();
        cargo(text, &mut f);
        assert_eq!(f.version.as_deref(), Some("0.3.1"));
        assert_eq!(f.dependencies.len(), 3);
        assert_eq!(infer_stack(&f.dependencies), vec!["Tokio", "Axum"]);
    }

    #[test]
    fn requirement_specs() {
        assert_eq!(split_requirement("fastapi>=0.110"), ("fastapi".into(), Some("0.110".into())));
        assert_eq!(split_requirement("uvicorn[standard]==0.29.0"), ("uvicorn".into(), Some("0.29.0".into())));
        assert_eq!(split_requirement("numpy"), ("numpy".into(), None));
    }

    #[test]
    fn go_and_pom() {
        let mut f = ManifestFacts::default();
        go_mod("module x\n\nrequire (\n  github.com/gin-gonic/gin v1.9.1\n)\nrequire golang.org/x/net v0.1.0\n", &mut f);
        assert_eq!(f.dependencies.len(), 2);
        let mut p = ManifestFacts::default();
        pom("<project><parent><version>9</version></parent><version>2.1.0</version><dependencies><dependency><artifactId>junit</artifactId></dependency></dependencies></project>", &mut p);
        assert_eq!(p.version.as_deref(), Some("2.1.0"));
        assert_eq!(p.dependencies[0].name, "junit");
    }
}
