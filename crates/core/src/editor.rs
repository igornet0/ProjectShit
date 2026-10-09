use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorDefinition {
    pub id: String,
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    pub default_editor_id: Option<String>,
    pub editors: Vec<EditorDefinition>,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            default_editor_id: Some("cursor".into()),
            editors: builtin_editors(),
        }
    }
}

pub fn builtin_editors() -> Vec<EditorDefinition> {
    vec![
        editor("vscode", "VS Code", "code", vec!["{path}"], true),
        editor("cursor", "Cursor", "cursor", vec!["{path}"], true),
        editor("zed", "Zed", "zed", vec!["{path}"], true),
        editor("idea", "IntelliJ IDEA", "idea", vec!["{path}"], true),
        editor("rustrover", "RustRover", "rustrover", vec!["{path}"], true),
        editor("sublime", "Sublime Text", "subl", vec!["{path}"], true),
        editor("vim", "Neovim", "nvim", vec!["{path}"], true),
    ]
}

fn editor(id: &str, name: &str, command: &str, args: Vec<&str>, builtin: bool) -> EditorDefinition {
    EditorDefinition {
        id: id.into(),
        name: name.into(),
        command: command.into(),
        args: args.into_iter().map(String::from).collect(),
        builtin,
    }
}

pub fn expand_args(args: &[String], path: &Path) -> Vec<String> {
    args.iter()
        .map(|a| a.replace("{path}", &path.to_string_lossy()))
        .collect()
}

/// GUI macOS apps get a minimal PATH (no /usr/local/bin). Resolve CLI binaries explicitly.
fn search_path_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join(".local/bin"));
    }
    dirs
}

fn macos_app_roots() -> Vec<PathBuf> {
    let mut roots = vec![PathBuf::from("/Applications")];
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join("Applications"));
    }
    roots
}

/// Known editor CLI paths inside macOS .app bundles (command name → path under Applications).
fn macos_bundle_cli_candidates(command: &str) -> Vec<PathBuf> {
    let bundle = match command {
        "cursor" => Some(("Cursor.app", "Contents/Resources/app/bin/cursor")),
        "code" => Some(("Visual Studio Code.app", "Contents/Resources/app/bin/code")),
        "zed" => Some(("Zed.app", "Contents/MacOS/zed")),
        "idea" => Some(("IntelliJ IDEA.app", "Contents/MacOS/idea")),
        "rustrover" => Some(("RustRover.app", "Contents/MacOS/rustrover")),
        "subl" => Some(("Sublime Text.app", "Contents/SharedSupport/bin/subl")),
        _ => None,
    };

    let Some((app, rel)) = bundle else {
        return vec![];
    };

    macos_app_roots()
        .into_iter()
        .map(|root| root.join(app).join(rel))
        .collect()
}

fn macos_app_display_name(editor_id: &str) -> Option<&'static str> {
    match editor_id {
        "cursor" => Some("Cursor"),
        "vscode" => Some("Visual Studio Code"),
        "zed" => Some("Zed"),
        "idea" => Some("IntelliJ IDEA"),
        "rustrover" => Some("RustRover"),
        "sublime" => Some("Sublime Text"),
        _ => None,
    }
}

pub fn resolve_executable(command: &str) -> Option<PathBuf> {
    let path = Path::new(command);
    if path.is_absolute() && path.is_file() {
        return Some(path.to_path_buf());
    }

    for dir in search_path_dirs() {
        let candidate = dir.join(command);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if std::env::consts::OS == "macos" {
        for candidate in macos_bundle_cli_candidates(command) {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

async fn spawn_editor_process(executable: &Path, args: &[String]) -> Result<(), String> {
    Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{e}"))
}

async fn open_macos_app(app_name: &str, path: &Path) -> Result<(), String> {
    Command::new("open")
        .arg("-a")
        .arg(app_name)
        .arg(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("{e}"))
}

pub async fn open_in_editor(editor: &EditorDefinition, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let args = expand_args(&editor.args, path);

    if let Some(executable) = resolve_executable(&editor.command) {
        spawn_editor_process(&executable, &args)
            .await
            .map_err(|e| format!("failed to launch {}: {e}", editor.name))?;
        return Ok(());
    }

    if std::env::consts::OS == "macos" {
        if let Some(app_name) = macos_app_display_name(&editor.id) {
            open_macos_app(app_name, path)
                .await
                .map_err(|e| format!("failed to launch {}: {e}", editor.name))?;
            return Ok(());
        }
    }

    Err(format!(
        "failed to launch {}: command '{}' not found (install the editor CLI or set a full path in Settings)",
        editor.name,
        editor.command
    ))
}

pub fn merge_editor_config(stored: Option<EditorConfig>) -> EditorConfig {
    let mut config = stored.unwrap_or_default();
    let builtins = builtin_editors();

    for builtin in builtins {
        if !config.editors.iter().any(|e| e.id == builtin.id) {
            config.editors.push(builtin);
        }
    }

    if config.default_editor_id.is_none() {
        config.default_editor_id = Some("cursor".into());
    }

    config
}

pub fn find_editor<'a>(config: &'a EditorConfig, id: Option<&str>) -> Option<&'a EditorDefinition> {
    let target = id.or(config.default_editor_id.as_deref())?;
    config.editors.iter().find(|e| e.id == target)
}

pub fn normalize_custom_editor(id: String, name: String, command: String, args: Vec<String>) -> EditorDefinition {
    EditorDefinition {
        id,
        name,
        command,
        args: if args.is_empty() {
            vec!["{path}".into()]
        } else {
            args
        },
        builtin: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_cursor_on_macos_if_installed() {
        if std::env::consts::OS != "macos" {
            return;
        }
        let resolved = resolve_executable("cursor");
        if Path::new("/Applications/Cursor.app").exists() {
            assert!(resolved.is_some(), "Cursor.app is installed but CLI was not resolved");
        }
    }
}
