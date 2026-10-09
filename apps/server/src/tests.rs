//! API tests against a mock GitHub.

use std::sync::{Arc, Mutex};

use axum::extract::Path;
use axum::routing::{get, post};
use axum::{Json, Router};
use project_hub_core::AppState;
use project_hub_database::init_db;
use serde_json::{json, Value};

use crate::api::{self, ApiState};
use crate::config::{CommandPolicy, Config};

#[derive(Default)]
struct FakeGitHub {
    state: String,
    closed_by_api: usize,
}

fn issue(number: i64, state: &str) -> Value {
    json!({
        "number": number,
        "title": "Ship it",
        "state": state,
        "html_url": format!("https://github.com/octo/demo/issues/{number}"),
        "body": null,
        "labels": [{ "name": "agent" }],
        "updated_at": "2026-10-09T10:00:00Z"
    })
}

async fn mock_github() -> (String, Arc<Mutex<FakeGitHub>>) {
    let gh = Arc::new(Mutex::new(FakeGitHub {
        state: "open".into(),
        ..Default::default()
    }));
    let (g1, g2, g3) = (gh.clone(), gh.clone(), gh.clone());
    let app = Router::new()
        .route("/user", get(|| async { Json(json!({ "login": "octocat" })) }))
        .route(
            "/user/repos",
            get(|| async {
                Json(json!([{
                    "id": 42, "full_name": "octo/demo", "clone_url": "https://github.com/octo/demo.git",
                    "ssh_url": null, "html_url": "https://github.com/octo/demo", "description": null,
                    "private": false, "default_branch": "main", "updated_at": "2026-10-01T00:00:00Z"
                }]))
            }),
        )
        .route(
            "/repos/octo/demo/issues",
            post(move |Json(body): Json<Value>| {
                let g = g1.clone();
                async move {
                    assert_eq!(body["title"], "Ship it");
                    assert_eq!(body["labels"][0], "agent");
                    g.lock().unwrap().state = "open".into();
                    Json(issue(5, "open"))
                }
            }),
        )
        .route(
            "/repos/octo/demo/issues/{n}",
            get(move |Path(n): Path<i64>| {
                let g = g2.clone();
                async move { Json(issue(n, &g.lock().unwrap().state)) }
            })
            .patch(move |Path(n): Path<i64>, Json(body): Json<Value>| {
                let g = g3.clone();
                async move {
                    let mut g = g.lock().unwrap();
                    g.state = body["state"].as_str().unwrap_or("open").to_string();
                    g.closed_by_api += 1;
                    Json(issue(n, &g.state))
                }
            }),
        );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), gh)
}

async fn serve(db_dir: &std::path::Path) -> String {
    let db = init_db(&db_dir.join("api.db")).await.unwrap();
    let config = Config {
        listen: "127.0.0.1:0".parse().unwrap(),
        token: Some("t0k".into()),
        db_path: db_dir.join("api.db"),
        commands: CommandPolicy::Detected,
        exit_on_stdin_eof: false,
        self_test: false,
    };
    let listener = tokio::net::TcpListener::bind(config.listen).await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = api::router(Arc::new(ApiState {
        app: Arc::new(AppState::new(db)),
        config,
    }));
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    format!("http://{addr}/api/v1")
}

async fn call(method: reqwest::Method, url: String, body: Option<Value>) -> (u16, Value) {
    let mut req = reqwest::Client::new().request(method, url).bearer_auth("t0k");
    if let Some(b) = body {
        req = req.json(&b);
    }
    let resp = req.send().await.unwrap();
    let status = resp.status().as_u16();
    let text = resp.text().await.unwrap();
    (status, serde_json::from_str(&text).unwrap_or(Value::Null))
}

fn git(dir: &std::path::Path, args: &[&str]) {
    let ok = std::process::Command::new("git")
        .args(["-c", "user.email=t@t", "-c", "user.name=t"])
        .args(args)
        .current_dir(dir)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?}");
}

