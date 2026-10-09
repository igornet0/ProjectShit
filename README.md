# Project Hub

Local-first desktop app that discovers projects on your machine, reads their structure, Git state, and tooling — and gives you a gallery, tasks, calendar, activity feed, and command execution on top.

Project files stay on disk. SQLite stores metadata only (`root_path`, tasks, settings — not file contents).

## Branches

| Branch | Purpose |
|--------|---------|
| **`main`** | Published source — application code and schema only |
| **`dev`** | Active development; may include extra tooling and experiments ahead of `main` |

## Architecture

The codebase is split into layers. Dependencies point inward: UI and the desktop shell call services; services call domain types and infrastructure crates.

```
┌─────────────────────────────────────────────────────────────┐
│  Presentation          frontend/  (React + TypeScript)     │
│                        invoke() → Tauri IPC                 │
└──────────────────────────────┬──────────────────────────────┘
                               │
┌──────────────────────────────▼──────────────────────────────┐
│  Application           apps/desktop/                        │
│                        commands/   thin IPC handlers          │
│                        services/   orchestration & use cases  │
└──────────────────────────────┬──────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
┌───────▼────────┐   ┌─────────▼────────┐   ┌────────▼────────┐
│  Domain        │   │  Infrastructure  │   │  Persistence    │
│  crates/domain │   │  discovery       │   │  migrations/    │
│  types, rules  │   │  git, github     │   │  SQL schema     │
│                │   │  executor        │   │                 │
│                │   │  watcher (stub)  │   │  database crate │
│                │   │                  │   │  → SQLite       │
└────────────────┘   └──────────────────┘   └─────────────────┘
                               │
                    filesystem + {app_data}/project-hub.db
```

### Crates

| Crate | Role |
|-------|------|
| **`domain`** | Core types: projects, tasks, folders, calendar, activity, GitHub models |
| **`discovery`** | Scan directories and detect project type (Rust, Node, Python, Go, Java, …) |
| **`database`** | SQLite via `sqlx`, repositories, embedded migrations |
| **`git`** | Local Git status, commits, dirty files (`git2`) |
| **`github`** | GitHub API client and OAuth Device Flow |
| **`executor`** | Run project commands (cargo, npm, python, make) |
| **`brdd`** | Per-project `.brdd` folder: brief analysis, version snapshots, changelog |
| **`core`** | Application services shared by the desktop shell and the HTTP server |
| **`watcher`** | Filesystem watching placeholder (planned) |

### Frontend

| Area | Location |
|------|----------|
| Pages | `Dashboard`, `Projects`, `Project detail`, `Tasks`, `Calendar`, `Settings` |
| State | Zustand stores in `frontend/src/stores/` |
| IPC | `frontend/src/api/index.ts` maps to Tauri commands |
| i18n | 10 locales under `frontend/src/i18n/locales/` |

## Repository layout (`main`)

Application source only:

```
project-hub/
├── apps/desktop/       # Tauri 2 shell — IPC commands
├── apps/server/        # headless HTTP API (project-hub-server)
├── crates/             # Rust libraries (domain, core services, infrastructure)
├── frontend/           # React UI
└── migrations/         # SQL migrations (embedded at build time)
```

Release tooling (also in the repo, not part of the runtime app):

```
scripts/                # Frontend dev/build helpers
docker/                 # Linux x64 release image
Makefile.build          # Cross-platform bundle builds
.github/workflows/      # CI
```

## Prerequisites

- Rust 1.77+ (`rustup`)
- Node.js 20+
- Platform deps for Tauri 2 — see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Development

```bash
# Frontend dependencies
cd frontend && npm install && cd ..

# Desktop app (Vite dev server + Tauri)
cd apps/desktop && cargo tauri dev

# Tests
cargo test --workspace
cd frontend && npm test
```

## `.brdd` — analysis in every project

Project Hub can keep a small folder inside each project:

| File | Content |
|------|---------|
| `summary.md` | short analysis: version, stack, size, layout, dependencies, commands, git, tasks |
| `project.json` | the same, machine-readable (for agents) |
| `versions.json` | version snapshots — a new one when the manifest version or `HEAD` changes |
| `CHANGELOG.md` | commits and diff stat between snapshots |
| `tasks.json` | the project's Project Hub tasks |
| `ai-summary.md` | notes written by you or an AI agent (BoardDo) — never overwritten |

Refresh it from the project page (`.brdd` panel), for all projects in Settings → `.brdd & automation`,
or over the API. In git repositories `/.brdd/` is added to `.git/info/exclude` (local only), so the
working tree stays clean; delete that line to commit the folder. “Maintain .brdd automatically”
refreshes every project touched by a scan.

## Headless server & HTTP API (`apps/server`)

