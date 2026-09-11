---
name: review-prs
description: "Show Platform PRs awaiting your review, their waiting time, and blockers on your own PRs. Use for /review-prs or requests to inspect the human review queue."
---

# Review PRs

Use the repository's deterministic policy reporter. Read `docs/CODEOWNERS_PR4449_SPEC.md` for the policy and `docs/PR_REVIEW_OPERATIONS.md` for setup and limitations.

1. Resolve the current GitHub user with `gh api user --jq .login`, unless the user names another reviewer.
2. From the repository root run `python3 -m scripts.pr_review.main report --user USER`. For structured output add `--format json`; to inspect a particular PR add `--pr NUMBER`.
3. Present actionable human reviews first, oldest first, with links, touched areas, and the next action. Separately summarize blockers on the user's own PRs. Distinguish author action, bot work, missing access, and the five-PR admission queue.
4. Waiting time is the current recorded human-review cycle. If no controller state exists, say the age is not recorded. Do not equate total PR age with review waiting time.

This command is read-only. It does not perform the code review itself. Never post `/self-reviewed`, submit an approval, request reviewers, change labels, or send Slack messages merely because this skill was invoked. Author self-review is a personal attestation after inspecting the current diff and bot outcomes; it cannot be inferred from a bot run.

If GitHub evidence is incomplete or inaccessible, report the error. Do not invent readiness or approval. Report preview results as computed policy, not proof that repository enforcement has been activated.
