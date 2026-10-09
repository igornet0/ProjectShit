//! Command line / environment configuration.

use std::net::SocketAddr;
use std::path::PathBuf;

/// Which project commands the API may run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPolicy {
    /// `/commands/run` is disabled.
    Off,
    /// Only commands Project Hub detected for the project (cargo test, npm run build, …).
    Detected,
    /// Any shell command inside the project folder.
    Any,
}

impl CommandPolicy {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "0" | "false" => Some(Self::Off),
            "detected" | "safe" => Some(Self::Detected),
            "any" | "all" => Some(Self::Any),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Detected => "detected",
            Self::Any => "any",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub listen: SocketAddr,
    /// Bearer token required on every route except `/api/v1/health`.
    pub token: Option<String>,
    pub db_path: PathBuf,
    pub commands: CommandPolicy,
    /// Exit when stdin closes (a parent process supervising us went away).
    pub exit_on_stdin_eof: bool,
    pub self_test: bool,
}

pub const USAGE: &str = "project-hub-server — headless Project Hub with an HTTP API

USAGE:
  project-hub-server [OPTIONS]
  project-hub-server --self-test        run an end-to-end check on a temporary database, print JSON

OPTIONS (env var in brackets):
  --listen ADDR          bind address, port 0 = random   [PROJECT_HUB_LISTEN, default 127.0.0.1:7878]
  --token TOKEN          require `Authorization: Bearer TOKEN`   [PROJECT_HUB_TOKEN]
  --db PATH              SQLite database (shared with the desktop app by default)   [PROJECT_HUB_DB]
  --commands POLICY      off | detected | any — which project commands may run   [PROJECT_HUB_COMMANDS, default off]
  --exit-on-stdin-eof    stop when stdin closes (for supervisors)
  -h, --help

On start the server prints one line `PROJECT_HUB_READY {\"url\":…}` to stdout.";

impl Config {
    pub fn from_args() -> Result<Self, String> {
        let env = |k: &str| std::env::var(k).ok().filter(|v| !v.trim().is_empty());
        let mut listen = env("PROJECT_HUB_LISTEN").unwrap_or_else(|| "127.0.0.1:7878".into());
        let mut token = env("PROJECT_HUB_TOKEN");
        let mut db = env("PROJECT_HUB_DB").map(PathBuf::from);
        let mut commands = env("PROJECT_HUB_COMMANDS")
            .map(|v| CommandPolicy::parse(&v).ok_or(format!("bad PROJECT_HUB_COMMANDS `{v}`")))
            .transpose()?
            .unwrap_or(CommandPolicy::Off);
        let mut exit_on_stdin_eof = false;
        let mut self_test = false;

        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            let mut value = |name: &str| args.next().ok_or(format!("{name} needs a value"));
            match arg.as_str() {
                "--listen" => listen = value("--listen")?,
                "--token" => token = Some(value("--token")?),
                "--db" => db = Some(PathBuf::from(value("--db")?)),
                "--commands" => {
                    let v = value("--commands")?;
                    commands = CommandPolicy::parse(&v).ok_or(format!("bad --commands `{v}`"))?;
                }
                "--exit-on-stdin-eof" => exit_on_stdin_eof = true,
                "--self-test" => self_test = true,
                "-h" | "--help" => return Err(USAGE.to_string()),
                other => return Err(format!("unknown argument `{other}`\n\n{USAGE}")),
            }
        }

        Ok(Self {
            listen: listen.parse().map_err(|e| format!("bad listen address `{listen}`: {e}"))?,
            token: token.filter(|t| !t.trim().is_empty()),
            db_path: db.unwrap_or_else(default_db_path),
            commands,
            exit_on_stdin_eof,
            self_test,
        })
    }
}

/// The desktop app's database (`app_data_dir` of `com.projecthub.desktop`), so
/// the server and the app see the same projects and tasks.
pub fn default_db_path() -> PathBuf {
    const APP_ID: &str = "com.projecthub.desktop";
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let base = if cfg!(target_os = "macos") {
        home.map(|h| h.join("Library/Application Support"))
    } else if cfg!(target_os = "windows") {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
        .join("project-hub.db")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policies() {
        assert_eq!(CommandPolicy::parse("Detected"), Some(CommandPolicy::Detected));
        assert_eq!(CommandPolicy::parse("nope"), None);
        assert!(default_db_path().ends_with("com.projecthub.desktop/project-hub.db"));
    }
}
