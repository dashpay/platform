# Platform Testnet Sync

This document defines the repo-side reporting contract for Platform sync against the latest public Dash Core release on testnet.

The Platform sync worker and server infrastructure live outside this repository. This repository only owns the public status surface:

- the `Platform Sync` README badge
- the `Platform testnet sync status` workflow
- the `Platform testnet sync` commit status context

## Visible Status

The visible status is intentionally the last completed run only. A currently running build or sync must not replace the useful completed result with a running state.

The external worker reports one final completed status for the tested Platform commit:

- Context: `Platform testnet sync`
- Success state:
  - `Sync Passed`
- Failure states:
  - `Build Failed`
  - `Sync Failed`

Detailed build, baseline, and sync logs belong behind the status `target_url`, not in the status description.

## Reporting Flow

When a run completes, the external worker sends a `repository_dispatch` event to `dashpay/platform`:

```json
{
  "event_type": "platform-testnet-sync-completed",
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

The workflow validates the target commit SHA, writes the `Platform testnet sync` commit status, and fails the workflow run for `build_failed` and `sync_failed` so the README badge reflects the final outcome.

Manual status testing is available through the `Platform testnet sync status` workflow's `workflow_dispatch` trigger with the same fields.

## Expectations For The External Worker

The worker should:

- maintain or consume a synced latest-public-Core testnet baseline
- build Platform for the target `dashpay/platform` commit
- run Platform sync against that baseline
- report only the final completed result to this repository
- keep detailed logs and diagnostics outside this repository, linked through `target_url`

The dispatch credential should be held only by the external worker/operator account. The status workflow requires an explicit 40-character commit SHA that exists in `dashpay/platform`, and workflow-provided `target_url` values are limited to the `https://github.com` origin.

The GitHub status description stays short so the repo panel remains readable.
