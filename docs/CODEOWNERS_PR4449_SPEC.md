# Platform PR review workflow

Status: implemented and locally verified; publication and activation pending. User selected GitHub Actions/native functionality and requested spec, independent review, then build. This supersedes the service-selection proposals in PR_REVIEW_WORKFLOW_RESEARCH.md. Publication, live protection changes and Slack delivery are separate from building and validating the implementation.

## Outcome

Implement the responsibility sheet with separate owners/reviewers, mandatory author self-review, both bots before automated human invitations, five active non-draft PRs per author, a daily report, and `/review-prs`. Unmapped paths fall back to QuantumExplorer and shumkov. Do not grant any team or person new access. Exclude strophy and Silvanassss. No special infrastructure assignment; unmapped workflows use fallback.

GitHub Actions is the selected runtime. A small Python standard-library implementation fits existing repository `.github`/`scripts` tooling and avoids another service or package install. One evaluator supplies the required status, label, reviewer requests and read-only report. The skill calls that evaluator; it does not determine routing, count PRs or calculate age itself.

## Authority and constraints

- Continue local `takeover/codeowners-4449`, tracking QE's PR branch, using forward commits only. Preserve `.orca/` and unrelated work.
- Keep existing CODEOWNERS effective until activation: change it to the direct sheet roster, removing invalid teams. Stage a comment-only `.github/CODEOWNERS` only during the separate activation procedure if native requests must be disabled. Never claim deferred automatic human requests while root CODEOWNERS remains active.
- Workflow mutations require repository variable `PR_REVIEW_AUTOMATION_ENABLED=true`, default off. CLI is read-only unless `--apply` is given. Slack sending is separately opt-in; default scheduled output is a report artifact.
- Do not relax branch protection during build. Owner exemption can be computed and tested, but is not active while native one-review/code-owner requirements remain.
- This Actions solution assumes trusted repository writers. Same-app status spoofing and existing admin bypass are not solved. Do not describe it as a strong adversarial permission boundary. No PR code executes with write credentials: event-driven evaluator checks out the repository default branch explicitly, with credentials persistence disabled.

## Ownership mapping

Canonical `.github/pr-review-policy.json` contains version, fallback, max_active_prs=5, target_branches=[v4.2-dev], and area records with literal directory prefixes, owners, reviewers, optional unresolved identities and documentation metadata. Root CODEOWNERS is generated from owner+reviewer union. No broad globs other than fallback. Reject unknown schema fields, malformed handles/paths, overlapping prefixes, duplicate area names, missing owner lists, and excluded identities. Directories absent on an older PR branch are not guessed into existence; manifest validation uses the authoritative source checkout.

| Area / paths under packages | Owners | Reviewers |
| --- | --- | --- |
| rs-drive; rs-drive-abci (separate area records) | QuantumExplorer | shumkov |
| rs-dpp | QuantumExplorer, shumkov | none |
| rs-platform-wallet; rs-platform-wallet-ffi (separate records) | llbartekll | ZocoLini, HashEngineering, romchornyi |
| rs-platform-wallet-storage | lklimek | none |
| swift-sdk | llbartekll, romchornyi | none |
| rs-sdk; rs-sdk-ffi (separate records) | lklimek, shumkov | none |
| dashmate | shumkov | Daniel (unresolved GitHub username) |
| js-dash-sdk; js-evo-sdk; wasm-sdk | shumkov | none |
| rs-dapi | lklimek | QuantumExplorer, shumkov |
| kotlin-sdk | HashEngineering | none |
| data-contracts, dashpay-contract, dpns-contract, masternode-reward-shares-contract, withdrawals-contract, wallet-utils-contract, token-history-contract, document-history-contract, keyword-search-contract | QuantumExplorer, shumkov | none |
| Everything else | QuantumExplorer, shumkov | none |

The first spec's mapping was independently checked against the screenshot. Adjacent proof-verifier, protocol utility, unified FFI/JNI, legacy dapi and JavaScript dash-spv packages remain fallback because the sheet does not unambiguously map them. External Tenderdash, GroveDB, Dash Evo Tool and rust-dashcore areas are identified in [MULTI_REPO_REVIEW_SCOPE.md](MULTI_REPO_REVIEW_SCOPE.md). They remain outside this Platform-only implementation; the extension combines reporting while preserving repository-local enforcement. Preserve contributor/QA names as display metadata, without promoting them to reviewers. QA is documentation only in this build.

