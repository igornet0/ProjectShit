//! `project-hub-server` — Project Hub without the window: the same services
//! and database as the desktop app, exposed as a token-protected HTTP API for
//! BoardDo, scripts and AI agents.

mod api;
mod config;
mod selftest;
#[cfg(test)]
mod tests;

use std::io::Write;
use std::sync::Arc;

use project_hub_core::AppState;
use project_hub_database::init_db;
use tracing_subscriber::EnvFilter;

use crate::api::ApiState;
use crate::config::Config;

#[tokio::main]
async fn main() {
    let config = match Config::from_args() {
        Ok(c) => c,
        Err(msg) => {
            // --help and --version are informational, not errors.
            let info = msg.starts_with("project-hub-server ");
            if info {
                println!("{msg}");
                std::process::exit(0);
            }
            eprintln!("{msg}");
            std::process::exit(2);
        }
    };

    // Logs go to stderr; stdout carries only machine-readable lines.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,project_hub=info")),
        )
        .init();

    if config.self_test {
        let report = selftest::run().await;
        println!("{}", serde_json::to_string_pretty(&report).unwrap_or_default());
        std::process::exit(if report.ok { 0 } else { 1 });
    }

    if let Err(err) = serve(config).await {
        eprintln!("project-hub-server: {err:#}");
        std::process::exit(1);
    }
}

async fn serve(config: Config) -> anyhow::Result<()> {
    let db = init_db(&config.db_path).await?;
    let app = Arc::new(AppState::new(db));
    let listener = tokio::net::TcpListener::bind(config.listen).await?;
    let addr = listener.local_addr()?;
    if config.token.is_none() && !addr.ip().is_loopback() {
        tracing::warn!(%addr, "listening on a non-loopback address without a token");
    }
    tracing::info!(%addr, db = %config.db_path.display(), commands = config.commands.as_str(), "project-hub-server listening");

    let exit_on_eof = config.exit_on_stdin_eof;
    let router = api::router(Arc::new(ApiState { app, config }));

    // The ready line is the contract with supervisors (BoardDo reads the URL from it).
    {
        let mut out = std::io::stdout().lock();
        writeln!(
            out,
            "PROJECT_HUB_READY {}",
            serde_json::json!({ "url": format!("http://{addr}"), "version": env!("CARGO_PKG_VERSION"), "api": api::API_VERSION })
        )?;
        out.flush()?;
    }

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown(exit_on_eof))
        .await?;
    Ok(())
}

async fn shutdown(exit_on_stdin_eof: bool) {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    let stdin_closed = async move {
        if !exit_on_stdin_eof {
            std::future::pending::<()>().await;
        }
        let _ = tokio::task::spawn_blocking(|| {
            let mut sink = Vec::new();
            let _ = std::io::Read::read_to_end(&mut std::io::stdin(), &mut sink);
        })
        .await;
    };
    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut s) = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            s.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = stdin_closed => tracing::info!("stdin closed — shutting down"),
        _ = terminate => {},
    }
}
