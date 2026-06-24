#!/usr/bin/env bash

set -euo pipefail

: "${PLATFORM_REPO_DIR:=/opt/dash-platform}"
: "${PLATFORM_BRANCH:=v3.1-dev}"
: "${PLATFORM_TESTNET_SYNC_STATE_DIR:=/var/lib/platform-testnet-sync}"
: "${PLATFORM_TESTNET_SYNC_LOG_DIR:=/var/log/platform-testnet-sync}"
: "${PLATFORM_TESTNET_SYNC_LOG_RETENTION_DAYS:=30}"

mkdir -p "${PLATFORM_TESTNET_SYNC_STATE_DIR}" "${PLATFORM_TESTNET_SYNC_LOG_DIR}"

find "${PLATFORM_TESTNET_SYNC_LOG_DIR}" \
  -mindepth 1 \
  -maxdepth 1 \
  -type d \
  -mtime +"${PLATFORM_TESTNET_SYNC_LOG_RETENTION_DAYS}" \
  -exec rm -rf {} +

exec 9>"${PLATFORM_TESTNET_SYNC_STATE_DIR}/run.lock"
if ! flock -n 9; then
  echo "Platform testnet sync is already running"
  exit 0
fi

cd "${PLATFORM_REPO_DIR}"

RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
export PLATFORM_TESTNET_SYNC_RUN_ID="${RUN_ID}"
export PLATFORM_TESTNET_SYNC_RUN_DIR="${PLATFORM_TESTNET_SYNC_LOG_DIR}/${RUN_ID}"

node .github/scripts/platform-testnet-sync/run.cjs
