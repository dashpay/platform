# Platform PR review operations

The implementation is initially a preview. Existing GitHub approval rules still apply, including to owner-authored PRs. No permissions are granted by these files. The design and roster are in [CODEOWNERS_PR4449_SPEC.md](CODEOWNERS_PR4449_SPEC.md).

## Local commands

Use Python 3.10 or newer and an authenticated `gh` CLI with access to repository review evidence and collaborator permissions. From the repository root:

```sh
python3 -m scripts.pr_review.main validate
python3 -m scripts.pr_review.main codeowners --check
python3 -m unittest discover -s scripts/pr_review/tests -v
python3 -m scripts.pr_review.main report --pr 4449
python3 -m scripts.pr_review.main report --user shumkov
python3 -m scripts.pr_review.main sync
```

All commands above are read-only. `sync --apply` additionally requires the enabled repository's Actions context. `/review-prs` uses the same reporter. A report failure means readiness could not be verified, not that the queue is empty.

Edit `.github/pr-review-policy.json` to change responsibility assignments, then regenerate the root file:

```sh
python3 -m scripts.pr_review.main codeowners > CODEOWNERS
```

Owners and reviewers appear together in native CODEOWNERS. Their distinct self-merge rules exist in the policy evaluator. All assigned identities must have verified repository write access; missing access blocks the area and does not trigger provisioning. Dashmate's Daniel identity remains unresolved. Confirm it before activation.

## Author and reviewer flow

1. Keep work in draft while preparing it. Up to five non-draft PRs per author targeting `v4.2-dev` are admitted, preserving existing admitted PRs. The sixth waits; automation never closes it or prevents creation.
2. Finish the current-head final review by both thepastaclaw and CodeRabbit, address outstanding bot changes requests, and resolve bot threads.
3. Inspect the final diff and verification yourself. Post an unedited issue comment containing exactly `/self-reviewed FULL_HEAD_SHA`, after both bot outcomes. A new commit or later bot completion requires a fresh attestation.
4. Automation requests eligible humans for areas needing approval. Owners can satisfy their own area's human requirement. A reviewer cannot satisfy that requirement on their own PR. Other CI and protection rules still apply.
5. Human objections block merge. After addressing them, renew self-review after the objection; the objector returns to the review queue. Their objection or thread still needs resolution before merge.

`ready-for-human` is an output, never authorization by itself. The report gives the current review cycle's waiting time when recorded. Requests made manually or by native CODEOWNERS can arrive earlier until native routing is disabled during activation.

## Activation checklist

Activation changes live behavior and must be reviewed separately from this build:

- Confirm Daniel and every owner's/reviewer's current account and access. Keep strophy and Silvanassss excluded; do not grant existing broad teams write access.
- Inspect the preview's initial five-slot allocation and bot-format compatibility on representative PRs. Resolve configuration errors.
- Merge the reviewed implementation under current protections. Ensure it is present on the repository default branch: privileged workflows explicitly execute that checkout. Scheduled workflows also require default-branch installation.
- Run the default-branch workflows in preview. Verify the Actions token can read collaborator permissions and complete review threads, and measure API use and runtime with the actual open queue.
- Create the `ready-for-human` label. Set `PR_REVIEW_AUTOMATION_ENABLED=true` only after verifying the preview.
- Observe controller comments, requests and `Platform PR policy` statuses on real PRs. Require this status on target branches while retaining existing CI.
- For delayed human invitations, place a comment-only `.github/CODEOWNERS` in the target branch, which takes precedence over root CODEOWNERS. Remove conflicting native required-code-owner/approval settings only when the replacement is verified and the owner exemption is accepted. Native routing and native approval settings are separate controls.

GitHub Actions shares an app identity across repository workflows. This setup assumes trusted repository writers and retains existing administrator authority; it is not an adversarial merge-permission boundary. A GitHub read followed by a write is not atomic. Events and scheduled reconciliation repair changes observed afterward. Duplicate open PRs with the same head are blocked because a commit status cannot distinguish their policy contexts.

## Daily report and optional Slack

The digest workflow produces a Markdown job summary and artifact at 02:00 UTC on weekdays. GitHub schedules may be delayed. Reports include author blockers; Slack includes only PRs ready for humans.

To enable delivery, configure an incoming webhook for the agreed Slack channel as secret `PR_REVIEW_SLACK_WEBHOOK` and set repository variable `PR_REVIEW_SLACK_ENABLED=true`. Test with the workflow dispatch's `send_slack` input. Once enabled, scheduled runs send automatically. Confirm the destination and schedule before setting the variable. Slack delivery is never retried automatically; an ambiguous error may mean Slack already received the message. Inspect before manually rerunning.

## Recovery

A configuration/API error must be investigated before treating the policy as satisfied. Missing permissions, deleted users, malformed/duplicate controller comments, truncated GitHub evidence, or changed bot marker formats cannot be interpreted as approval. Do not edit an author attestation to repair it; the author must post a new one.

A controller comment stores admission and review-cycle age. Deleting it loses that recorded history; the next run recomputes admission. More than five persisted admissions for an author is inconsistent state and requires investigation. Never delete unrelated comments as a recovery shortcut.

The evaluator is serialized repository-wide. Events reconcile the affected author; scheduled runs rotate through three PRs every 15 minutes as missed-event repair. A stable 68-PR queue takes up to six hours to cover. `sync --batch-size 3` reproduces the scheduled selection; unbatched `sync` explicitly evaluates the full queue. Read-only reports and privileged revalidation can consume substantial GitHub API quota on a large queue; check workflow runtime and rate limits before activation. An API outage cannot guarantee revocation of a previously written success when the status endpoint is also unavailable.

For rollback, restore the previously recorded native approval/routing settings first, then remove the required custom status and disable `PR_REVIEW_AUTOMATION_ENABLED`. Disabling writes alone leaves old statuses and labels in place. Disable Slack independently with `PR_REVIEW_SLACK_ENABLED=false`.
