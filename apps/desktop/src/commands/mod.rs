use std::sync::Arc;

use crate::services::AppState;
use tokio::sync::Mutex;

mod editors;
mod folders;
mod git;
mod github_cmd;
mod projects;
mod tasks;

pub use editors::*;
pub use folders::*;
pub use git::*;
pub use github_cmd::*;
pub use projects::*;
pub use tasks::*;

pub type SharedState = Arc<Mutex<AppState>>;