#[tokio::test]
async fn tasks_sync_with_github_issues_both_ways() {
    let (gh_base, gh) = mock_github().await;
    // Process-wide; only this test talks to GitHub.
    std::env::set_var("PROJECT_HUB_GITHUB_API", &gh_base);

    let tmp = tempfile::tempdir().unwrap();
    let repo = tmp.path().join("projects/demo");
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::write(repo.join("Cargo.toml"), "[package]\nname = \"demo\"\nversion = \"0.1.0\"\n").unwrap();
    std::fs::write(repo.join("src/main.rs"), "fn main() {}\n").unwrap();
    git(&repo, &["init", "-q"]);
    git(&repo, &["remote", "add", "origin", "https://github.com/octo/demo.git"]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "init"]);

    let base = serve(tmp.path()).await;
    use reqwest::Method;

    let (s, v) = call(Method::POST, format!("{base}/github/connect"), Some(json!({ "token": "gh-test" }))).await;
    assert_eq!(s, 200, "{v}");
    assert_eq!(v["username"], "octocat");
    assert_eq!(v["connected"], true);

    let (s, v) = call(Method::POST, format!("{base}/roots"), Some(json!({ "path": tmp.path().join("projects") }))).await;
    assert_eq!(s, 200, "{v}");
    let pid = v["projects"][0]["id"].as_str().unwrap().to_string();
    let (_, detail) = call(Method::GET, format!("{base}/projects/{pid}/detail"), None).await;
    assert_eq!(detail["github_repo"], "octo/demo");

    let (s, task) = call(
        Method::POST,
        format!("{base}/tasks"),
        Some(json!({ "project_id": pid, "title": "Ship it", "source": "boarddo", "external_ref": "objective:x/i1" })),
    )
    .await;
    assert_eq!(s, 201, "{task}");
    let tid = task["id"].as_str().unwrap().to_string();

    let (s, linked) = call(Method::POST, format!("{base}/tasks/{tid}/github-issue"), Some(json!({ "labels": ["agent"] }))).await;
    assert_eq!(s, 200, "{linked}");
    assert_eq!(linked["link"]["github_issue_number"], 5);
    assert_eq!(linked["link"]["source"], "boarddo", "link keeps its source");

    // Closed on GitHub → task done.
    gh.lock().unwrap().state = "closed".into();
    let (_, report) = call(Method::POST, format!("{base}/github/issues/sync"), None).await;
    assert_eq!(report["tasks_completed"], 1, "{report}");
    let (_, t) = call(Method::GET, format!("{base}/tasks/{tid}"), None).await;
    assert_eq!(t["status"], "done");

    // Reopened on GitHub → task back to todo.
    gh.lock().unwrap().state = "open".into();
    let (_, report) = call(Method::POST, format!("{base}/github/issues/sync"), None).await;
    assert_eq!(report["tasks_reopened"], 1, "{report}");

    // Done in Project Hub → issue gets closed.
    let (_, t) = call(Method::PATCH, format!("{base}/tasks/{tid}"), Some(json!({ "status": "done" }))).await;
    assert_eq!(t["status"], "done");
    let (_, report) = call(Method::POST, format!("{base}/github/issues/sync"), None).await;
    assert_eq!(report["issues_closed"], 1, "{report}");
    assert_eq!(gh.lock().unwrap().state, "closed");
    assert_eq!(gh.lock().unwrap().closed_by_api, 1);

    // Detected-commands policy: arbitrary shell is refused, detected ones run.
    let (s, v) = call(Method::POST, format!("{base}/projects/{pid}/commands/run"), Some(json!({ "command": "rm -rf ." }))).await;
    assert_eq!(s, 403, "{v}");
    assert_eq!(v["code"], "command_not_allowed");
    assert!(repo.join("src/main.rs").exists());

    // Unknown ids → typed 404 / 400 errors.
    let (s, v) = call(Method::GET, format!("{base}/tasks/{}", uuid::Uuid::new_v4()), None).await;
    assert_eq!((s, v["code"].as_str()), (404, Some("not_found")));
    let (s, _) = call(Method::GET, format!("{base}/projects/not-a-uuid"), None).await;
    assert_eq!(s, 400);
}
