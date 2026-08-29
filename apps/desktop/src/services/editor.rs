use std::path::Path;
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

pub async fn open_in_editor(editor: &EditorDefinition, path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }

    let args = expand_args(&editor.args, path);
    let mut cmd = Command::new(&editor.command);
    cmd.args(&args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());

    cmd.spawn()
        .map_err(|e| format!("failed to launch {}: {e}", editor.name))?;

    Ok(())
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