Daniel must not be silently omitted or assigned a guessed account: the area has an explicit unresolved identity, and affected PRs report a configuration blocker; CODEOWNERS generation lists known handles with a documentation note, and activation verification fails while any identity is unresolved. Roman's existing romchornyi mapping remains stated. Full activation still requires confirming the roster.

## Evaluation model

Each PR is evaluated from a fresh, bounded GitHub snapshot: repository/id, author, base/head SHAs, draft/open status, creation time, changed files including previous filenames for renames, submitted reviews, issue comments, unresolved review threads, requested reviewers and recorded controller state. Paginate all lists; detect truncation and missing data. Unknown or incomplete evidence never produces success. Restrict evaluation to configured target branches. GitHub permission checks determine whether prospective human approvers still have write/maintain/admin access; explicitly named accounts with insufficient permission block their area rather than widening its roster.

For every changed path (both sides of a rename), resolve one area by exact prefix, otherwise fallback. An area's human requirement is satisfied if the authenticated PR author is an owner OR another owner/reviewer approved the current head. One eligible approval may cover multiple areas. Latest decisive review wins; COMMENTED does not erase an earlier decision, DISMISSED removes that review's approval, and outstanding CHANGES_REQUESTED by any verified repository writer or configured bot blocks regardless of owner exemption. Never infer authority from commit email or display name.

### Self-review

Use an unedited issue comment authored by the PR author with exactly `/self-reviewed FULL_HEAD_SHA`. It records that the author inspected the final diff and verification evidence. No GitHub self-approval is attempted. Require confirmation at or after both bot completion timestamps, so the final review includes their outcome. New commits require a new confirmation. Edited comments are not accepted because another writer can edit comment text. Automation enforces the attestation, not the quality of the human's reading. `/review-prs` reports the required command but never posts an attestation on the user's behalf.

### Bot evidence

Both CodeRabbit (coderabbitai[bot] or legacy coderabbitai) and thepastaclaw must have completed the current head. Do not accept any historical submitted review or a merely green generic CodeRabbit status (`fail_commit_status:false` permits skipped reviews).

- thepastaclaw: a current-head APPROVED or COMMENTED review with the anchored producer marker `<!-- thepastaclaw-review-phase v1 phase=final sha=HEAD ... -->`. Preliminary phases fail. Any outstanding changes request fails.
- CodeRabbit: current-head APPROVED review, or a comment from the configured CodeRabbit identity containing parseable `final_review_risk_coverage` JSON with `kind=reviewed` and `coveredCommitId=HEAD`. Treat that as completion evidence, not proof findings are fixed.
- Any unresolved bot review thread prevents readiness; unresolved human threads block merge but permit re-review after an author response; unknown producer formats and API failures are pending/error with explanation. No natural-language interpretation or automatic dispute resolution.
- Any edited or replaced bot summary is evaluated afresh; if it no longer certifies HEAD, readiness is removed. Final receipts are tied to bot account identity and head, never copied markers from another user.

Live evidence inspected: PR #4651 contains a thepastaclaw final-phase marker at the review's commit_id, with CHANGES_REQUESTED (must remain blocked). PR #4449 and other CodeRabbit summaries expose final_review_risk_coverage markers. These are adapters for observed formats, not guaranteed vendor APIs; tests pin them and unknown formats fail closed.

### Five active PRs

Scope: open non-draft PRs targeting configured Platform branches per authenticated PR author. This is review admission, not a pre-creation ban. Drafts do not occupy a slot; bot accounts are not silently mapped to humans. Count must not fail all PRs for an over-limit author.

Maintain admitted slots in the controller state comment, independent of head SHA, until the PR closes or returns to draft. Read lifecycle events to invalidate old admission after close/reopen or draft/ready transitions, including when reconciliation missed the inactive period. Preserve existing admitted non-drafts first; admit waiting PRs in creation-time/number order up to five. If conflicting state already shows more than five admitted slots, retain the earliest five recorded admissions, flag inconsistent state, and never silently succeed on unknown history. Serialize mutation runs repository-wide with cancel-in-progress=false. Events reconcile the affected author; every 15 minutes a rotating batch of three open PRs repairs missed events. Fetch only those authors' admission histories. A stable queue of 68 PRs takes at most six hours to cover; this is a repair interval, not a promise of immediate event delivery. Manual full sweeps remain explicit and can consume substantial quota. Read-only reports simulate admission without recording or sending anything. Activation preview shows which existing PRs would wait; no auto-close or forced draft conversions.

