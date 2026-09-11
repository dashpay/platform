# Review policy across the responsibility sheet

Status: scope decision and extension proposal, 2026-09-11. The existing build operates only on dashpay/platform. No other repository or Slack destination has been changed.

## Repository map

| Sheet column | Repository | Actual scope | Visible owner/reviewer assignments |
| --- | --- | --- | --- |
| Platform crates and SDKs | dashpay/platform | Existing packages/* policy | Existing reviewed manifest |
| Tenderdash | dashpay/tenderdash | Repository; default v1.8-dev | Lukasz (lklimek) owner; Ivan (shumkov) reviewer; Sam contributor only |
| GroveDB | dashpay/grovedb | Repository; default develop | Sam (QuantumExplorer) owner; Ivan contributor only |
| DET | dashpay/dash-evo-tool | Dash Evo Tool; default v1.0-dev | Lukasz (lklimek) owner; Thephez QA; Sam/Ivan/Pasta contributors |
| dash-spv | dashpay/rust-dashcore | dash-spv/ exists on default dev | Borja and Kevin marked reviewers; no visible owner |
| key-wallet/mana… | dashpay/rust-dashcore | key-wallet/ and key-wallet-manager/ exist on default dev | Sam, Borja and Kevin marked reviewers; no visible owner |
| protocol/r… (cropped) | Candidate: dashpay/rust-dashcore | dash/ is its dashcore crate; exact sheet label/scope needs confirmation | Do not assign from a cropped column |

The rust-dashcore workspace also includes dash-spv-ffi/, key-wallet-ffi/, rpc-client/, rpc-json/, hashes/, internals/, dash-network/ and supporting crates. Their presence does not establish that the sheet assigns them to an adjacent area. Map confirmed areas literally; apply the approved QuantumExplorer + shumkov fallback only to unmapped paths, subject to verified permissions in each repository. Missing owners inside a named area remain a configuration gap, not an invitation to promote contributors or reviewers.

Keep strophy and Silvanassss excluded everywhere. A display name in the original sheet does not override that exclusion. Verify Borja/Kevin account identities before translating the external columns into live configuration; do not restore an excluded account through an alias. Do not give existing broad teams write access.

## Chosen organization

Use five repository configurations and one shared evaluator implementation. Each repository owns its literal path map, target branches, required policy status, self-review/bot gates, and reviewer requests. Keep credentials for review/status mutations local to that repository. Share versioned evaluator code at a pinned revision; do not copy five independently maintained implementations. Retain current protection settings until each repository passes its own activation checks.

Aggregate reporting in one scheduled workflow, initially hosted in Platform. Collect a fresh read-only snapshot from all five configured repositories, then produce one DM per distinct owner/reviewer across the whole set and one shared channel summary. Include repo-qualified PR identifiers, personal actionable reviews and age, author blockers, and aggregate queue counts. Extend /review-prs to consume the same reporter, with an optional repository filter. Do not send one DM per repository. A failed repository read must be shown as unavailable/incomplete, never silently counted as an empty queue.

The collector requires a GitHub App installed only on these selected repositories with read permissions sufficient for PRs, issue comments/timelines, policy contents and collaborator metadata. It does not need merge, contents-write or repository-administration permission. Generate short-lived installation tokens inside Actions; no separately hosted service is needed. The ordinary GITHUB_TOKEN is limited to its workflow repository. Slack personal delivery uses a bot token with chat:write and an explicit GitHub-login-to-Slack-user-ID mapping, plus the configured channel ID. No email-directory lookup or guessed identities are needed. Validate the exact Slack app scopes and destination access before activation.

Keep the implemented hard admission limit at five non-draft PRs per author per repository for the initial rollout. The combined report must also show the author's total of open non-draft PRs targeting each repository's configured branches across all five repositories and flag totals above five as a workload warning. This is an explicit limitation: it does not enforce a global five-PR cap. A hard global cap would require shared admission state/serialization and cross-repository revocation; do not silently introduce that coupling or describe five-per-repository as five overall. Global enforcement is a separate policy decision if the workload warning proves insufficient.

## Current repository differences

Read-only baseline checked on 2026-09-11; refresh immediately before changing settings:

- rust-dashcore/dev: no CODEOWNERS found in supported locations; classic protection requires two approvals. Existing Ready for Review Label automation uses a CodeRabbit approval and CI, with no current-head binding in the reviewed approval filter. Reconcile that automation with the new label/state logic; do not have two independent writers disagree about readiness.
- Tenderdash/v1.8-dev: .github/CODEOWNERS currently lists QuantumExplorer and lklimek, whereas the sheet makes Sam a contributor and Ivan a reviewer. Automatic CodeRabbit reviews are disabled in its configuration. Resolve that producer configuration before requiring both bots; do not silently weaken the agreed gate.
- GroveDB/develop: root CODEOWNERS lists QuantumExplorer. Effective rules include one approval plus a code-owner requirement; the ruleset adds requirements beyond classic protection. Preserve them until an approved replacement exists.
- Dash Evo Tool/v1.0-dev: no CODEOWNERS found; a ruleset requires a PR with zero approvals plus Clippy/TestSuite checks. Introducing human area requirements changes its current merge behavior.

Recent sampled reviews show both bots in Tenderdash and Dash Evo Tool, and CodeRabbit in sampled rust-dashcore/GroveDB PRs. These samples do not prove consistent current-head coverage or that an unsampled bot is absent. Confirm producer settings and receipts for each repository before activation. Existing administrator/ruleset bypass authority also requires review; a shared status does not automatically remove it.

## Alternatives rejected

- Treating every sheet column as a repository: dash-spv and key-wallet share rust-dashcore and need crate-level routing there.
- Extending Platform CODEOWNERS with external paths: it cannot govern files or PRs in another repository.
- Five independent Slack schedules: would send duplicate personal digests and inconsistent summaries.
- One central writer with broad repository access: unnecessary for combined reporting and increases the impact of a credential failure.
- Copying Platform's bot gate blindly: each repository must demonstrate both configured producers and their current-head completion receipts before enabling that gate.
- Adding similarly named repositories automatically: dash-shared-core and archived rust-dashcore-rpc are not separate rollout targets identified by this sheet; neither are rs-tenderdash-abci or GroveDB helper repositories without an explicit sheet mapping.

## Rollout and remaining evidence

1. Finish Platform's personal-DM/shared-summary extension and reusable reporter design before publishing enforcement elsewhere.
2. Prepare repository-local policy PRs for Tenderdash, GroveDB and Dash Evo Tool from the clear sheet columns. Audit their current CODEOWNERS precedence, access, protected target branches and both bots; preserve existing protections until replacement checks work.
3. Prepare rust-dashcore's area map separately. Resolve the missing SPV/wallet owners, cropped protocol scope and external display-name mappings before activation. Do not treat all of rust-dashcore as one ownership area.
4. Review the extension spec and code independently, then enable combined reports in preview. Configure Slack recipients/channel and opt into delivery only after checking the exact messages and aggregate completeness.

Tests must cover duplicate people/PR numbers across repositories, one DM per person, partial collector failures, unmapped Slack identities, absent bot installations, independent per-repo permissions, and truthful per-repository versus combined workload counts. No remote mutation is authorized merely by identifying repositories.

## Evidence

- Source responsibility screenshot: .orca/drops/Screenshot 2026-09-10 at 16.45.09.png. Its final column is visibly cropped.
- Platform Cargo.toml lines defining dashcore, dash-spv, key-wallet and key-wallet-manager all use https://github.com/dashpay/rust-dashcore.
- Live rust-dashcore workspace: https://github.com/dashpay/rust-dashcore/blob/dev/Cargo.toml .
- DET identity: https://github.com/dashpay/dash-evo-tool/blob/v1.0-dev/README.md .
- Repository metadata and root trees fetched read-only for all four external repositories on 2026-09-11.
- GitHub token scope: https://docs.github.com/en/actions/concepts/security/github_token .
- Cross-repository app authentication within Actions: https://docs.github.com/en/apps/creating-github-apps/authenticating-with-a-github-app/making-authenticated-api-requests-with-a-github-app-in-a-github-actions-workflow .
- Slack direct-message API: https://docs.slack.dev/reference/methods/chat.postmessage .

Independent scope review found no must-fixes; its count-scope clarification is incorporated. A separate permissions/workflow review supplies the live-baseline differences above.
