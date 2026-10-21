---
name: prs
description: "Your pull requests and the ones waiting on you, across Platform, rust-dashcore, Tenderdash, GroveDB and Dash Evo Tool: what each is blocked on, how long it has waited, and how many of your five slots per repository are taken. Use for /prs or any request to inspect the review queue."
---

# PRs

Use the shared policy reporter in `dashpay/stale_prs_are_bad`. Clone or locate that repository (`$PR_REVIEW_HOME` if set, else `gh repo clone dashpay/stale_prs_are_bad` into a temporary directory) and read its `guides/PR_REVIEW_ARCHITECTURE.md` for the policy and `guides/PR_REVIEW_OPERATIONS.md` for setup and limitations.

1. Resolve the current GitHub user with `gh api user --jq .login`, unless the user names another reviewer.
2. From that checkout's root run `python3 -m pr_review.aggregate report --user USER`. For structured output add `--format json`; to limit repositories add `--repo dashpay/REPOSITORY`. For one PR run `python3 -m pr_review.main report --repo dashpay/REPOSITORY --pr NUMBER`; the policy comes from that checkout's `policies/`.
3. Present two sections. First, what is waiting on this person to review: oldest first, with links, touched areas and the next action. Second, their own pull requests and what each is blocked on — a bot that has not reported, an unresolved bot thread, a change request, a missing self-review, or a slot that is still queued. Use repository-qualified PR identifiers. Distinguish author action, bot work, missing access, and the five-PR admission queue within each repository. Combined totals above five are workload warnings, not a global hard cap.
4. Waiting time is the current recorded human-review cycle. If no controller state exists, say the age is not recorded. Do not equate total PR age with review waiting time.
5. The report says nothing about CI. It reads this policy's own commit status and never the repository's check runs, so never state or imply that a build passed or failed. If asked about CI, say it is not covered and read the checks directly.

This command is read-only. It does not perform the code review itself. Never post `/self-reviewed`, submit an approval, request reviewers, change labels, or send Slack messages merely because this skill was invoked. Author self-review is a personal attestation after inspecting the current diff and bot outcomes; it cannot be inferred from a bot run.

If any repository evidence is incomplete or inaccessible, name that repository and state that totals are partial. Preview policy rows are computed recommendations, not live enforcement. Report the error. Do not invent readiness or approval. Report preview results as computed policy, not proof that repository enforcement has been activated.
