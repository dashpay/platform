#!/usr/bin/env bash

set -euo pipefail

: "${PLATFORM_REPO_DIR:=/opt/dash-platform}"
: "${PLATFORM_BRANCH:=v3.1-dev}"
: "${LATEST_CORE_TESTNET_STATE_DIR:=/var/lib/latest-core-testnet-sync}"
: "${LATEST_CORE_TESTNET_LOG_DIR:=/var/log/latest-core-testnet-sync}"

mkdir -p "${LATEST_CORE_TESTNET_STATE_DIR}" "${LATEST_CORE_TESTNET_LOG_DIR}"

exec 9>"${LATEST_CORE_TESTNET_STATE_DIR}/run.lock"
if ! flock -n 9; then
  echo "latest Core testnet sync is already running"
  exit 0
fi

cd "${PLATFORM_REPO_DIR}"

git fetch --prune origin "${PLATFORM_BRANCH}"
git checkout "${PLATFORM_BRANCH}"
git reset --hard "origin/${PLATFORM_BRANCH}"

RUN_ID="$(date -u +%Y%m%dT%H%M%SZ)"
export LATEST_CORE_TESTNET_RUN_ID="${RUN_ID}"
export LATEST_CORE_TESTNET_SYNC_RUN_DIR="${LATEST_CORE_TESTNET_LOG_DIR}/${RUN_ID}"
export TARGET_SHA
TARGET_SHA="$(git rev-parse HEAD)"

node .github/scripts/latest-core-testnet-sync/run.cjs
