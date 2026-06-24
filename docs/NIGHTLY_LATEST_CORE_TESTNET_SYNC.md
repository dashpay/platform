# Nightly Latest Core Testnet Sync

This document defines the first repo-side contract for reporting Platform sync against the latest public Dash Core release on testnet.

The visible GitHub status is intentionally the last completed run only. A currently running build or sync must not replace the useful completed result with a running state.

## Visible Status

The external orchestrator reports one commit status on the tested Platform commit:

- Context: `Latest public Core testnet sync`
- Success state:
  - `Sync Passed`
- Failure states:
  - `Build Failed`
  - `Sync Failed`

Detailed build, Core, and sync logs belong behind the status `target_url`, not in the status description.

## System Phases

1. Keep a long-lived testnet Core node synced on the latest public Core release.
2. Build Platform for the target `dashpay/platform` commit on a fast disposable builder.
3. Run Platform sync on a normal sync runner using the synced Core baseline and freshly built Platform artifacts.
4. Report only the final completed result back to this repository.

Core baseline maintenance should be owned by a preconfigured, stateful, warm Core system. Platform build should be disposable and cache-heavy. Platform sync should be reproducible and log-rich.

## Scheduling

The long-running sync is owned by a persistent worker, not by a GitHub-hosted runner. Platform sync can take 16+ hours, which is too long for GitHub-hosted runner limits and too expensive to keep idle while waiting for chain progress.

The worker assets live in `ops/latest-core-testnet-sync/`:

- `install-worker.sh`
- `run-worker.sh`
- `latest-core-testnet-sync.service`
- `latest-core-testnet-sync.timer`
- `latest-core-testnet-sync.env.example`

The default timer runs nightly at 01:30 UTC with a 30 minute randomized delay. The service uses a lock file, so overlapping runs exit without changing the visible GitHub status.

The worker does not publish a pending or running commit status. It leaves the previous completed status in place while the new run is active, then publishes one final completed result when the run finishes.

The Platform sync worker assumes the latest-Core testnet node/baseline already exists. It may run a readiness command to verify that baseline before building and syncing Platform, but it should not perform a full Core chain sync itself.

The worker delegates host-specific work to environment variables:

- `LATEST_CORE_TESTNET_CORE_READY_COMMAND`
- `LATEST_CORE_TESTNET_PLATFORM_BUILD_COMMAND`
- `LATEST_CORE_TESTNET_PLATFORM_SYNC_COMMAND`

Optional variables:

- `LATEST_CORE_TESTNET_CORE_VERSION_COMMAND` overrides release discovery. It must print the Core version/tag on stdout.
- `LATEST_CORE_TESTNET_PHASE_TIMEOUT_MINUTES` overrides the per-phase timeout. Defaults to 1440 minutes.
- `LATEST_CORE_TESTNET_RESOLVE_TIMEOUT_MINUTES` overrides the Core-version resolution command timeout. Defaults to 30 minutes.
- `LATEST_CORE_RELEASE_REPO` overrides the GitHub release source. Defaults to `dashpay/dash`.
- `LATEST_CORE_TESTNET_LOG_DIR` controls where run logs and metadata are written.
- `LATEST_CORE_TESTNET_TARGET_URL` links the GitHub status to durable logs or a run page.

Each phase command receives:

- `LATEST_CORE_VERSION`
- `PLATFORM_SHA`
- `LATEST_CORE_TESTNET_SYNC_RUN_DIR`

The run directory should be used as the durable artifact location for phase logs and metadata.

The GitHub token is used only by the parent worker process for final status publication. It is intentionally stripped from the environment passed to Core readiness, Platform build, and Platform sync commands.

## Reporting Contract

When a run completes, the orchestrator sends a `repository_dispatch` event:

```json
{
  "event_type": "latest-core-testnet-sync-completed",
  "client_payload": {
    "status": "sync_passed",
    "target_sha": "0000000000000000000000000000000000000000",
    "target_url": "https://github.com/dashpay/platform/actions/runs/0000000000",
    "core_version": "vX.Y.Z",
    "platform_sha": "0000000000000000000000000000000000000000",
    "completed_at": "2026-06-24T05:30:00Z"
  }
}
```

Allowed `status` values:

- `sync_passed`
- `build_failed`
- `sync_failed`

Manual status testing is available through the `Latest public Core testnet sync status` workflow's `workflow_dispatch` trigger with the same fields. The normal worker path reports directly through the GitHub commit statuses API.

The dispatch credential should be held only by the persistent sync worker/operator account. The status workflow requires an explicit 40-character commit SHA that exists in `dashpay/platform`, and workflow-provided `target_url` values are limited to the `https://github.com` origin.

## Log and Artifact Expectations

The `target_url` should lead to a run page or artifact bundle containing:

- selected public Core version and binary/image digest
- target Platform commit and image digests
- Core tip height/hash and sync health at run start
- build logs
- Platform service logs
- Platform sync progress and terminal state
- concise failure phase and reason

The GitHub status description stays short so the repo panel remains readable.