### Label, status and notifications

`ready-for-human` is computed when admission, self-review, bot evidence, permission checks and bot-thread checks pass, and an unsatisfied human area or actionable human objection remains. A human changes request or human thread blocks merge, but after an author self-review attestation newer than that objection, the objecting reviewer is requested again even on owner-authored PRs. Until that fresh author response, show waiting-for-author. This separates merge eligibility from readiness for re-review. Owner-only or already-approved PRs are merge-policy-ready, not a human inbox item. The status context is `Platform PR policy`; it is pending while any requirement is unsatisfied and success only when human areas also pass. API/configuration failure produces no success; attempt an error status on affected current heads in apply mode.

One bot-owned issue comment records schema version, admission timestamp, current head, state and ready_since, and explains remaining requirements. Read only exact controller markers from github-actions[bot]; invalid state must be reported. This persistence shares the trusted-writer limitation. Carry ready_since only across consecutive human-ready evaluations on the same head; reset/pause when blocked, and display unknown before a first recorded admission to the queue. Generic updated_at and total PR age are never used as review waiting time.

Request only missing eligible humans for unsatisfied areas plus current human objectors needing a renewed review, excluding author/bots. A person already requested is not requested again. Limit batches to GitHub's reviewer request capacity and report undelivered requests. Do not remove manually assigned reviewers. Never treat a label as proof of readiness. Manual mentions/review requests remain possible.

Before each mutation, re-fetch PR state/head/base and abort if changed. Non-actionable pending/error evaluations do not need repeated full review snapshots: they cannot certify a merge or send human invitations. Actionable states revalidate complete evidence and same-author admission history. Write a pending status before readiness mutations, then persist state/label/requests, then publish the final status for that same snapshot. Serialize local apply effects; failures stop the run and the next reconciliation repairs partial state. Immediately before success, re-fetch all policy-relevant evidence (reviews, comments, threads, permissions and repository admission context), compare a fingerprint excluding the controller's own expected writes, and reevaluate. A mismatch remains pending for the next reconciliation. Old head results never certify a new head. The normal GITHUB_TOKEN REST budget is 1,000 requests/hour/repository; bounded scheduled batches and author-scoped events reduce consumption, but bursts and pagination can still exhaust quota and must fail visibly. GitHub offers no atomic read-and-publish transaction, so a residual race remains after the final read and is repaired by events/scheduled reconciliation; this is not a claim of race-free adversarial enforcement. Refuse success when another open PR shares the same head with different policy context; commit-scoped statuses cannot safely distinguish them.

## Runtime and CLI

Files: canonical JSON; `scripts/pr_review/policy.py` pure validation/evaluation; `scripts/pr_review/github.py` paginated gh API adapter; `scripts/pr_review/main.py` CLI, reconciliation and report; tests under `scripts/pr_review/tests`; Actions evaluator/tests workflows; `.claude/skills/review-prs/SKILL.md` (existing project `.codex/skills` link exposes it in Codex).

CLI supports `report` (Markdown or JSON, optional --user/--pr), `sync` (preview unless --apply), `codeowners --check` and `validate`. Require explicit repository identity on apply; use the configured default repository dashpay/platform otherwise. Use subprocess argument arrays and stdin JSON, never shell interpolation of PR data. No dependencies beyond Python and gh.

The evaluator workflow runs on pull_request_target, issue_comment, workflow_dispatch, scheduled reconciliation and workflow_run completion. A minimal read-only pull_request_review signal workflow triggers workflow_run for prompt review/dismissal reevaluation; its untrusted outputs are ignored. Filter workflow_run names to avoid evaluator recursion, and evaluate fresh GitHub data instead of event bodies. Event jobs explicitly checkout the default branch, pin checkout action to an audited SHA, and use minimal token permissions. Bot check/summary changes converge through comment events and scheduled sweep. Check default-branch changes as well; don't assume GITHUB_TOKEN mutations start another workflow.

A daily scheduled workflow produces a report artifact. A dispatch input plus `PR_REVIEW_SLACK_ENABLED=true` and `PR_REVIEW_SLACK_WEBHOOK` allow a real Slack test after setup; enabled scheduled runs also send. Missing required credentials fail explicitly. Use plain-text Slack fields/escaped links and disable automatic mentions so PR titles cannot ping channels. One scheduled digest per weekday at 02:00 UTC; delivery retries are not automatic because Slack webhooks do not provide exactly-once semantics. GitHub's native personal reminders remain available separately. Daily report content and `/review-prs` both call the same read-only reporter.

