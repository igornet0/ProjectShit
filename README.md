# Project Hub

Local-first project hub: discovers projects on your machine, understands their structure, Git, language and tooling — and provides a gallery, tasks, calendar, activity and command execution on top.

## Architecture

```
React UI  →  Tauri Commands  →  Application Core  →  Domain / Infrastructure
                                                              ↓
                                                           SQLite
```

Project source files stay on disk. SQLite stores metadata only (`root_path`, not file contents).

## Prerequisites

- Rust 1.77+
- Node.js 20+
- macOS / Linux / Windows

## Development

```bash
# Install frontend dependencies
cd frontend && npm install && cd ..

# Run desktop app in dev mode (from apps/desktop)
cd apps/desktop && cargo tauri dev

# Or build release bundle
cd apps/desktop && cargo tauri build

# Run Rust tests (from repo root)
cargo test --workspace

# Run frontend tests
cd frontend && npm test
```

## Workspace Layout

```
project-hub/
├── apps/desktop/       # Tauri 2 desktop shell
├── crates/
│   ├── domain/         # Core domain types
│   ├── discovery/      # Project detection engine
│   ├── git/            # Git integration (git2)
│   ├── database/       # SQLite + repositories
│   ├── executor/       # Command execution
│   └── watcher/        # Filesystem watching
├── frontend/           # React + TypeScript UI
└── migrations/         # SQL migration files
```

## MVP v0.1 Roadmap

| Version | Feature              |
|---------|----------------------|
| 0.1     | Tauri + React + SQLite |
| 0.2     | Project discovery    |
| 0.3     | Gallery              |
| 0.4     | Project details      |
| 0.5     | Git integration      |
| 0.6     | Tasks                |
| 0.7     | Calendar             |
| 0.8     | Commands             |
| 0.9     | Filesystem watcher   |
| 1.0     | MVP release          |