`project-hub-server` is Project Hub without the window — the same services and, by default, the
same database as the desktop app — exposed as a JSON API for scripts, AI agents and
[BoardDo](https://github.com/igornet0) (which installs and supervises it as an extension).

```bash
cargo build --release -p project-hub-server
./target/release/project-hub-server --self-test          # end-to-end check on a temporary DB
PROJECT_HUB_TOKEN=secret ./target/release/project-hub-server --listen 127.0.0.1:7878 --commands detected
```

| Option (env) | Meaning |
|--------------|---------|
| `--listen` (`PROJECT_HUB_LISTEN`) | bind address, default `127.0.0.1:7878`, port `0` = random |
| `--token` (`PROJECT_HUB_TOKEN`) | require `Authorization: Bearer …` on every route except health |
| `--db` (`PROJECT_HUB_DB`) | database path (default: the desktop app's `project-hub.db`) |
| `--commands` (`PROJECT_HUB_COMMANDS`) | `off` (default) · `detected` (only detected project commands) · `any` |
| `--exit-on-stdin-eof` | stop when the supervising parent closes stdin |

On start it prints `PROJECT_HUB_READY {"url":…}` on stdout. Endpoints (`/api/v1`, errors are
`{"error","code"}`):

- `GET /health` · `GET /projects?q=&limit=&offset=` · `GET /projects/{id}` · `GET /projects/{id}/detail`
- `POST /projects/scan {path}` · `POST /projects/{id}/refresh` · `GET|POST /roots`
- `GET /projects/{id}/git?limit=` · `GET /projects/{id}/tree?path=&depth=` · `GET /projects/{id}/file?path=&max_bytes=` · `GET /projects/{id}/search?q=`
- `GET /projects/{id}/commands` · `POST /projects/{id}/commands/run {command, timeout_secs}`
- `GET /projects/{id}/brdd` · `POST /projects/{id}/brdd/refresh {force_snapshot}` · `PUT /projects/{id}/brdd/notes {markdown}` · `POST /brdd/refresh-all` · `GET|PUT /brdd/settings`
- `GET /tasks?project_id=&status=&ids=&source=` · `POST /tasks` (`source`, `external_ref` link the task to the caller) · `GET|PATCH|DELETE /tasks/{id}`
- `POST /tasks/{id}/github-issue {labels}` · `GET /projects/{id}/issues?state=` · `POST /github/issues/sync` · `GET /github` · `POST /github/connect` · `POST /github/sync`
- `GET /activity?project_id=&limit=`

File access is read-only and confined to the project folder.

### Tasks ↔ GitHub issues

A task can open an issue in its project's repository (`github-issue`). Sync is two-way: a closed
issue completes the task, a reopened one moves a done task back to *todo*, and a task completed in
Project Hub closes its issue.

## GitHub OAuth (optional)

GitHub sync uses OAuth Device Flow. Each environment needs its own OAuth App — do not commit Client IDs.

1. Create an OAuth App: [GitHub → Developer settings](https://github.com/settings/applications/new)
   - Name: `Project Hub`
   - Homepage: `http://localhost`
   - Callback URL: leave empty (Device Flow)
2. Copy the Client ID into one of:
   - `apps/desktop/github-oauth-client-id` (copy from `github-oauth-client-id.example`)
   - `GITHUB_OAUTH_CLIENT_ID` environment variable at build time

Tokens are stored in the app data directory after login, not in the repository.

## Release builds

From the repository root:

```bash
make -f Makefile.build help

# Native installers for current OS
make -f Makefile.build bundle

# All macOS architectures (on macOS)
make -f Makefile.build bundles-darwin

# Linux x64 via Docker (recommended on arm64 Mac)
make -f Makefile.build bundle-linux-x64

# All 7 targets (macOS + Linux + Windows; needs Docker, cross, cargo-xwin)
make -f Makefile.build bundles-all WINDOWS_RUNNER=cargo-xwin
```

Artifacts are collected under `bundles/<target-triple>/`.

| Target | Output |
|--------|--------|
| `aarch64-apple-darwin` | `.app`, `.dmg` |
| `x86_64-apple-darwin` | `.app`, `.dmg` |
| `universal-apple-darwin` | Universal `.app`, `.dmg` |
| `x86_64-unknown-linux-gnu` | `.deb`, AppImage |
| `aarch64-unknown-linux-gnu` | `.deb`, AppImage |
| `x86_64-pc-windows-msvc` | `.msi`, NSIS |
| `aarch64-pc-windows-msvc` | `.msi`, NSIS |

## Roadmap

| Version | Focus |
|---------|--------|
| 0.1 | Tauri + React + SQLite foundation |
| 0.2 | Project discovery |
| 0.3 | Gallery |
| 0.4 | Project details |
| 0.5 | Git integration |
| 0.6 | Tasks |
| 0.7 | Calendar |
| 0.8 | Commands |
| 0.9 | Filesystem watcher |
| 1.0 | MVP release |

## License

MIT — see [LICENSE](LICENSE).
