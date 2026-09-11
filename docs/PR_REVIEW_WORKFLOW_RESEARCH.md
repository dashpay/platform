# Platform PR workflow research

Historical alternatives research. The user subsequently selected GitHub Actions/native functionality; [CODEOWNERS_PR4449_SPEC.md](CODEOWNERS_PR4449_SPEC.md) is the authoritative implementation decision. The provisional hosted-service recommendation below is retained as research context.

Date: 2026-09-11. Research and proposed behavior only; no applications installed, no Slack messages sent, no repository settings changed.

## Conclusion

The expanded requirement is an end-to-end review workflow: author workload limits, bot completion, delayed human requests, ownership-aware approvals, and a daily reviewer inbox. Policy-bot can handle approval policy but is not a complete workflow coordinator. Mergify is the strongest hosted candidate to validate for the expanded scope; gitStream is another substantial candidate. Neither has been established here as an off-the-shelf implementation of the exact per-author five-PR limit and thepastaclaw completion contract.

Prefer one policy engine, one canonical ownership map from the sheet, and one shared queue calculation consumed by Slack and a read-only skill. Do not deploy policy-bot and Mergify together for the same approval decision. The previous policy-bot recommendation is provisional pending this expanded comparison.

## What is already available

### GitHub's native PR cap

GitHub introduced a configurable open-PR cap on 2026-06-17. It genuinely prevents additional non-draft PR creation for users WITHOUT write access. Drafts do not count; writers/admins are exempt. Repository and organization settings and a bypass list are available. Therefore it can cover outside contributors but cannot enforce the requested cap on Platform's existing developers.

Read-only API check: `GET repos/dashpay/platform/interaction-limits/pulls/creation-cap` returned `enabled:false, max_open_pull_requests:1`. This is a disabled setting, not an active limit of one.

