//! Bounded walk of the project tree: file counts, lines of code, layout.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

/// Directories never descended into.
pub const SKIP_DIRS: &[&str] = &[
    ".git",
    ".brdd",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    ".next",
    ".nuxt",
    ".turbo",
    ".venv",
    "venv",
    "__pycache__",
    ".mypy_cache",
    ".pytest_cache",
    ".gradle",
    ".idea",
    ".vscode",
    "coverage",
    "bundles",
    "vendor",
];

const MAX_LOC_FILE_BYTES: u64 = 512 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageStat {
    pub language: String,
    pub files: usize,
    pub loc: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DirStat {
    pub name: String,
    pub files: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Structure {
    pub files: usize,
    pub dirs: usize,
    pub source_files: usize,
    /// Lines in source files (not docs / config).
    pub loc: usize,
    pub languages: Vec<LanguageStat>,
    pub top_dirs: Vec<DirStat>,
    pub has_tests: bool,
    pub has_docker: bool,
    pub ci: Vec<String>,
    /// The walk stopped at the file limit.
    pub truncated: bool,
}

/// Source language for an extension; `None` for docs, config and binaries.
pub fn language_for(ext: &str) -> Option<&'static str> {
    Some(match ext {
        "rs" => "Rust",
        "ts" | "tsx" | "mts" | "cts" => "TypeScript",
        "js" | "jsx" | "mjs" | "cjs" => "JavaScript",
        "py" | "pyi" => "Python",
        "go" => "Go",
        "java" => "Java",
        "kt" | "kts" => "Kotlin",
        "swift" => "Swift",
        "c" | "h" => "C",
        "cpp" | "cc" | "cxx" | "hpp" | "hh" => "C++",
        "cs" => "C#",
        "rb" => "Ruby",
        "php" => "PHP",
        "dart" => "Dart",
        "lua" => "Lua",
        "sh" | "bash" | "zsh" => "Shell",
        "sql" => "SQL",
        "vue" => "Vue",
        "svelte" => "Svelte",
        "css" | "scss" | "sass" | "less" => "CSS",
        "html" | "htm" => "HTML",
        _ => return None,
    })
}

pub fn walk(root: &Path, max_files: usize) -> Structure {
    let mut s = Structure::default();
    let mut langs: BTreeMap<&'static str, (usize, usize)> = BTreeMap::new();
    let mut top: BTreeMap<String, usize> = BTreeMap::new();

    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            e.depth() == 0
                || !(e.file_type().is_dir()
                    && e.file_name().to_str().is_some_and(|n| SKIP_DIRS.contains(&n)))
        })
        .filter_map(Result::ok);

    for entry in walker {
        if entry.depth() == 0 {
            continue;
        }
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        if entry.file_type().is_dir() {
            s.dirs += 1;
            let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
            if matches!(name.as_str(), "tests" | "test" | "__tests__" | "spec") {
                s.has_tests = true;
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        if s.files >= max_files {
            s.truncated = true;
            break;
        }
        s.files += 1;

        if let Some(first) = rel.components().next() {
            if rel.components().count() > 1 {
                *top.entry(first.as_os_str().to_string_lossy().to_string()).or_default() += 1;
            }
        }

        let file_name = entry.file_name().to_string_lossy().to_ascii_lowercase();
        if file_name.starts_with("dockerfile") || file_name.starts_with("docker-compose") || file_name == "compose.yaml" {
            s.has_docker = true;
        }
        if rel_str.starts_with(".github/workflows/") || rel_str == ".gitlab-ci.yml" {
            s.ci.push(rel_str.clone());
        }
        if file_name.contains("_test.") || file_name.contains(".test.") || file_name.contains(".spec.") || file_name.starts_with("test_") {
            s.has_tests = true;
        }

        let ext = entry
            .path()
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_ascii_lowercase())
            .unwrap_or_default();
        let Some(lang) = language_for(&ext) else { continue };
        s.source_files += 1;
        let lines = entry
            .metadata()
            .ok()
            .filter(|m| m.len() <= MAX_LOC_FILE_BYTES)
            .and_then(|_| std::fs::read(entry.path()).ok())
            .map(|bytes| bytes.iter().filter(|b| **b == b'\n').count())
            .unwrap_or(0);
        s.loc += lines;
        let slot = langs.entry(lang).or_default();
        slot.0 += 1;
        slot.1 += lines;
    }

    let mut languages: Vec<LanguageStat> = langs
        .into_iter()
        .map(|(language, (files, loc))| LanguageStat {
            language: language.to_string(),
            files,
            loc,
        })
        .collect();
    languages.sort_by(|a, b| b.loc.cmp(&a.loc).then(b.files.cmp(&a.files)));
    s.languages = languages;

    let mut top_dirs: Vec<DirStat> = top
        .into_iter()
        .map(|(name, files)| DirStat { name, files })
        .collect();
    top_dirs.sort_by(|a, b| b.files.cmp(&a.files).then(a.name.cmp(&b.name)));
    top_dirs.truncate(12);
    s.top_dirs = top_dirs;
    s.ci.sort();
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_sources_and_skips_build_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join("target/debug")).unwrap();
        std::fs::create_dir_all(root.join(".github/workflows")).unwrap();
        std::fs::create_dir_all(root.join("tests")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {\n}\n").unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();
        std::fs::write(root.join("target/debug/x.rs"), "ignored\n").unwrap();
        std::fs::write(root.join("README.md"), "# hi\n").unwrap();
        std::fs::write(root.join(".github/workflows/ci.yml"), "on: push\n").unwrap();
        std::fs::write(root.join("Dockerfile"), "FROM rust\n").unwrap();

        let s = walk(root, 1000);
        assert_eq!(s.source_files, 2);
        assert_eq!(s.loc, 3);
        assert_eq!(s.languages[0].language, "Rust");
        assert!(s.has_tests && s.has_docker);
        assert_eq!(s.ci, vec![".github/workflows/ci.yml"]);
        assert_eq!(s.top_dirs[0].name, "src");
        assert!(!s.truncated);
        assert!(walk(root, 2).truncated);
    }
}
