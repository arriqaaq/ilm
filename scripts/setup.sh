#!/usr/bin/env bash
set -euo pipefail

CHECK_ONLY=false
[[ "${1:-}" == "--check-only" ]] && CHECK_ONLY=true

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# --- OS detection ---
OS="$(uname -s)"
case "$OS" in
  Darwin) PLATFORM=mac ;;
  Linux)  PLATFORM=linux ;;
  *)      echo "Unsupported OS: $OS (only macOS and Linux are supported)"; exit 1 ;;
esac

# --- Status helpers ---
ok()   { printf "  \033[32m[ok]\033[0m   %-14s %s\n" "$1" "$2"; }
miss() { printf "  \033[31m[MISS]\033[0m %-14s %s\n" "$1" "$2"; MISSING=true; }
old()  { printf "  \033[33m[old]\033[0m  %-14s %s\n" "$1" "$2"; MISSING=true; }
warn() { printf "  \033[33m[warn]\033[0m %-14s %s\n" "$1" "$2"; }
info() { printf "  \033[36m[info]\033[0m %-14s %s\n" "$1" "$2"; }

MISSING=false

has_sudo() {
  command -v sudo &>/dev/null
}

# --- Load nvm if available ---
load_nvm() {
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ -s "$NVM_DIR/nvm.sh" ]]; then
    # shellcheck source=/dev/null
    \. "$NVM_DIR/nvm.sh"
    return 0
  fi
  return 1
}

# ============================================================
echo ""
if $CHECK_ONLY; then
  echo "Environment check ($PLATFORM)"
else
  echo "Developer setup ($PLATFORM)"
fi
echo "=========================================="
echo ""

