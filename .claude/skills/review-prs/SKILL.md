---
name: review-prs
description: "Show PRs across Platform, rust-dashcore, Tenderdash, GroveDB and Dash Evo Tool awaiting your review, their waiting time, and blockers on your own PRs. Use for /review-prs or requests to inspect the human review queue."
---

# Review PRs

Use the repository's deterministic policy reporter. Read `docs/PR_REVIEW_ARCHITECTURE.md` for the policy and `docs/PR_REVIEW_OPERATIONS.md` for setup and limitations.

1. Resolve the current GitHub user with `gh api user --jq .login`, unless the user names another reviewer.
2. From the repository root run `python3 -m scripts.pr_review.aggregate report --user USER`. For structured output add `--format json`; to limit repositories add `--repo dashpay/REPOSITORY`. For one PR run `python3 -m scripts.pr_review.main report --repo dashpay/REPOSITORY --repository-root /path/to/target-checkout --pr NUMBER` from the Platform/shared engine checkout; the target checkout supplies its canonical policy.
3. Present actionable human reviews first, oldest first, with links, touched areas, and the next action. Separately summarize blockers on the user's own PRs. Use repository-qualified PR identifiers. Distinguish author action, bot work, missing access, and the five-PR admission queue within each repository. Combined totals above five are workload warnings, not a global hard cap.
4. Waiting time is the current recorded human-review cycle. If no controller state exists, say the age is not recorded. Do not equate total PR age with review waiting time.

This command is read-only. It does not perform the code review itself. Never post `/self-reviewed`, submit an approval, request reviewers, change labels, or send Slack messages merely because this skill was invoked. Author self-review is a personal attestation after inspecting the current diff and bot outcomes; it cannot be inferred from a bot run.

If any repository evidence is incomplete or inaccessible, name that repository and state that totals are partial. Preview policy rows are computed recommendations, not live enforcement. Report the error. Do not invent readiness or approval. Report preview results as computed policy, not proof that repository enforcement has been activated.