### Requested notification extension (not yet implemented)

Send each distinct configured owner/reviewer a personal daily Slack digest: PRs currently awaiting their review, waiting time, links and next actions, plus blockers on their own PRs. A person with both roles receives one digest. Skip empty personal digests. Separately send one shared channel summary for everyone: actionable review queue, waiting times, responsible reviewers, and aggregate author/bot/admission blockers. Both outputs use the same daily snapshot and the existing bot/self-review readiness rules. Channel membership must not expand the reviewer roster.

The current implementation sends only a channel webhook digest. Personal delivery and the broader channel summary require a reviewed implementation extension, explicit GitHub-login-to-Slack-user mapping, and a configured shared channel. Delivery remains disabled until configured; the previously recorded 58-test result does not cover this extension.

The skill reads current state and reports reviews needed, author/bot blockers, waiting time and next actions. It has no authority to post self-review, request reviewers, resolve threads or send Slack messages. Existing feedback skills handle subsequently authorized work.

## Verification and rollout

Write meaningful tests before implementation. Test sheet entries explicitly, owner vs reviewer, mixed paths, rename/deletion, self-review spoof/edit/stale SHA, bot preliminary/stale/unknown receipts, bot failure even for owner, approvals dismissed/superseded, permissions, sixth PR waiting while five progress, sticky admission across head changes, duplicate SHA, incomplete pagination, label spoofing, readiness age and idempotent effects; review dismissal/self-review deletion during publication; human changes-requested then fixed/bot-reviewed/self-reviewed returning to the queue, including an owner-authored PR. Mock GitHub; tests make no network calls. Add dry-run live evidence separately without any mutation.

Independent code reviewers inspect pure policy semantics, GitHub/Actions trust/event wiring, and skill/report behavior. Resolve must-fixes before delivery. Validate the skill frontmatter and existing cross-tool link.

Activation is a separate exact packet: resolve Daniel/roster; review initial admission preview; merge reviewed files under existing protections; run read-only default-branch workflow; create ready-for-human label; enable evaluator writes; verify real statuses/requests; disable native CODEOWNERS auto-routing only with explicit replacement; require Platform PR policy and only then remove conflicting native approval rules if accepted. Keep native CI requirements and existing emergency authority unchanged unless specifically authorized. Configure Slack destination/timezone separately before enabling sends.

No runtime code behavior changes to Platform crates, no test-suite rebuilds, no service deployment, no team provisioning. Do not report live enforcement or Slack delivery merely because local tests pass.

## Sources and review

Prior research: PR_REVIEW_WORKFLOW_RESEARCH.md. GitHub: https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows ; https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners ; https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks .

Two independent spec reviewers found same-head evidence revalidation and human re-review invitation gaps. Both are incorporated above, including the owner-authored objection case. User's latest instruction authorizes building after review; no additional pre-code approval round is inferred for the already selected approach. Material deviations discovered in review must be surfaced before implementation.

API budget source: https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api . Lifecycle event source: https://docs.github.com/en/rest/using-the-rest-api/issue-event-types . Implementation review added missed-lifecycle admission reset, collection-error revocation, per-mutation identity guards, and bounded scheduled work to avoid an unbounded full-queue API sweep.

## Build verification

- 58 offline unit tests pass. Regression tests demonstrated failures before fixes for stale bot outcomes, altered review evidence, lifecycle slot reuse, unbounded event work, and invalid numeric configuration.
- Both independent code reviews completed; their must-fixes were incorporated. The accepted trusted-writer and non-atomic GitHub publication limits remain.
- actionlint passes all four new workflows; canonical CODEOWNERS matches the manifest; skill validation passes and the existing Codex skills link exposes `/review-prs`.
- Read-only live preview of PR #4449 reports `waiting-bots`: thepastaclaw final review is missing for the current head, and bot objections/threads remain outstanding. No review requests, statuses, comments, labels, permissions, branch protections or Slack messages were changed during verification.
- Daniel's handle remains unresolved. The implementation is local on `takeover/codeowners-4449`; this verification does not claim an updated remote PR or active merge enforcement.
