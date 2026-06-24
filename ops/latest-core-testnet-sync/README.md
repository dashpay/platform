# Latest Public Core Testnet Sync Worker

The Platform sync can run for 16+ hours, so the long-running work belongs on a persistent worker instead of a GitHub-hosted runner.

The worker:

1. Updates and cleans a dedicated `dashpay/platform` checkout.
2. Resolves the latest public Dash Core release.
3. Runs the configured Core sync, Platform build, and Platform sync commands.
4. Publishes one final commit status to the tested Platform commit:
   - `Sync Passed`
   - `Build Failed`
   - `Sync Failed`

It does not publish a running status. The previous completed GitHub status remains visible while a new worker run is active.

## Install

On the sync worker, clone a dedicated checkout first:

```bash
sudo useradd --system --create-home --shell /usr/sbin/nologin platform-sync
sudo git clone https://github.com/dashpay/platform.git /opt/dash-platform
sudo chown -R platform-sync:platform-sync /opt/dash-platform
```

Then install the units:

```bash
sudo /opt/dash-platform/ops/latest-core-testnet-sync/install-worker.sh

sudo editor /etc/latest-core-testnet-sync.env
sudo systemctl enable --now latest-core-testnet-sync.timer
```

The checkout should be dedicated to this worker because the harness resets and cleans it to `origin/$PLATFORM_BRANCH` before every run.
The installer validates that `PLATFORM_REPO_DIR` already exists as a writable git checkout for the `platform-sync` user.

## Required Configuration

Set these in `/etc/latest-core-testnet-sync.env`:

- `GITHUB_TOKEN`
- `LATEST_CORE_TESTNET_CORE_SYNC_COMMAND`
- `LATEST_CORE_TESTNET_PLATFORM_BUILD_COMMAND`
- `LATEST_CORE_TESTNET_PLATFORM_SYNC_COMMAND`

The phase commands run from `PLATFORM_REPO_DIR` and receive:

- `LATEST_CORE_VERSION`
- `PLATFORM_SHA`
- `LATEST_CORE_TESTNET_SYNC_RUN_DIR`

Write logs or machine-readable metadata into `LATEST_CORE_TESTNET_SYNC_RUN_DIR` so failures can be inspected without overloading the GitHub status panel.

`GITHUB_TOKEN` and `GH_TOKEN` are stripped from phase command environments. They remain available only to the worker harness for publishing the final commit status.

## Operations

Run immediately:

```bash
sudo systemctl start latest-core-testnet-sync.service
```

Check timer:

```bash
systemctl list-timers latest-core-testnet-sync.timer
```

Follow logs:

```bash
journalctl -u latest-core-testnet-sync.service -f
```

Run artifacts are stored under `LATEST_CORE_TESTNET_LOG_DIR`.
Run directories older than `LATEST_CORE_TESTNET_LOG_RETENTION_DAYS` are pruned by `run-worker.sh`; the default is 30 days.
