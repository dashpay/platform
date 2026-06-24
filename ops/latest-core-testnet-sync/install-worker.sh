#!/usr/bin/env bash

set -euo pipefail

: "${PLATFORM_REPO_DIR:=/opt/dash-platform}"
PLATFORM_SYNC_USER=platform-sync
PLATFORM_SYNC_GROUP=platform-sync

if [[ "${EUID}" -ne 0 ]]; then
  echo "install-worker.sh must be run as root" >&2
  exit 1
fi

if ! getent group "${PLATFORM_SYNC_GROUP}" >/dev/null 2>&1; then
  groupadd --system "${PLATFORM_SYNC_GROUP}"
fi

if ! id "${PLATFORM_SYNC_USER}" >/dev/null 2>&1; then
  useradd \
    --system \
    --create-home \
    --shell /usr/sbin/nologin \
    --gid "${PLATFORM_SYNC_GROUP}" \
    "${PLATFORM_SYNC_USER}"
fi

install -d -o "${PLATFORM_SYNC_USER}" -g "${PLATFORM_SYNC_GROUP}" "${PLATFORM_REPO_DIR}"
install -d -o "${PLATFORM_SYNC_USER}" -g "${PLATFORM_SYNC_GROUP}" /var/lib/latest-core-testnet-sync
install -d -o "${PLATFORM_SYNC_USER}" -g "${PLATFORM_SYNC_GROUP}" /var/log/latest-core-testnet-sync

install -m 0644 \
  "${PLATFORM_REPO_DIR}/ops/latest-core-testnet-sync/latest-core-testnet-sync.service" \
  /etc/systemd/system/latest-core-testnet-sync.service
install -m 0644 \
  "${PLATFORM_REPO_DIR}/ops/latest-core-testnet-sync/latest-core-testnet-sync.timer" \
  /etc/systemd/system/latest-core-testnet-sync.timer

if [[ ! -f /etc/latest-core-testnet-sync.env ]]; then
  install -m 0600 \
    "${PLATFORM_REPO_DIR}/ops/latest-core-testnet-sync/latest-core-testnet-sync.env.example" \
    /etc/latest-core-testnet-sync.env
  echo "Created /etc/latest-core-testnet-sync.env; fill it before enabling the timer."
else
  chmod 0600 /etc/latest-core-testnet-sync.env
fi

systemctl daemon-reload

echo "Installed latest Core testnet sync worker units."
echo "Next:"
echo "  1. Edit /etc/latest-core-testnet-sync.env"
echo "  2. Run: systemctl start latest-core-testnet-sync.service"
echo "  3. Enable nightly timer: systemctl enable --now latest-core-testnet-sync.timer"
