#!/usr/bin/env bash

set -euo pipefail

: "${PLATFORM_REPO_DIR:=/opt/dash-platform}"
: "${PLATFORM_BRANCH:=v3.1-dev}"
: "${LATEST_CORE_TESTNET_STATE_DIR:=/var/lib/latest-core-testnet-sync}"
: "${LATEST_CORE_TESTNET_LOG_DIR:=/var/log/latest-core-testnet-sync}"
: "${LATEST_CORE_TESTNET_LOG_RETENTION_DAYS:=30}"

mkdir -p "${LATEST_CORE_TESTNET_STATE_DIR}" "${LATEST_CORE_TESTNET_LOG_DIR}"

find "${LATEST_CORE_TESTNET_LOG_DIR}" \
  -mindepth 1 \
  -maxdepth 1 \
  -type d \
  -mtime +"${LATEST_CORE_TESTNET_LOG_RETENTION_DAYS}" \
  -exec rm -rf {} +

exec 9>"${LATEST_CORE_TESTNET_STATE_DIR}/run.lock"
if ! flock -n 9; then
  echo "latest Core testnet sync is already running"
  exit 0
fi

cd "${PLATFORM_REPO_DIR}"

RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
export LATEST_CORE_TESTNET_RUN_ID="${RUN_ID}"
export LATEST_CORE_TESTNET_SYNC_RUN_DIR="${LATEST_CORE_TESTNET_LOG_DIR}/${RUN_ID}"

node .github/scripts/latest-core-testnet-sync/run.cjs
