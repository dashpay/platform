# Platform Testnet Sync Worker

The Platform sync can run for 16+ hours, so the long-running work belongs on a persistent worker instead of a GitHub-hosted runner.

The worker:

1. Updates and cleans a dedicated `dashpay/platform` checkout.
2. Resolves the testnet baseline version from the latest public Dash Core release.
3. Verifies the preconfigured testnet baseline, then runs Platform build and Platform sync commands.
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
sudo /opt/dash-platform/ops/platform-testnet-sync/install-worker.sh

sudo editor /etc/platform-testnet-sync.env
sudo systemctl enable --now platform-testnet-sync.timer
```

The checkout should be dedicated to this worker because the harness resets and cleans it to `origin/$PLATFORM_BRANCH` before every run.
The installer validates that `PLATFORM_REPO_DIR` already exists as a writable git checkout for the `platform-sync` user.

## Required Configuration

Set these in `/etc/platform-testnet-sync.env`:

- `GITHUB_TOKEN`
- `PLATFORM_TESTNET_SYNC_BASELINE_READY_COMMAND`
- `PLATFORM_TESTNET_SYNC_PLATFORM_BUILD_COMMAND`
- `PLATFORM_TESTNET_SYNC_PLATFORM_SYNC_COMMAND`

`PLATFORM_TESTNET_SYNC_BASELINE_READY_COMMAND` should verify that the preconfigured baseline is on the selected `CORE_VERSION` and synced far enough for the Platform sync run. It is not intended to perform baseline synchronization on the Platform worker.

The worker commands run from `PLATFORM_REPO_DIR` and receive:

- `CORE_VERSION`
- `PLATFORM_SHA`
- `PLATFORM_TESTNET_SYNC_RUN_DIR`

Write logs or machine-readable metadata into `PLATFORM_TESTNET_SYNC_RUN_DIR` so failures can be inspected without overloading the GitHub status panel.

`GITHUB_TOKEN` and `GH_TOKEN` are stripped from worker command environments. They remain available only to the worker harness for publishing the final commit status.

## Operations

Run immediately:

```bash
sudo systemctl start platform-testnet-sync.service
```

Check timer:

```bash
systemctl list-timers platform-testnet-sync.timer
```

Follow logs:

```bash
journalctl -u platform-testnet-sync.service -f
```

Run artifacts are stored under `PLATFORM_TESTNET_SYNC_LOG_DIR`.
Run directories older than `PLATFORM_TESTNET_SYNC_LOG_RETENTION_DAYS` are pruned by `run-worker.sh`; the default is 30 days.
