//! Read-only access to a project's source: tree, file contents, text search.
//! Every path is resolved inside the project root (no `..`, no symlink escape).

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::ServiceError;

const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    ".next",
    ".venv",
    "venv",
    "__pycache__",
    ".gradle",
    ".idea",
    "coverage",
];
pub const DEFAULT_MAX_FILE_BYTES: u64 = 256 * 1024;
const HARD_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_TREE_ENTRIES: usize = 2_000;
const MAX_SEARCH_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeEntry {
    /// Relative to the project root, `/`-separated.
    pub path: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeResult {
    pub root: String,
    pub entries: Vec<TreeEntry>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileContent {
    pub path: String,
    pub size: u64,
    pub truncated: bool,
    pub binary: bool,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub query: String,
    pub hits: Vec<SearchHit>,
    pub files_scanned: usize,
    pub truncated: bool,
}

pub struct CodeService;

impl CodeService {
    /// Join `rel` onto `root`, refusing anything that leaves the root.
    pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, ServiceError> {
        let rel = rel.trim().trim_start_matches(['/', '\\']);
        let mut out = root.to_path_buf();
        for comp in Path::new(rel).components() {
            match comp {
                Component::Normal(part) => out.push(part),
                Component::CurDir => {}
                _ => return Err(ServiceError::InvalidPath(format!("path escapes the project: {rel}"))),
            }
        }
        let canon_root = root.canonicalize().map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        if let Ok(canon) = out.canonicalize() {
            if !canon.starts_with(&canon_root) {
                return Err(ServiceError::InvalidPath(format!("path escapes the project: {rel}")));
            }
        }
        Ok(out)
    }

    pub fn tree(root: &Path, rel: &str, depth: usize) -> Result<TreeResult, ServiceError> {
        let base = Self::resolve(root, rel)?;
        if !base.is_dir() {
            return Err(ServiceError::NotFound(format!("directory {rel}")));
        }
        let mut entries = Vec::new();
        let mut truncated = false;
        let walker = WalkDir::new(&base)
            .follow_links(false)
            .max_depth(depth.clamp(1, 8))
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                e.depth() == 0
                    || !(e.file_type().is_dir()
                        && e.file_name().to_str().is_some_and(|n| SKIP_DIRS.contains(&n)))
            })
            .filter_map(Result::ok);
        for entry in walker.filter(|e| e.depth() > 0) {
            if entries.len() >= MAX_TREE_ENTRIES {
                truncated = true;
                break;
            }
            let path = relative(root, entry.path());
            let is_dir = entry.file_type().is_dir();
            entries.push(TreeEntry {
                path,
                kind: if is_dir { "dir" } else { "file" }.into(),
                size: if is_dir { None } else { entry.metadata().ok().map(|m| m.len()) },
            });
        }
        Ok(TreeResult {
            root: relative(root, &base),
            entries,
            truncated,
        })
    }

    pub fn read_file(root: &Path, rel: &str, max_bytes: Option<u64>) -> Result<FileContent, ServiceError> {
        let path = Self::resolve(root, rel)?;
        let meta = std::fs::metadata(&path).map_err(|_| ServiceError::NotFound(format!("file {rel}")))?;
        if !meta.is_file() {
            return Err(ServiceError::NotFound(format!("file {rel}")));
        }
        let limit = max_bytes.unwrap_or(DEFAULT_MAX_FILE_BYTES).clamp(1, HARD_MAX_FILE_BYTES);
        let bytes = read_prefix(&path, limit).map_err(|e| ServiceError::InvalidPath(e.to_string()))?;
        let binary = bytes.iter().take(8_000).any(|b| *b == 0);
        Ok(FileContent {
            path: relative(root, &path),
            size: meta.len(),
            truncated: meta.len() > limit,
            binary,
            content: if binary {
                String::new()
            } else {
                String::from_utf8_lossy(&bytes).to_string()
            },
        })
    }

    /// Case-insensitive substring search over text files.
    pub fn search(root: &Path, query: &str, limit: usize) -> Result<SearchResult, ServiceError> {
        let needle = query.trim().to_lowercase();
        if needle.len() < 2 {
            return Err(ServiceError::InvalidPath("query must be at least 2 characters".into()));
        }
        let limit = limit.clamp(1, 500);
        let mut hits = Vec::new();
        let mut files_scanned = 0;
        let mut truncated = false;
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                e.depth() == 0
                    || !(e.file_type().is_dir()
                        && e.file_name()
                            .to_str()
                            .is_some_and(|n| SKIP_DIRS.contains(&n) || n == ".brdd"))
            })
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file());
        'files: for entry in walker {
            if entry.metadata().map(|m| m.len() > MAX_SEARCH_FILE_BYTES).unwrap_or(true) {
                continue;
            }
            let Ok(bytes) = std::fs::read(entry.path()) else { continue };
            if bytes.iter().take(8_000).any(|b| *b == 0) {
                continue;
            }
            files_scanned += 1;
            let text = String::from_utf8_lossy(&bytes);
            for (idx, line) in text.lines().enumerate() {
                if line.to_lowercase().contains(&needle) {
                    if hits.len() >= limit {
                        truncated = true;
                        break 'files;
                    }
                    hits.push(SearchHit {
                        path: relative(root, entry.path()),
                        line: idx + 1,
                        text: line.trim().chars().take(240).collect(),
                    });
                }
            }
        }
        Ok(SearchResult {
            query: query.to_string(),
            hits,
            files_scanned,
            truncated,
        })
    }
}

fn read_prefix(path: &Path, limit: u64) -> std::io::Result<Vec<u8>> {
    use std::io::Read;
    let mut buf = Vec::new();
    std::fs::File::open(path)?.take(limit).read_to_end(&mut buf)?;
    Ok(buf)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_read_search_and_escape_guard() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src/nested")).unwrap();
        std::fs::create_dir_all(root.join("node_modules/x")).unwrap();
        std::fs::write(root.join("src/main.rs"), "fn main() {\n    // TODO: wire login\n}\n").unwrap();
        std::fs::write(root.join("src/nested/a.rs"), "pub fn login() {}\n").unwrap();
        std::fs::write(root.join("node_modules/x/login.js"), "login()").unwrap();
        std::fs::write(root.join("blob.bin"), [0u8, 1, 2, 3]).unwrap();

        let tree = CodeService::tree(root, "", 3).unwrap();
        let paths: Vec<&str> = tree.entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"src/nested/a.rs"));
        assert!(!paths.iter().any(|p| p.starts_with("node_modules/")));

        let file = CodeService::read_file(root, "src/main.rs", Some(10)).unwrap();
        assert!(file.truncated);
        assert_eq!(file.content, "fn main() ");
        assert!(CodeService::read_file(root, "blob.bin", None).unwrap().binary);

        let found = CodeService::search(root, "LOGIN", 10).unwrap();
        assert_eq!(found.hits.len(), 2);
        assert!(found.hits.iter().all(|h| h.path.starts_with("src/")));

        assert!(CodeService::read_file(root, "../etc/passwd", None).is_err());
        assert!(CodeService::tree(root, "/../..", 1).is_err());
    }
}
