#!/usr/bin/env bash

# Install build dependencies for the project on Ubuntu 24.04 (noble).
#
# Main goal of this script is to build dependencies for environments
# operated by AI agents (e.g., Codex environments) where we may not have
# full control over the base image, but can run scripts to set up the
# necessary build tooling.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "${SCRIPT_DIR}/.." && pwd)

log() {
  printf '[install] %s\n' "$*"
}

warn() {
  printf '[install] WARNING: %s\n' "$*" >&2
}

# Verify we are running on Ubuntu 24.04 (noble)
if [[ -r /etc/os-release ]]; then
  # shellcheck disable=SC1091
  source /etc/os-release
  if [[ "${ID:-}" != "ubuntu" || "${VERSION_ID:-}" != "24.04" ]]; then
    warn "This script is tailored for Ubuntu 24.04; detected ${ID:-unknown} ${VERSION_ID:-unknown}."
  fi
else
  warn "Unable to detect operating system version via /etc/os-release."
fi

if command -v sudo >/dev/null 2>&1 && [[ "${EUID}" -ne 0 ]]; then
  SUDO="sudo"
else
  SUDO=""
fi

run_apt() {
  if [[ -n "${SUDO}" ]]; then
    # shellcheck disable=SC2024
    ${SUDO} env DEBIAN_FRONTEND=noninteractive apt-get "$@"
  else
    env DEBIAN_FRONTEND=noninteractive apt-get "$@"
  fi
}

log "Updating APT metadata"
run_apt update

log "Installing system build dependencies"
run_apt install -y --no-install-recommends \
  apt-transport-https \
  binutils \
  build-essential \
  ca-certificates \
  clang \
  cmake \
  curl \
  docker.io \
  git \
  gnupg \
  jq \
  libclang-dev \
  libsnappy-dev \
  libssl-dev \
  libzmq3-dev \
  llvm \
  pkg-config \
  python3 \
  python3-dev \
  python3-pip \
  python3-venv \
  unzip \
  wget \
  xz-utils \
  zip

log "Ensuring Node.js 20 LTS via NodeSource"
NODE_MAJOR=0
if command -v node >/dev/null 2>&1; then
  NODE_MAJOR=$(node --version | sed 's/v\([0-9]*\).*/\1/')
fi
if (( NODE_MAJOR < 20 )); then
  if [[ -n "${SUDO}" ]]; then
    curl -fsSL https://deb.nodesource.com/setup_20.x | ${SUDO} -E bash -
  else
    curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
  fi
  run_apt install -y --no-install-recommends nodejs
else
  log "Node.js $(node --version) already satisfies the required version."
fi

enable_corepack() {
  if ! command -v corepack >/dev/null 2>&1; then
    warn "corepack command not found; skipping yarn enablement."
    return
  fi

  local target_user="${SUDO_USER:-${USER:-$(id -un)}}"
  local user_home
  user_home=$(getent passwd "${target_user}" | cut -d: -f6 || true)
  if [[ -z "${user_home}" ]]; then
    user_home="${HOME:-}"
  fi

  if [[ -n "${SUDO}" && -n "${user_home}" && "${target_user}" != "root" ]]; then
    if ! sudo -u "${target_user}" HOME="${user_home}" corepack enable >/dev/null 2>&1; then
      warn "Failed to enable corepack for user ${target_user}."
    fi
  else
    if ! corepack enable >/dev/null 2>&1; then
      warn "Failed to enable corepack for the current user."
    fi
  fi
}

enable_corepack