Sources: [GitHub announcement](https://github.blog/changelog/2026-06-17-limit-open-pull-requests-for-users-without-write-access/), [repository limits](https://docs.github.com/en/communities/moderating-comments-and-conversations/limiting-interactions-in-your-repository), [API](https://docs.github.com/en/rest/interactions/repos).

### GitHub's native Slack reminders

GitHub already supports daily scheduled reminders for personal or team review requests. An organization owner authorizes the Slack workspace; individual reviewers can configure their own reminders. This does not require granting broad organization teams repository write access. Each reminder covers up to five repositories and shows up to twenty oldest PRs per repository, so it is not an exhaustive large-queue report.

Use this first if simple daily pending-review reminders suffice. Exact waiting-in-stage duration, per-author queue caps, and bot-readiness explanations require richer queue data. No Slack workspace/account configuration was inspected or changed in this research.

Sources: [scheduled reminders](https://docs.github.com/en/subscriptions-and-notifications/concepts/scheduled-reminders), [personal reminders](https://docs.github.com/en/enterprise-cloud%40latest/subscriptions-and-notifications/how-tos/managing-your-scheduled-reminders).

### Existing local skills and repository automation

The current Platform skill tree contains pr-description, release, emulator-control, simulator-control, and protocol-upgrade-test. `.codex/skills` links to `.claude/skills`. No reviewer-inbox/waiting-age skill was found there.

Shared local skills include `monitor-pr` (converges a single PR's CI and review threads) and `address-pr-comments` (handles feedback with reply-before-resolution). These help the author clear bot findings; neither produces a multi-PR reviewer queue. Inspected those skills as existing assets, without invoking their mutation workflows.

At fetched `origin/v4.2-dev` (`d03953b4f3`), `.coderabbit.yaml` sets `reviews.fail_commit_status:false`. Its comment references an AI Review workflow, but `.github/workflows/pr-ai-review.yml` is absent from that ref. An older local revision contained a workflow accepting ANY submitted review by EITHER CodeRabbit or thepastaclaw. Neither the current configuration nor that historical check proves both bots completed successfully for the current PR revision or that their findings were addressed. Do not reuse it as that proof.

## Product comparison

| Option | Verified strengths | Work still needed for this request |
| --- | --- | --- |
| GitHub native | Outside-contributor PR cap, CODEOWNERS requests, review protections, scheduled Slack reminders | Writer cap; owner-author exemption; bot-before-human orchestration; precise queue-age report |
| policy-bot | File/author rules and AND/OR approval policy, trusted app status, base-branch config | Hosting; workload admission; label lifecycle; bot evidence adapter; daily queue report |
| Mergify | Hosted merge-protection rules, file/author/review/check conditions, labels, conditional review requests, Slack integration | Exact five-PR author admission and bot completion adapter; verify review freshness, ownership exemptions, notification timing and desired daily digest |
| gitStream | Conditional reviewers/required reviewers, labels, draft changes, per-PR approval counts, Slack webhook actions, custom extensions | Verify strict owner/reviewer policy, trust boundary and event coverage; cross-PR counting and exact daily digest need additional logic |
| Graphite | Reviewer inbox with needs-review/returned/waiting sections, Slack notifications | Not established as enforcement for this sheet, a five-PR cap, or both custom bot gates; useful optional reviewer UI |

Sources: [policy-bot](https://github.com/palantir/policy-bot), [Mergify protections](https://docs.mergify.com/merge-protections/), [Mergify conditions](https://docs.mergify.com/configuration/conditions/), [review requests](https://docs.mergify.com/workflow/actions/request_reviews/), [Mergify Slack](https://docs.mergify.com/integrations/slack/), [gitStream actions](https://docs.gitstream.cm/automation-actions/), [gitStream Slack](https://docs.gitstream.cm/integrations/slack/), [Graphite inbox](https://graphite.com/docs/use-pr-inbox), [Graphite Slack](https://graphite.com/docs/slack-notifications).

This is a documentation comparison, not a live product trial. No service is claimed to cover everything without custom work. Prices, contract terms, and installation permission sets have not been evaluated and must be checked before product selection.

## Proposed lifecycle

    Draft / author work
        -> Bot review
        -> Author addresses bot feedback (repeat as needed)
        -> ready-for-human + targeted review requests
        -> Human review / author changes
        -> Approved or owner exemption + all other gates satisfied
        -> Human chooses to merge

These are proposed states, not all necessarily separate labels. Use a single visible `ready-for-human` label for the primary human queue. Label state is computed output; manually adding a label must not certify readiness or bypass merge checks.

### Bot completion

Default proposal: both thepastaclaw and CodeRabbit must finish their applicable review of the current head, with no outstanding blocking findings. A preliminary phase, a historical review, a rate-limit/skip result, or an unresolved changes-requested review does not qualify. Explicitly define any exception policy rather than silently treating one bot as enough.

The controller must inspect structured identity, head SHA, completion state and thread/disposition evidence. Fixing a finding and explaining a justified disagreement are both legitimate ways to address it; simply resolving a thread is not evidence of correctness. Automated evaluation cannot decide whether a disputed finding is valid by guessing from prose. A bot revalidation or a recorded authorized disposition is needed. No universal adapter for thepastaclaw's final-stage contract was located in Platform; investigate its actual producer before implementation.

A new head invalidates bot readiness and removes the label until reevaluation. An owner exemption bypasses only the human approval requirement, not bot completion, CI, or unresolved objections. Human change requests also remove the PR from the reviewer's actionable queue until the author responds; they do not disappear from the PR.

CodeRabbit supports reviewing drafts, but `reviews.auto_review.drafts` defaults to false. A draft-first flow must explicitly enable and verify that behavior, and independently verify thepastaclaw and required CI support drafts. Source: [CodeRabbit configuration](https://docs.coderabbit.ai/reference/configuration).

### Review-request timing changes the earlier CODEOWNERS design

GitHub automatically requests CODEOWNERS when a PR opens as non-draft or is marked ready. A label cannot defer that request. A cooperative draft-first process can preserve native CODEOWNERS: the controller marks the PR ready only after bot checks and author readiness. However, an author manually marking it ready can cause an immediate request before the controller reacts; converting it back cannot retract the notification.

For a strict automation-controlled human queue, use the canonical sheet mapping to request reviewers only after readiness, instead of active native CODEOWNERS auto-routing. That materially changes the original PR design and needs alignment. It cannot prevent a person from manually requesting a review or mentioning another person; it governs automated invitations, reminders and merge eligibility. Avoid claiming otherwise.

Source: [GitHub CODEOWNERS behavior](https://docs.github.com/en/repositories/managing-your-repositorys-settings-and-features/customizing-your-repository/about-code-owners).

### Five-PR limit

For outside contributors, use the native cap if its non-draft scope matches the selected policy. For existing writers, none of the evaluated documentation established a native pre-creation cap. A controller can withhold admission to the automated review queue and publish an explanatory pending gate, but cannot claim GitHub's Create PR button is disabled for writers. Do not auto-close people's work.

Recommend a maximum of five active review slots per PR author in Platform, while allowing drafts. This is explicitly a proposal that differs from limiting all open PRs. Decide whether slots begin at bot review or human readiness; bot review admission limits bot load too. Author-level admission must be serialized or deterministically reconciled, not implemented as concurrent independent count checks. Bots acting under their own account count under that account unless a verified principal mapping is configured; do not guess a human author from commit email.

The sixth PR waits; the five admitted PRs remain able to finish. On merge, close, or an explicit return to inactive draft, release a slot and reevaluate the waiting queue. Pending active PRs must not all fail merely because an author is over the limit. Existing excess work needs a transition: initial visibility/warnings plus an explicit choice of active slots, not mass closure or silent removal of requests.

Live snapshot: 68 open PRs. Non-drafts include PastaPastaPasta 14, QuantumExplorer 7, shumkov 7, llbartekll 5. These are not counts of bot-clean or human-ready PRs, and must not be used as a substitute for readiness. The count was obtained from all pages of the open-PR API.

### Daily Slack and a read-only review skill

Proposed skill: `/review-prs`, with optional user/repository filters. It should call a deterministic queue command, not let an LLM infer statuses or calculate durations. The same command supplies the Slack digest, so the two outputs agree.

Suggested columns: PR/link, author, area, why the user is an eligible/requested reviewer, current head, bot gate state, actionable waiting time, latest meaningful author change, unresolved human requests, and next action. Separate "your review is needed" from "waiting for the author/bots" and "your PRs need work".

Use the latest readiness/re-request transition for the current review cycle, not PR creation time or generic `updated_at`. Store or reconstruct transition times and pause actionable waiting while the author or bots are responsible. Report missing historical timestamps as unknown. Preserve cumulative wait separately if useful; do not reset all aging just because someone adds a comment or cycles a label.

For a custom digest, configure explicit GitHub-to-Slack identity mapping, timezone and destination, deduplicate by recipient/date/review cycle, and send links with short reasons rather than one notification per polling event. Draft a message preview before enabling any Slack delivery. Daily delivery is an intended feature, not authorization to send a test message now.

## Recommendation and next evidence

Compare Mergify and gitStream using the concrete Platform policy; prefer Mergify provisionally because its hosted protection engine, workflow rules and trusted checks cover more of this expanded request than policy-bot alone. Validate one mixed-area PR, one reviewer-authored PR, one owner-authored PR, bot skip/failure/current-head completion, deferred human requests and stale approvals before adoption. Keep merge queues/automatic merging out of scope unless independently needed.

Whichever engine wins, expect bounded custom work for writer admission and thepastaclaw/CodeRabbit evidence normalization. Reuse native Slack reminders initially if they meet the desired digest; build a richer report only for the confirmed extra fields. The `/review-prs` skill should consume the same state, not become the enforcement authority.

No changes to QE's remote PR until the revised workflow scope and engine are agreed. Preserve the sheet's roles, excluded people, and QuantumExplorer + shumkov fallback throughout.

## Independent review

Feasibility/product-claim and workflow/security reviewers returned CLEAN for research scope. Before adoption, reconcile Mergify's actual installation permissions with the previous proposal's no-contents-write/no-merge-permission constraint; choosing it does not silently authorize wider app access. Engine validation, exact bot completion contracts and admission semantics remain unresolved.
