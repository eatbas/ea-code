#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# Maestro — cross-platform dev launcher
# Detects OS/arch, checks prerequisites, installs deps, and runs tauri dev.
# Works on: Git Bash (Windows ARM & Intel), macOS (Apple Silicon & Intel).
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP_DIR="$SCRIPT_DIR/frontend/desktop"
TAURI_DIR="$DESKTOP_DIR/src-tauri"

# ── Colours (no-op when not a terminal) ──────────────────────────────────────
if [ -t 1 ]; then
  GREEN='\033[0;32m'; YELLOW='\033[1;33m'; RED='\033[0;31m'; BOLD='\033[1m'; RESET='\033[0m'
else
  GREEN=''; YELLOW=''; RED=''; BOLD=''; RESET=''
fi

info()  { printf "${GREEN}✓${RESET} %s\n" "$1"; }
warn()  { printf "${YELLOW}⚠${RESET} %s\n" "$1"; }
fail()  { printf "${RED}✗${RESET} %s\n" "$1"; exit 1; }
header(){ printf "\n${BOLD}── %s ──${RESET}\n" "$1"; }

# ── Detect platform ──────────────────────────────────────────────────────────
header "Environment"

OS_RAW="$(uname -s)"
ARCH_RAW="$(uname -m)"

case "$OS_RAW" in
  MINGW*|MSYS*|CYGWIN*) PLATFORM="windows" ;;
  Darwin)               PLATFORM="macos"   ;;
  Linux)                PLATFORM="linux"   ;;
  *)                    fail "Unsupported OS: $OS_RAW" ;;
esac

# On Windows ARM64, Git Bash runs under x86 emulation so uname -m reports
# x86_64. The real architecture is embedded in the uname -s output string
# (e.g. MINGW64_NT-10.0-26200-ARM64) or in the Rust host triple.
case "$ARCH_RAW" in
  aarch64|arm64) ARCH="arm64" ;;
  x86_64|AMD64)
    if [ "$PLATFORM" = "windows" ] && echo "$OS_RAW" | grep -qi "ARM64"; then
      ARCH="arm64"
    else
      ARCH="x64"
    fi
    ;;
  *) fail "Unsupported architecture: $ARCH_RAW" ;;
esac

info "Platform: $PLATFORM ($ARCH)"

# ── Platform-specific: clang setup ───────────────────────────────────────────
# Clang must be resolved BEFORE any cargo commands because cargo install (e.g.
# tauri-cli) runs outside the project tree and won't see .cargo/config.toml.
# We resolve the LLVM bin directory once and export CC/CXX + PATH for the
# whole session so every cargo invocation can find clang.

resolve_llvm_bin_win() {
  # 1. Explicit override
  if [ -n "${MAESTRO_LLVM_BIN:-}" ] && [ -f "$MAESTRO_LLVM_BIN/clang.exe" ]; then
    echo "$MAESTRO_LLVM_BIN"; return; fi
  # 2. Already on PATH
  if command -v clang >/dev/null 2>&1; then
    dirname "$(command -v clang)"; return; fi
  # 3. Standalone LLVM install
  if [ -f "${PROGRAMFILES:-}/LLVM/bin/clang.exe" ]; then
    echo "$PROGRAMFILES/LLVM/bin"; return; fi
  if [ -f "${ProgramFiles:-}/LLVM/bin/clang.exe" ]; then
    echo "$ProgramFiles/LLVM/bin"; return; fi
  # 4. Visual Studio bundled Clang (2022/2019 editions)
  local vs_base
  for vs_base in "$PROGRAMFILES" "${ProgramFiles:-}" "${ProgramFiles_x86_:-}"; do
    [ -z "$vs_base" ] && continue
    for yr in 2022 2019; do
      for ed in Community Professional Enterprise BuildTools Preview; do
        local p="$vs_base/Microsoft Visual Studio/$yr/$ed/VC/Tools/Llvm/bin"
        if [ -f "$p/clang.exe" ]; then echo "$p"; return; fi
      done
    done
  done
  return 1
}

if [ "$PLATFORM" = "windows" ] && [ "$ARCH" = "arm64" ]; then
  header "Windows ARM64 — Clang"
  if LLVM_BIN="$(resolve_llvm_bin_win)"; then
    export PATH="$LLVM_BIN:$PATH"
    export CC="$LLVM_BIN/clang.exe"
    export CXX="$LLVM_BIN/clang++.exe"
    CLANG_VER="$(clang --version 2>/dev/null | head -1)" || CLANG_VER="(found)"
    info "Clang — $CLANG_VER"
    info "LLVM bin: $LLVM_BIN (exported to PATH, CC, CXX)"
  else
    fail "Clang not found. ARM64 builds need LLVM Clang. Install Visual Studio 'C++ Clang tools' or standalone LLVM, or set MAESTRO_LLVM_BIN to your LLVM bin directory."
  fi
fi

if [ "$PLATFORM" = "macos" ]; then
  header "macOS — Clang"
  if command -v clang >/dev/null 2>&1; then
    CLANG_VER="$(clang --version 2>/dev/null | head -1)" || CLANG_VER="(found)"
    info "Clang — $CLANG_VER"
  else
    fail "Clang not found. Run: xcode-select --install"
  fi
fi

# ── Check prerequisites ─────────────────────────────────────────────────────
header "Prerequisites"

check_cmd() {
  local cmd="$1" label="${2:-$1}" hint="${3:-}"
  if command -v "$cmd" >/dev/null 2>&1; then
    local ver
    ver="$("$cmd" --version 2>/dev/null | head -1)" || ver="(version unknown)"
    info "$label — $ver"
    return 0
  else
    if [ -n "$hint" ]; then
      fail "$label not found. $hint"
    else
      fail "$label not found."
    fi
  fi
}

check_cmd node  "Node.js"  "Install from https://nodejs.org/"
check_cmd npm   "npm"
check_cmd rustc "Rust"     "Install from https://rustup.rs/"
check_cmd cargo "Cargo"

# Tauri CLI — install if missing (clang is already on PATH if needed)
if command -v cargo-tauri >/dev/null 2>&1 || cargo tauri --version >/dev/null 2>&1; then
  TAURI_VER="$(cargo tauri --version 2>/dev/null | head -1)" || TAURI_VER="(installed)"
  info "Tauri CLI — $TAURI_VER"
else
  warn "Tauri CLI not found — installing via cargo…"
  cargo install tauri-cli
  info "Tauri CLI installed"
fi

# ── Git submodules ───────────────────────────────────────────────────────────
header "Submodules"

cd "$SCRIPT_DIR"
UNINIT="$(git submodule status | grep -c '^-' || true)"
if [ "$UNINIT" -gt 0 ]; then
  warn "$UNINIT submodule(s) not initialised — running git submodule update…"
  git submodule update --init --recursive
  info "Submodules initialised"
else
  info "All submodules up to date"
fi

# ── npm dependencies ─────────────────────────────────────────────────────────
header "Dependencies"

cd "$DESKTOP_DIR"
if [ ! -d "node_modules" ]; then
  warn "node_modules missing — running npm install…"
  npm install
  info "npm packages installed"
elif [ "package.json" -nt "node_modules/.package-lock.json" ] 2>/dev/null; then
  warn "package.json changed — running npm install…"
  npm install
  info "npm packages updated"
else
  info "npm packages up to date"
fi

# ── Launch ───────────────────────────────────────────────────────────────────
header "Starting Maestro"
info "Running tauri dev from $DESKTOP_DIR"
printf "\n"

exec npm run tauri dev