# --- 1. Rust ---
check_rust() {
  if ! command -v rustc &>/dev/null; then return 1; fi
  local ver
  ver=$(rustc --version | grep -oE '[0-9]+\.[0-9]+' | head -1)
  local major=${ver%%.*}
  local minor=${ver#*.}
  [[ "$major" -gt 1 ]] || { [[ "$major" -eq 1 ]] && [[ "$minor" -ge 85 ]]; }
}

rust_version() {
  rustc --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -1
}

if check_rust; then
  ok "Rust" "$(rust_version) (>= 1.85 required)"
elif command -v rustc &>/dev/null; then
  old "Rust" "$(rust_version) — need >= 1.85 for edition 2024"
  if ! $CHECK_ONLY; then
    info "Rust" "updating via rustup..."
    rustup update stable
    # shellcheck source=/dev/null
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    if check_rust; then
      ok "Rust" "$(rust_version) (updated)"
    else
      miss "Rust" "update failed — install manually: https://rustup.rs"
    fi
  fi
else
  miss "Rust" "not installed"
  if ! $CHECK_ONLY; then
    info "Rust" "installing via rustup..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    if check_rust; then
      ok "Rust" "$(rust_version) (installed)"
    else
      miss "Rust" "install failed — try manually: https://rustup.rs"
    fi
  fi
fi

# --- 2. Node.js ---
check_node() {
  if ! command -v node &>/dev/null; then return 1; fi
  local ver
  ver=$(node --version | grep -oE '[0-9]+' | head -1)
  [[ "$ver" -ge 20 ]]
}

node_version() {
  node --version 2>/dev/null || echo "none"
}

load_nvm 2>/dev/null || true

if check_node; then
  ok "Node.js" "$(node_version) (>= 20 required)"
else
  if command -v node &>/dev/null; then
    old "Node.js" "$(node_version) — need >= 20"
  else
    miss "Node.js" "not installed"
  fi
  if ! $CHECK_ONLY; then
    if load_nvm 2>/dev/null; then
      info "Node.js" "installing v20 via nvm..."
      nvm install 20
      nvm use 20
    elif [[ "$PLATFORM" == "mac" ]] && command -v brew &>/dev/null; then
      info "Node.js" "installing via Homebrew..."
      brew install node
    elif [[ "$PLATFORM" == "linux" ]] && has_sudo; then
      info "Node.js" "installing via NodeSource..."
      curl -fsSL https://deb.nodesource.com/setup_lts.x | sudo -E bash -
      sudo apt-get install -y nodejs
    else
      miss "Node.js" "could not install — install nvm: https://github.com/nvm-sh/nvm"
    fi
    if check_node; then
      ok "Node.js" "$(node_version) (installed)"
    fi
  fi
fi

# --- 3. Python 3 + venv ---
check_python() {
  command -v python3 &>/dev/null && python3 -c "import venv" 2>/dev/null
}

python_version() {
  python3 --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo "none"
}

if check_python; then
  ok "Python 3" "$(python_version)"
  ok "venv" "python3 -m venv works"
elif command -v python3 &>/dev/null; then
  ok "Python 3" "$(python_version)"
  miss "venv" "python3 -m venv not available"
  if ! $CHECK_ONLY; then
    if [[ "$PLATFORM" == "linux" ]] && has_sudo; then
      info "venv" "installing python3-venv..."
      sudo apt-get install -y python3-venv
      if check_python; then
        ok "venv" "installed"
      fi
    else
      miss "venv" "install manually: sudo apt install python3-venv"
    fi
  fi
else
  miss "Python 3" "not installed"
  if ! $CHECK_ONLY; then
    if [[ "$PLATFORM" == "mac" ]]; then
      info "Python 3" "install Xcode CLT: xcode-select --install"
      miss "Python 3" "run: xcode-select --install"
    elif [[ "$PLATFORM" == "linux" ]] && has_sudo; then
      info "Python 3" "installing..."
      sudo apt-get install -y python3 python3-venv
      if check_python; then
        ok "Python 3" "$(python_version) (installed)"
      fi
    fi
  fi
fi

# --- 4. uv ---
if command -v uv &>/dev/null; then
  ok "uv" "$(uv --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo 'installed')"
else
  miss "uv" "not installed"
  if ! $CHECK_ONLY; then
    info "uv" "installing via astral.sh..."
    curl -LsSf https://astral.sh/uv/install.sh | sh
    export PATH="$HOME/.local/bin:$PATH"
    if command -v uv &>/dev/null; then
      ok "uv" "$(uv --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo 'installed')"
    else
      miss "uv" "install failed — try manually: https://docs.astral.sh/uv/"
    fi
  fi
fi

# --- 5. Ollama (optional, never errors out) ---
echo ""
echo "Optional:"
install_ollama() {
  if command -v ollama &>/dev/null; then
    ok "Ollama" "$(ollama --version 2>/dev/null | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' || echo 'installed')"
    return
  fi
  if $CHECK_ONLY; then
    warn "Ollama" "not installed (optional — needed for LLM features)"
    return
  fi
  info "Ollama" "attempting install (optional)..."
  if [[ "$PLATFORM" == "mac" ]] && command -v brew &>/dev/null; then
    if brew install ollama 2>/dev/null; then
      ok "Ollama" "installed via Homebrew"
    else
      warn "Ollama" "brew install failed — install manually: https://ollama.com/download"
    fi
  elif [[ "$PLATFORM" == "linux" ]] && has_sudo; then
    if curl -fsSL https://ollama.com/install.sh | sh 2>/dev/null; then
      ok "Ollama" "installed"
    else
      warn "Ollama" "install failed — install manually: https://ollama.com/download"
    fi
  else
    warn "Ollama" "skipped (no brew/sudo) — install manually: https://ollama.com/download"
  fi
}
install_ollama

# --- 6. npm + frontend deps ---
echo ""
echo "Project:"
if command -v npm &>/dev/null; then
  ok "npm" "$(npm --version 2>/dev/null)"
else
  miss "npm" "not found (should come with Node.js)"
fi

if [[ -d "$PROJECT_DIR/frontend/node_modules" ]]; then
  ok "frontend" "node_modules/ present"
else
  if $CHECK_ONLY; then
    miss "frontend" "node_modules/ missing — run: make setup"
  else
    info "frontend" "running npm install..."
    (cd "$PROJECT_DIR/frontend" && npm install)
    ok "frontend" "npm install complete"
  fi
fi

# --- 7. Data readiness ---
echo ""
echo "Data:"
if [[ -f "$PROJECT_DIR/qul/qpc-hafs.json" ]]; then
  ok "qul" "qpc-hafs.json present"
else
  miss "qul" "qul/qpc-hafs.json missing"
fi

if [[ -f "$PROJECT_DIR/data/semantic_hadith.json" ]]; then
  ok "semantic" "data/semantic_hadith.json present"
else
  warn "semantic" "missing — run: make semantic-setup"
fi

if [[ -d "$PROJECT_DIR/db_data" ]]; then
  ok "database" "db_data/ present"
else
  warn "database" "missing — run: make pipeline-full (or pipeline-lite)"
fi

# --- Summary ---
echo ""
echo "=========================================="
if $MISSING; then
  if $CHECK_ONLY; then
    echo "Some required tools are missing. Run: make setup"
    exit 1
  else
    echo "Some tools could not be installed — see [MISS] items above."
    exit 1
  fi
else
  if $CHECK_ONLY; then
    echo "All required tools installed."
  else
    echo "Setup complete! Next steps:"
    echo "  make build          # compile backend + frontend"
    echo "  make pipeline-lite  # ingest data (no embeddings)"
    echo "  make dev            # start dev server"
  fi
fi
