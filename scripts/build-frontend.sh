#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FRONTEND_DIR="$ROOT/frontend"

setup_node_path() {
  export PATH="/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"
  if [ -n "${HOME:-}" ]; then
    export PATH="${HOME}/.local/bin:${PATH}"
    if [ -d "${HOME}/.fnm/aliases/default/bin" ]; then
      export PATH="${HOME}/.fnm/aliases/default/bin:${PATH}"
    fi
    if [ -d "${HOME}/.nvm/versions/node" ]; then
      for ver in $(ls -1 "${HOME}/.nvm/versions/node" 2>/dev/null | sort -V); do
        export PATH="${HOME}/.nvm/versions/node/${ver}/bin:${PATH}"
      done
    fi
  fi
}
setup_node_path

if [ -f "${FRONTEND_DIR}/dist/index.html" ] && \
   { [ "${SKIP_FRONTEND_BUILD:-}" = "1" ] || [ "${CI:-}" = "true" ] || [ "${CI:-}" = "1" ]; }; then
  echo "Skipping frontend build (dist exists, CI/SKIP_FRONTEND_BUILD set)"
  exit 0
fi

if ! command -v npm >/dev/null 2>&1; then
  echo "npm not found in PATH. Install Node.js 20+ or build frontend first:" >&2
  echo "  cd frontend && npm run build" >&2
  exit 127
fi

cd "${FRONTEND_DIR}"
npm run build
