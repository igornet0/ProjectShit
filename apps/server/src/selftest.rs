//! `--self-test`: start the real API on a temporary database and a sample
//! project, exercise it over HTTP and report every step. Supervisors (BoardDo)
//! run it after building and before enabling the integration.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use project_hub_core::AppState;
use project_hub_database::init_db;
use reqwest::{Method, StatusCode};
use serde::Serialize;
use serde_json::{json, Value};

use crate::api::{self, ApiState};
use crate::config::{CommandPolicy, Config};

#[derive(Debug, Serialize)]
pub struct Step {
    pub name: String,
    pub ok: bool,
    pub ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub ok: bool,
    pub version: &'static str,
    pub passed: usize,
    pub failed: usize,
    pub duration_ms: u64,
    pub steps: Vec<Step>,
}

#[derive(Clone)]
struct Api {
    http: reqwest::Client,
    base: String,
    token: String,
}

impl Api {
    async fn call(&self, method: Method, path: &str, body: Option<Value>, auth: bool) -> Result<(StatusCode, Value), String> {
        let mut req = self.http.request(method, format!("{}{path}", self.base));
        if auth {
            req = req.bearer_auth(&self.token);
        }
        if let Some(b) = body {
            req = req.json(&b);
        }
        let resp = req.send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().await.map_err(|e| e.to_string())?;
        let value = if text.is_empty() {
            Value::Null
        } else {
            serde_json::from_str(&text).unwrap_or(Value::String(text))
        };
        Ok((status, value))
    }

}

async fn record<Fut>(steps: &mut Vec<Step>, name: &str, fut: Fut) -> Option<Value>
where
    Fut: std::future::Future<Output = Result<Value, String>>,
{
    let started = Instant::now();
    let result = fut.await;
    let ms = started.elapsed().as_millis() as u64;
    let (ok, detail, value) = match result {
        Ok(v) => (true, None, Some(v)),
        Err(e) => (false, Some(e), None),
    };
    steps.push(Step {
        name: name.into(),
        ok,
        ms,
        detail,
    });
    value
}

/// `step!(steps, api, "name", |c| { ... })` — `c` is an owned API client.
macro_rules! step {
    ($steps:expr, $api:expr, $name:expr, |$c:ident| $body:block) => {{
        let $c = $api.clone();
        record(&mut $steps, $name, async move $body).await
    }};
}

fn expect(cond: bool, msg: impl Into<String>) -> Result<(), String> {
    if cond {
        Ok(())
    } else {
        Err(msg.into())
    }
}

fn write_sample(root: &Path) -> std::io::Result<()> {
    let project = root.join("selftest-app");
    std::fs::create_dir_all(project.join("src"))?;
    std::fs::write(
        project.join("Cargo.toml"),
        "[package]\nname = \"selftest-app\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nserde = \"1\"\n",
    )?;
    std::fs::write(project.join("src/main.rs"), "fn main() {\n    println!(\"hello from self-test\");\n}\n")?;
    std::fs::write(project.join("README.md"), "# Self-test app\n\nSample project created by project-hub-server --self-test.\n")?;
    Ok(())
}