install_rust_toolchain() {
  local toolchain
  toolchain=$(grep -m 1 '^channel' "${REPO_ROOT}/rust-toolchain.toml" | awk '{print $3}' | tr -d '"' || true)
  if [[ -z "${toolchain}" ]]; then
    warn "Unable to determine toolchain from rust-toolchain.toml; defaulting to stable."
    toolchain="stable"
  fi

  if ! command -v rustup >/dev/null 2>&1; then
    log "Installing rustup with toolchain ${toolchain}"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --default-toolchain "${toolchain}"
  else
    log "Updating rustup and installing toolchain ${toolchain}"
    rustup self update
    rustup toolchain install "${toolchain}"
    rustup default "${toolchain}"
  fi

  if [[ -f "${HOME}/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    source "${HOME}/.cargo/env"
  fi

  rustup target add wasm32-unknown-unknown
  rustup component add --toolchain "${toolchain}" clippy rustfmt
}

install_rust_toolchain

install_protoc() {
  local version="32.0"
  local install_root="/usr/local/lib/protoc-${version}"
  local arch

  if command -v protoc >/dev/null 2>&1; then
    local current
    current=$(protoc --version | awk '{print $2}')
    if [[ "${current}" == "${version}" ]]; then
      log "protoc ${version} already installed."
      return
    fi
  fi

  case "$(uname -m)" in
    x86_64)
      arch="x86_64"
      ;;
    aarch64|arm64)
      arch="aarch_64"
      ;;
    *)
      warn "Unsupported architecture $(uname -m) for protoc; skipping installation."
      return
      ;;
  esac

  log "Installing protoc ${version}"
  local tmp_dir
  tmp_dir=$(mktemp -d)
  curl -fsSL "https://github.com/protocolbuffers/protobuf/releases/download/v${version}/protoc-${version}-linux-${arch}.zip" -o "${tmp_dir}/protoc.zip"
  unzip -q "${tmp_dir}/protoc.zip" -d "${tmp_dir}/protoc"

  if [[ -n "${SUDO}" ]]; then
    ${SUDO} rm -rf "${install_root}"
    ${SUDO} mkdir -p "${install_root}"
    ${SUDO} cp -r "${tmp_dir}/protoc"/* "${install_root}/"
    ${SUDO} ln -sf "${install_root}/bin/protoc" /usr/local/bin/protoc
  else
    rm -rf "${install_root}"
    mkdir -p "${install_root}"
    cp -r "${tmp_dir}/protoc"/* "${install_root}/"
    ln -sf "${install_root}/bin/protoc" /usr/local/bin/protoc
  fi

  rm -rf "${tmp_dir}"
}

install_protoc

install_cargo_tools() {
  local wasm_bindgen_version="0.2.103"

  if ! command -v wasm-bindgen >/dev/null 2>&1 || [[ "$(wasm-bindgen --version | awk '{print $2}')" != "${wasm_bindgen_version}" ]]; then
    log "Installing wasm-bindgen-cli ${wasm_bindgen_version}"
    cargo install --locked "wasm-bindgen-cli@${wasm_bindgen_version}"
  else
    log "wasm-bindgen-cli ${wasm_bindgen_version} already installed."
  fi

  if ! command -v wasm-pack >/dev/null 2>&1; then
    log "Installing wasm-pack"
    cargo install --locked wasm-pack
  else
    log "wasm-pack already installed."
  fi
}

install_cargo_tools

configure_docker_group() {
  if ! command -v docker >/dev/null 2>&1; then
    return
  fi

  local target_user="${SUDO_USER:-${USER:-$(id -un)}}"
  if id -nG "${target_user}" 2>/dev/null | grep -qw docker; then
    return
  fi

  if [[ -n "${SUDO}" && "${target_user}" != "root" ]]; then
    log "Adding ${target_user} to docker group (requires logout/login to take effect)"
    ${SUDO} usermod -aG docker "${target_user}" || warn "Failed to add ${target_user} to docker group."
  else
    warn "Run 'sudo usermod -aG docker <user>' to manage Docker without sudo."
  fi
}

configure_docker_group

log "Build tooling installation complete. Suggested next steps:"
log "  1. Restart your shell so PATH updates take effect (source ~/.profile)."
log "  2. Run 'yarn setup' from the repository root to build dependencies."
