# PR review operations

All five repository configurations start in preview. The implementation does not grant permissions or change protection settings. The ownership rules and repository boundaries are described in [PR_REVIEW_ARCHITECTURE.md](PR_REVIEW_ARCHITECTURE.md).

## Read the queue

Use Python 3.10 or newer and an authenticated `gh` CLI. Run from Platform's checkout:

```sh
python3 -m scripts.pr_review.aggregate report
python3 -m scripts.pr_review.aggregate report --user shumkov
python3 -m scripts.pr_review.aggregate report --repo dashpay/rust-dashcore --format json
python3 -m scripts.pr_review.aggregate report --format deliveries
```

`/review-prs` uses this reporter. It combines repository-qualified PRs, actionable reviews, waiting times and author blockers. Counts include open, non-draft PRs on configured target branches. Five slots are enforced per repository; combined totals above five are workload warnings. Missing repository evidence is explicitly unavailable and totals are partial. The command exits nonzero for incomplete collection while preserving its report.

`.github/pr-review-repositories.json` selects the five repositories. In `preview` mode it uses the local seed manifest and labels results as preview. Switch an entry to `active` only after its policy is merged and verified; active collection reads that repository's canonical `.github/pr-review-policy.json` from its current default branch. It never substitutes a seed when an active repository cannot be read.

## Owner, reviewer and author flow

1. Keep unfinished work in draft. The oldest eligible PRs fill up to five author slots, preserving existing admissions. The sixth waits; automation does not prevent PR creation.
2. Finish final review by thepastaclaw and CodeRabbit on the current head, address outstanding bot changes requests and resolve bot threads.
3. Inspect the final diff and verification yourself. Post exactly `/self-reviewed FULL_HEAD_SHA` as an unedited issue comment after both bot outcomes. New commits or later bot completion require a new attestation.
4. Humans are invited for areas that still need approval. Owners satisfy their own area's human requirement; reviewers cannot do so on their own PRs. Existing CI and native protections continue to apply.
5. Human objections still block merging. Renew self-review after addressing an objection so its reviewer is invited back; the objection/thread must also be cleared before merge.

The computed `ready-for-human` label is an output, not independent proof of readiness. Waiting time is the current recorded review cycle, not total PR age. Native or manual review requests may arrive earlier until native automatic routing is explicitly disabled during activation.

## Shared implementation and repository-local rollout

The evaluator is shared through `.github/workflows/pr-review-reusable.yml`. Repository callers pin a full Platform commit SHA; the callee checks out that engine revision separately from the caller's trusted default-branch policy. The caller's GITHUB_TOKEN can mutate only its own repository. Target repository code is not executed with the write token.

Prepare reviewable rollout files after committing the shared implementation:

```sh
python3 -m scripts.pr_review.rollout --repo dashpay/tenderdash --engine-revision FULL_PLATFORM_COMMIT_SHA --output /tmp/tenderdash-review-policy
```

Repeat for GroveDB, Dash Evo Tool and rust-dashcore. The destination must not exist. Packets contain the policy, native CODEOWNERS, a pinned caller workflow and the review-signal workflow; they contain no copied evaluator. The referenced Platform commit must be published before another repository can call it. Inspect and apply packet files in a clean target checkout; the generator never overwrites a target repository itself.

Each repository needs its own review before activation:

- Confirm roster identities and repository permissions. Keep strophy and Silvanassss excluded and broad teams unchanged. Daniel remains unresolved in Platform; rust-dashcore has explicit missing-owner and cropped-scope gaps.
- Confirm configured target branches, existing CODEOWNERS precedence, native approval rules and both bot producers. Tenderdash currently disables automatic CodeRabbit reviews. rust-dashcore's existing readiness automation needs an explicit migration. Do not silently replace either.
- Merge under existing protections and run preview from the default branch. Verify complete evidence reads, token permissions and API usage.
- Create `ready-for-human`, then opt into writes with repository variable `PR_REVIEW_AUTOMATION_ENABLED=true`.
- Verify real current-head statuses, comments and requests before requiring `Platform PR policy`. Remove conflicting native approval/code-owner rules only when the owner exemption is approved and the replacement is working. Keep CI requirements.
- To suppress native early invitations, replace the effective CODEOWNERS with a comment-only `.github/CODEOWNERS` during that separate activation. Generated native routing is not delayed routing.

Events reevaluate the affected PR and changes to its author's slot assignments. Selector-less workflow invocations rotate three PRs at a time; the scheduled repair runs every 15 minutes. A stable 68-PR queue takes up to six hours to cover if event signals are missed. Pagination and event bursts can still exhaust quota; errors remain visible. Full local `sync` is an explicit unbounded sweep.

## Combined daily Slack delivery

One scheduled job at 02:00 UTC every day collects a single snapshot for the channel summary and all personal digests. GitHub schedules may be delayed. Each distinct mapped Slack user gets one combined DM, even if several GitHub identities map to that person. Empty personal digests are skipped; unavailable evidence is reported. The channel gets the actionable queue, author/bot blockers and workload warnings.

Configure a GitHub App installed only on these five repositories with read access to contents, pull requests, issues and metadata. Set repository variable `PR_REVIEW_APP_ID` and secret `PR_REVIEW_APP_PRIVATE_KEY` in Platform. The workflow creates and revokes short-lived selected-repository tokens. It requests no write or administrator permissions. Without this app, the job falls back to its repository token and must report any external repositories it cannot read as unavailable.

Configure a Slack app with `chat:write`, invite it to the shared channel, and set secret `PR_REVIEW_SLACK_BOT_TOKEN`. Fill `.github/pr-review-slack.json` with the exact channel ID and GitHub-login-to-Slack-user-ID mappings. Null values intentionally block sending and appear in delivery previews. No directory/email lookup or guessed identity is used. Set `PR_REVIEW_SLACK_ENABLED=true` only after inspecting the delivery preview and confirming recipients.

Test through workflow dispatch with `send_slack=true`. Enabled scheduled runs send automatically. Personal or repository filters cannot be combined with sending. Messages use plain-text blocks to prevent PR-controlled text from pinging channels. A missing mapping blocks all sends; partial repository evidence is disclosed in any delivered summary, never described as a complete empty queue.

Delivery is attempted once per destination and is not automatically retried. An error or uncertain acknowledgement stops later destinations and records which were delivered, uncertain or not attempted. Inspect Slack and those receipts before rerunning: a manual rerun is a new invocation and can duplicate earlier successful deliveries. The generated report is retained even when delivery fails.

## Verification and recovery

```sh
python3 -m unittest discover -s scripts/pr_review/tests -v
python3 -m scripts.pr_review.main validate
python3 -m scripts.pr_review.main codeowners --check
```

Edit seed ownership manifests and regenerate CODEOWNERS deliberately. An empty owner list is permitted only when the named area carries an explicit unresolved configuration blocker. It never grants owner powers to its reviewers.

The Actions solution assumes trusted repository writers. It shares an app identity across workflows, and GitHub offers no atomic read-and-publish transaction. Existing administrator bypass remains. These limitations are not solved by adding a shared collector.

For rollback, restore the recorded native approval/routing settings before removing the required custom status and disabling evaluator writes. Disabling writes alone leaves old statuses and labels. Disable Slack separately with `PR_REVIEW_SLACK_ENABLED=false`. Never repair an attestation by editing it for the author; require a new author comment. Investigate malformed or conflicting controller history rather than deleting unrelated comments.
