#!/usr/bin/env bash
set -euo pipefail

ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
cd "$ROOT"

export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"
export CARGO_TERM_COLOR="${CARGO_TERM_COLOR:-always}"
export RUSTFLAGS="${RUSTFLAGS:--Dwarnings}"

log() {
  printf '[northroot-codex-setup] %s\n' "$*"
}

have() {
  command -v "$1" >/dev/null 2>&1
}

install_with_brew() {
  if have brew; then
    brew install "$@" || true
  fi
}

install_with_apt() {
  if have apt-get; then
    if [ "$(id -u)" = "0" ]; then
      apt-get update
      DEBIAN_FRONTEND=noninteractive apt-get install -y "$@" || true
    elif have sudo; then
      sudo apt-get update
      sudo DEBIAN_FRONTEND=noninteractive apt-get install -y "$@" || true
    fi
  fi
}

ensure_baseline_packages() {
  install_with_brew ca-certificates curl git jq just pkg-config ripgrep
  install_with_apt ca-certificates curl git jq pkg-config ripgrep build-essential libssl-dev
}

ensure_rustup() {
  if have rustup; then
    return
  fi

  log "installing rustup with the minimal stable profile"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \
    | sh -s -- -y --profile minimal --default-toolchain stable
  export PATH="$HOME/.cargo/bin:$PATH"
}

ensure_rust_toolchain() {
  ensure_rustup

  local toolchain
  toolchain="$(awk -F '"' '/channel/ { print $2; exit }' rust-toolchain.toml)"
  if [ -z "$toolchain" ]; then
    echo "could not read Rust channel from rust-toolchain.toml" >&2
    return 1
  fi

  log "ensuring Rust toolchain ${toolchain} with clippy and rustfmt"
  rustup toolchain install "$toolchain" --profile minimal --component clippy --component rustfmt
  rustup component add clippy rustfmt --toolchain "$toolchain"

  if [ "${NORTHROOT_CODEX_DEEP_TOOLS:-0}" = "1" ]; then
    log "ensuring nightly miri for deep checks"
    rustup toolchain install nightly --profile minimal --component miri || true
  fi
}

ensure_just() {
  if have just; then
    return
  fi

  install_with_brew just

  if ! have just && have cargo; then
    log "installing just via cargo"
    cargo install just --locked || true
  fi
}

ensure_optional_cargo_tools() {
  if [ "${NORTHROOT_CODEX_DEEP_TOOLS:-0}" != "1" ]; then
    return
  fi

  local tools=(
    cargo-audit
    cargo-deny
    cargo-fuzz
    cargo-llvm-cov
  )

  for tool in "${tools[@]}"; do
    if ! have "$tool"; then
      log "installing optional deep-check tool: ${tool}"
      cargo install "$tool" --locked || true
    fi
  done
}

fetch_dependencies() {
  log "fetching workspace dependencies"
  cargo fetch --locked

  log "fetching CLI dependencies"
  cargo fetch --locked --manifest-path apps/northroot/Cargo.toml
}

report_environment() {
  local required=(git cargo rustc rustup rustfmt cargo-clippy)
  local missing=()

  for cli in "${required[@]}"; do
    if ! have "$cli"; then
      missing+=("$cli")
    fi
  done

  if [ "${#missing[@]}" -gt 0 ]; then
    printf 'missing required Codex environment tools: %s\n' "${missing[*]}" >&2
    return 1
  fi

  if ! have just; then
    printf 'missing optional convenience CLI: just\n' >&2
  fi

  rustc --version
  cargo --version
  log "environment ready"
}

ensure_baseline_packages
ensure_rust_toolchain
ensure_just
ensure_optional_cargo_tools
fetch_dependencies
report_environment
