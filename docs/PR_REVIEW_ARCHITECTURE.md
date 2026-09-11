# PR review architecture

Each repository owns its `.github/pr-review-policy.json`. The shared Python evaluator implements the policy; CODEOWNERS is generated from the same manifest. CODEOWNERS requests native reviews but does not express the complete owner/reviewer distinction. The custom status supplies that distinction alongside existing GitHub protections.

## Ownership and repository boundaries

An owner satisfies the human requirement for their own area, including on their own PR. A reviewer can satisfy another author's requirement. Changes spanning areas must satisfy every affected area. The fallback is QuantumExplorer and shumkov for unmapped paths only. An area with unresolved ownership blocks readiness; it does not silently inherit fallback ownership. Review eligibility is checked against current repository permissions. Automation never grants access or expands a team's privileges.

The repository registry covers:

| Repository | Policy scope |
| --- | --- |
| `dashpay/platform` | Package-specific ownership in the canonical manifest |
| `dashpay/rust-dashcore` | Separate SPV and wallet areas; unresolved ownership stays explicit |
| `dashpay/tenderdash` | Whole-repository owner lklimek and reviewer shumkov |
| `dashpay/grovedb` | Whole-repository owner QuantumExplorer |
| `dashpay/dash-evo-tool` | Whole-repository owner lklimek |

Contributor and QA sheet roles are not automatically promoted to owner or reviewer. Uncertain sheet entries are recorded as activation blockers in the manifests and operations guide. No broad existing team receives write access to satisfy CODEOWNERS.

## Local enforcement

The local event workflow uses trusted default-branch policy and shared engine code. External repository callers pin both the reusable workflow and engine to the same full commit SHA. Pull request code is not executed by the privileged controller.

An author has five active, non-draft admission slots per repository. Admission persists across ordinary updates; lifecycle events invalidate obsolete admission records. More PRs may exist, but excess PRs wait before human review. The controller reconciles affected PRs on events and rotates through small batches on the scheduled sweep.

Before human review, both configured bots must finish on the current head, bot objections must be addressed, and the author must post an unedited `/self-reviewed FULL_HEAD_SHA` comment after those bot outcomes. This is an explicit author attestation, not an automated substitute for inspecting the diff. New commits invalidate the attestation and head-bound approvals.

The evaluator determines actionable reviewers, blockers and readiness from fresh GitHub evidence. The publisher revalidates evidence before writing its status, state comment, label and review requests. Errors block readiness. Existing checks and native protection requirements still apply; owner exemption in this policy does not override an independent GitHub approval requirement. Repository administrators and trusted writers retain their existing authority.

## Combined reporting and Slack

`aggregate.py` collects one read-only snapshot across the registry. Preview entries use local seed policies; active entries read each repository's canonical policy from its current default branch. Missing evidence is reported as unavailable with partial totals. Repository-qualified identities prevent identical PR numbers in different repositories from colliding. Combined author totals above five are warnings; there is no cross-repository hard cap.

`notifications.py` derives the shared channel summary and personal digests from that same snapshot. Personal destinations come from the owner/reviewer roster and explicit GitHub-to-Slack ID mappings. Multiple GitHub identities mapped to one Slack user produce one combined digest. People with no actionable review or author blocker receive no empty digest unless repository evidence is unavailable.

Messages remain previewable when Slack IDs are missing, with null destinations. Delivery rejects any configuration error and requires the explicit enable flag and bot token. Plain-text blocks prevent repository-controlled content from generating Slack mentions. Each destination is attempted once; an uncertain acknowledgement stops later sends. Manual reruns can duplicate acknowledged messages, so inspect delivery receipts before retrying.

The central reporter uses a separate read-only GitHub App token for the selected repositories. Local enforcement uses each repository's own workflow token. Slack delivery starts disabled. See [PR_REVIEW_OPERATIONS.md](PR_REVIEW_OPERATIONS.md) for commands, configuration and activation checks.
