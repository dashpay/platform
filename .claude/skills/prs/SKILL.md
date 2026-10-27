---
name: prs
description: "Your pull requests and the ones waiting on you, across Platform, rust-dashcore, Tenderdash, GroveDB and Dash Evo Tool: what each is blocked on, how long it has waited, and how many of your five slots per repository are taken. Use for /prs or any request to inspect the review queue."
---

# PRs

Use the shared policy reporter in `dashpay/stale_prs_are_bad`. Clone or locate that repository (`$PR_REVIEW_HOME` if set, else `gh repo clone dashpay/stale_prs_are_bad` into a temporary directory) and read its `guides/PR_REVIEW_ARCHITECTURE.md` for the policy and `guides/PR_REVIEW_OPERATIONS.md` for setup and limitations.

1. Resolve the current GitHub user with `gh api user --jq .login`, unless the user names another reviewer.
2. From that checkout's root run `python3 -m pr_review.aggregate report --user USER`. For structured output add `--format json`; to limit repositories add `--repo dashpay/REPOSITORY`. For one PR run `python3 -m pr_review.main report --repo dashpay/REPOSITORY --pr NUMBER`; the policy comes from that checkout's `policies/`.
3. Present two sections. First, what is waiting on this person to review: oldest first, with links, their part and the next action. Their part is what the report prints after `your part:` — each area they may approve that nobody has approved on the current head, with who else may approve it instead (one approval per area is enough), and `re-review or resolve your objection` where an objection of theirs is still open. From `--format json`, take the `approvals` entries with `owned` false, empty `approved_by` and the person in `approvers` (case-insensitive), plus a re-review when they are in `objectors`. Second, their own pull requests and what each is blocked on — a bot that has not reported, an unresolved bot thread, a change request, a missing self-review, or a slot that is still queued. Use repository-qualified PR identifiers. Distinguish author action, bot work, missing access, and the five-PR admission queue within each repository. Combined totals above five are workload warnings, not a global hard cap.
4. Waiting time is the current recorded human-review cycle. If no controller state exists, say the age is not recorded. Do not equate total PR age with review waiting time.
5. The report knows the build only as one verdict for the current head — green, running or failed, from its check runs and statuses — which gates the review request. State it only as the row's blockers do; to see which check failed, read the checks directly.

This command is read-only. It does not perform the code review itself. Never post `/self-reviewed`, submit an approval, request reviewers, change labels, or send Slack messages merely because this skill was invoked. Author self-review is a personal attestation after inspecting the current diff and bot outcomes; it cannot be inferred from a bot run.

If any repository evidence is incomplete or inaccessible, name that repository and state that totals are partial. Preview policy rows are computed recommendations, not live enforcement. Report the error. Do not invent readiness or approval. Report preview results as computed policy, not proof that repository enforcement has been activated.