pub async fn run() -> Report {
    let started = Instant::now();
    let mut steps = Vec::new();
    let tmp = match tempfile::tempdir() {
        Ok(t) => t,
        Err(e) => return failed_report(started, "temp dir", e.to_string()),
    };
    let projects_dir = tmp.path().join("projects");
    if let Err(e) = write_sample(&projects_dir) {
        return failed_report(started, "sample project", e.to_string());
    }
    let sample = projects_dir.join("selftest-app");
    // A git repo when git is available (exercises status / snapshots).
    let has_git = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&sample)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        && std::process::Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=selftest", "add", "."])
            .current_dir(&sample)
            .status()
            .is_ok()
        && std::process::Command::new("git")
            .args(["-c", "user.email=t@t", "-c", "user.name=selftest", "commit", "-q", "-m", "init"])
            .current_dir(&sample)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

    let db = match init_db(&tmp.path().join("selftest.db")).await {
        Ok(db) => db,
        Err(e) => return failed_report(started, "database", e.to_string()),
    };
    let token = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    let config = Config {
        listen: "127.0.0.1:0".parse().expect("static addr"),
        token: Some(token.clone()),
        db_path: tmp.path().join("selftest.db"),
        commands: CommandPolicy::Off,
        exit_on_stdin_eof: false,
        self_test: true,
    };
    let listener = match tokio::net::TcpListener::bind(config.listen).await {
        Ok(l) => l,
        Err(e) => return failed_report(started, "bind", e.to_string()),
    };
    let addr = listener.local_addr().expect("bound");
    let router = api::router(Arc::new(ApiState {
        app: Arc::new(AppState::new(db)),
        config,
    }));
    let server = tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });

    let api = Api {
        http: reqwest::Client::new(),
        base: format!("http://{addr}/api/v1"),
        token,
    };

    step!(steps, api, "health", |c| {
        let (s, v) = c.call(Method::GET, "/health", None, false).await?;
        expect(s.is_success() && v["status"] == "ok" && v["auth"] == true, format!("{s} {v}"))?;
        Ok(v)
    });

    step!(steps, api, "auth: request without token is rejected", |c| {
        let (s, v) = c.call(Method::GET, "/projects", None, false).await?;
        expect(s == StatusCode::UNAUTHORIZED, format!("expected 401, got {s} {v}"))?;
        Ok(v)
    });

    let dir = projects_dir.display().to_string();
    step!(steps, api, "add project root + scan", |c| {
        let (s, v) = c.call(Method::POST, "/roots", Some(json!({ "path": dir })), true).await?;
        let n = v["projects"].as_array().map(|a| a.len()).unwrap_or(0);
        expect(s.is_success() && n == 1, format!("{s}: {n} project(s) — {v}"))?;
        Ok(v)
    });

    let listed = step!(steps, api, "list + search projects", |c| {
            let (s, v) = c.call(Method::GET, "/projects?q=selftest", None, true).await?;
            expect(s.is_success() && v["total"] == 1, format!("{s} {v}"))?;
            Ok(v)
        });
    let pid = listed
        .as_ref()
        .and_then(|v| v["items"][0]["id"].as_str())
        .map(str::to_string)
        .unwrap_or_default();

    let p = pid.clone();
    step!(steps, api, "project detail (commands, packages, .brdd digest)", |c| {
        let (s, v) = c.call(Method::GET, &format!("/projects/{p}/detail"), None, true).await?;
        let commands = v["commands"].as_array().map(|a| a.len()).unwrap_or(0);
        expect(s.is_success() && commands > 0 && v["brdd"]["exists"] == false, format!("{s} commands={commands} {}", v["brdd"]))?;
        Ok(v)
    });

    let p = pid.clone();
    step!(steps, api, "code: tree / file / search", |c| {
        let (s, tree) = c.call(Method::GET, &format!("/projects/{p}/tree?depth=3"), None, true).await?;
        let has_main = tree["entries"]
            .as_array()
            .is_some_and(|e| e.iter().any(|x| x["path"] == "src/main.rs"));
        expect(s.is_success() && has_main, format!("tree {s} {tree}"))?;
        let (s, file) = c.call(Method::GET, &format!("/projects/{p}/file?path=src/main.rs"), None, true).await?;
        expect(s.is_success() && file["content"].as_str().is_some_and(|t| t.contains("fn main")), format!("file {s} {file}"))?;
        let (s, esc) = c.call(Method::GET, &format!("/projects/{p}/file?path=../../etc/passwd"), None, true).await?;
        expect(s == StatusCode::BAD_REQUEST, format!("path escape must fail, got {s} {esc}"))?;
        let (s, found) = c.call(Method::GET, &format!("/projects/{p}/search?q=self-test"), None, true).await?;
        expect(s.is_success() && found["hits"].as_array().is_some_and(|h| !h.is_empty()), format!("search {s} {found}"))?;
        Ok(found)
    });

    let p = pid.clone();
    let sample_dir = sample.clone();
    step!(steps, api, ".brdd: refresh writes analysis, versions, changelog", |c| {
        let (s, v) = c.call(Method::POST, &format!("/projects/{p}/brdd/refresh"), Some(json!({})), true).await?;
        expect(s.is_success() && v["snapshot"]["n"] == 1, format!("{s} {v}"))?;
        expect(v["analysis"]["version"] == "0.1.0", format!("version {}", v["analysis"]["version"]))?;
        for f in ["project.json", "summary.md", "versions.json", "CHANGELOG.md", "tasks.json"] {
            expect(sample_dir.join(".brdd").join(f).is_file(), format!("missing .brdd/{f}"))?;
        }
        let (s, notes) = c
            .call(Method::PUT, &format!("/projects/{p}/brdd/notes"), Some(json!({ "markdown": "AI: ok" })), true)
            .await?;
        expect(s.is_success() && notes["notes_md"] == "AI: ok", format!("notes {s} {notes}"))?;
        Ok(v)
    });

    if has_git {
        let p = pid.clone();
        step!(steps, api, "git status (+ .brdd keeps the tree clean)", |c| {
            let (s, v) = c.call(Method::GET, &format!("/projects/{p}/git"), None, true).await?;
            let st = &v["status"];
            expect(s.is_success() && st["untracked"] == 0 && st["modified"] == 0, format!("{s} {st}"))?;
            Ok(v)
        });
    }

    let p = pid.clone();
    let created = step!(steps, api, "tasks: create with an external link", |c| {
            let body = json!({
                "project_id": p,
                "title": "Self-test task",
                "priority": "high",
                "source": "selftest",
                "external_ref": "selftest:1",
            });
            let (s, v) = c.call(Method::POST, "/tasks", Some(body), true).await?;
            expect(
                s == StatusCode::CREATED && v["link"]["source"] == "selftest" && v["status"] == "todo",
                format!("{s} {v}"),
            )?;
            Ok(v)
        });
    let tid = created
        .as_ref()
        .and_then(|v| v["id"].as_str())
        .map(str::to_string)
        .unwrap_or_default();

    let t = tid.clone();
    step!(steps, api, "tasks: update status and track by source", |c| {
        let (s, v) = c.call(Method::PATCH, &format!("/tasks/{t}"), Some(json!({ "status": "done" })), true).await?;
        expect(s.is_success() && v["status"] == "done", format!("patch {s} {v}"))?;
        let (s, v) = c.call(Method::GET, &format!("/tasks?source=selftest&status=done&ids={t}"), None, true).await?;
        expect(s.is_success() && v.as_array().map(|a| a.len()) == Some(1), format!("list {s} {v}"))?;
        Ok(v)
    });

    step!(steps, api, "activity feed", |c| {
        let (s, v) = c.call(Method::GET, "/activity?limit=20", None, true).await?;
        let kinds: Vec<String> = v
            .as_array()
            .map(|a| a.iter().filter_map(|x| x["activity_type"].as_str().map(str::to_string)).collect())
            .unwrap_or_default();
        expect(
            s.is_success() && kinds.iter().any(|k| k.contains("task_created")) && kinds.iter().any(|k| k.contains("task_completed")),
            format!("{s} {kinds:?}"),
        )?;
        Ok(v)
    });

    let p = pid.clone();
    step!(steps, api, "commands are refused by default policy", |c| {
        let (s, v) = c
            .call(Method::POST, &format!("/projects/{p}/commands/run"), Some(json!({ "command": "echo hi" })), true)
            .await?;
        expect(s == StatusCode::FORBIDDEN && v["code"] == "commands_disabled", format!("{s} {v}"))?;
        Ok(v)
    });

    let t = tid.clone();
    step!(steps, api, "tasks: delete", |c| {
        let (s, v) = c.call(Method::DELETE, &format!("/tasks/{t}"), None, true).await?;
        expect(s == StatusCode::NO_CONTENT, format!("delete {s} {v}"))?;
        let (s, v) = c.call(Method::GET, &format!("/tasks/{t}"), None, true).await?;
        expect(s == StatusCode::NOT_FOUND, format!("get after delete {s} {v}"))?;
        Ok(Value::Null)
    });

    server.abort();
    finish(started, steps)
}

fn failed_report(started: Instant, name: &str, err: String) -> Report {
    finish(
        started,
        vec![Step {
            name: name.into(),
            ok: false,
            ms: 0,
            detail: Some(err),
        }],
    )
}

fn finish(started: Instant, steps: Vec<Step>) -> Report {
    let failed = steps.iter().filter(|s| !s.ok).count();
    Report {
        ok: failed == 0 && !steps.is_empty(),
        version: env!("CARGO_PKG_VERSION"),
        passed: steps.len() - failed,
        failed,
        duration_ms: started.elapsed().as_millis() as u64,
        steps,
    }
}
