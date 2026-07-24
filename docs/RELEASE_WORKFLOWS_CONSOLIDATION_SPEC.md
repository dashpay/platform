# Spec: Consolidate Swift & Kotlin SDK releases under the platform release

**v2 — revised after 3-lens review (GHA feasibility, security, simplicity/release-ops).**

## Problem

The monorepo releases with a **single version** (tag `vX.Y.Z[-pre.N]` → GitHub
release → `release.yml` publishes npm packages, Docker images, dashmate
packages). The Swift FFI (`swift-sdk-release.yml`, `ffi-*` tags) and Kotlin SDK
(`kotlin-sdk-release.yml`, `kotlin-sdk-*` tags) release workflows sit outside
this: separate tag namespaces, separate versions, separate GitHub releases.

Facts that shape the solution:

- **Neither `ffi-*` nor `kotlin-sdk-*` tags have ever been pushed** — the
  standalone workflows have never produced a real release, and nothing is on
  Maven Central yet (`org.dashj:dash-sdk-android` 404s). No legacy consumers.
- PR **#4193** (bfoss765) hardens `kotlin-sdk-release.yml` and adds a
  two-job Maven Central publish (unprotected build job + deploy job gated by
  the `maven-central` GitHub environment). All blocking review findings are
  fixed in its current diff. PR **#4220** is the matching admin runbook
  (docs only). The Gradle/jreleaser publish machinery
  (`publishReleasePublicationToStagingRepository`, `jreleaserDeploy`, the
  `verifyJniLibsForRemotePublish`/`verifyStagedAarForRemotePublish` guards,
  `-PsdkVersion`) is **already merged** on `v4.1-dev`.
- The `maven-central` environment exists on `dashpay/platform` but is
  **unconfigured**: no required reviewers, no deployment tag policy, no
  secrets.

## Goal

One release act — publishing the platform GitHub release for `vX.Y.Z[-pre.N]`
— also:

1. builds and attaches the Kotlin release AAR to that same release, and
   publishes `org.dashj:dash-sdk-android:X.Y.Z[-pre.N]` to Maven Central
   (behind the environment approval gate);
2. builds and attaches `DashSDKFFI-<version>.xcframework.zip` + its SwiftPM
   checksum to that same release;
3. retires the separate `ffi-*` / `kotlin-sdk-*` tag namespaces.

## Chosen approach

Follow `release.yml`'s existing pattern of delegating to reusable workflows
(`release-docker-image.yml` precedent).

### 1. `release-kotlin-sdk.yml` (reusable, replaces `kotlin-sdk-release.yml`)

Triggers: `workflow_call` (required `tag` input — the platform tag, e.g.
`v4.1.0-rc.1`) + `workflow_dispatch` (emergency SDK-only re-run). **No other
triggers, ever** (no `pull_request`/`pull_request_target`).

The emergency dispatch path exists **solely** for the ergonomic of re-running
the SDK release without rebuilding four Docker images via `release.yml`'s
dispatch. That path — and only that path — is why #4193's free-form-input
defenses survive. Note (review finding): #4193's ref-resolution logic branches
on `github.event_name == "push"` / reads `github.event.inputs.*`; under
`workflow_call` those are empty/reflect the caller's event, so the logic is
**rewritten, not absorbed verbatim**:

- `workflow_call`: the tag arrives as `inputs.tag` (trusted — comes from
  `github.event.release.tag_name` on a published release). Checkout at
  `refs/tags/<tag>`.
- `workflow_dispatch`: input is free-form → keep #4193's guards: accept only
  a plain tag name or `refs/tags/` form, `gh api`-verify the tag exists,
  hand checkout an explicit `refs/tags/<tag>`. The dispatch **must be started
  at the tag ref in the UI/CLI** — the environment's tag policy matches
  `github.ref`, not the checked-out commit (documented in PUBLISHING.md).

Two-job structure (adapted from #4193):

- **build job** (unprotected; references NO publishing secrets, only
  `github.token`):
  - checkout `refs/tags/<tag>`; resolve built SHA;
  - derive Maven version: strip leading `v` → `4.1.0-rc.1`; validate with
    #4193's whole-string regex (already accepts `-rc.1`-style suffixes);
    reject `-SNAPSHOT`. (#4193's "tag without version form → AAR-only"
    branch is dead under platform tags and is dropped.)
  - build native libs (both ABIs, release profile) + release AAR (existing
    steps unchanged);
  - **tag guard** (#4193): hard-fail if the tag no longer resolves to the
    built SHA; skip-not-overwrite if the asset already exists;
  - **release-exists guard** (review finding): `softprops` *creates* a
    release when none exists — on the dispatch path assert the GitHub
    release for the tag already exists, so a dispatch can never mint one;
  - attach `dash-sdk-android-<version>.aar` to the platform release passing
    **only `tag_name` + `files`** — no `body`, no `name`, no `prerelease`,
    no `generate_release_notes` (any of those would clobber the platform
    release's notes/name/prerelease flag);
  - upload `jniLibs` artifact (7-day retention — no approval pause exists)
    for the deploy job.
- **`maven-central-deploy` job** (`environment: maven-central`,
  `needs: build`):
  - all-or-nothing check of the five `JRELEASER_*` secrets (skip-with-notice
    when none, hard-fail when partial — #4193 verbatim);
  - skip-with-notice when the version is already on Maven Central (repo1
    probe) — Central versions are immutable, so an emergency re-dispatch
    after a successful publish must not fail on the duplicate, while the
    "secrets configured later → re-dispatch" recovery flow still deploys;
  - checkout pinned to the **built SHA** (not the tag — a force-move between
    the jobs must not change what's published), restore `jniLibs`, stage
    signed artifacts (`ORG_GRADLE_PROJECT_signingKey`/`signingPassword`),
    `jreleaserDeploy` with `-PsdkVersion=<version>` — #4193 verbatim.
- Both jobs carry a per-tag `concurrency` group (`cancel-in-progress:
  false`) so a same-tag emergency dispatch can never race the
  release-triggered run past the asset-exists guards.

### 2. `release-swift-sdk.yml` (reusable, replaces `swift-sdk-release.yml`)

Same trigger surface and ref handling as above. Existing build steps
unchanged (Xcode 16, `build_ios.sh --target all --profile release`, zip +
`swift package compute-checksum`). Changes (several from review — the Swift
side must mirror Kotlin's guards):

- checkout pinned to `refs/tags/<tag>` (today's file checks out the default
  ref — latent bug) and resolve built SHA; same tag guard + version regex;
- attach `DashSDKFFI-<version>.xcframework.zip` +
  `DashSDKFFI-<version>.checksum.txt` passing **only `tag_name` + `files`**
  (the current standalone file passes `name:`/`prerelease: false`/`body:` —
  on the platform release that would rename it and un-prerelease it);
- **never overwrite** an already-attached same-named zip (replaces the old
  "skip if checksum unchanged" rule, which overwrote on a changed checksum —
  an immutability hole); the durable home for the consumer checksum is the
  `checksum.txt` asset; the SwiftPM `binaryTarget` snippet goes in the job
  step summary as a convenience copy only.

### 3. `release.yml` additions

```yaml
release-kotlin-sdk:
  name: Release Kotlin SDK
  if: ${{ github.event_name == 'release' }}
  permissions:
    contents: write   # attach assets; workflow-level default is read-only
  uses: ./.github/workflows/release-kotlin-sdk.yml
  with:
    tag: ${{ github.event.release.tag_name }}

release-swift-sdk:
  name: Release Swift SDK FFI
  if: ${{ github.event_name == 'release' }}
  permissions:
    contents: write
  uses: ./.github/workflows/release-swift-sdk.yml
  with:
    tag: ${{ github.event.release.tag_name }}
```

The SDK jobs run **only on the `release: published` event**, not on
`release.yml`'s own dispatch path (review finding): a dispatch runs at a
branch ref — which the environment's `v*` tag policy rejects — and a
dispatched tag may have no published release to attach to. SDK re-runs go
through the reusable workflows' own dispatch, started at the tag ref.

- **Job-level `permissions: contents: write` is mandatory** (review): the
  caller job's permissions cap the called workflow's token; workflow-level
  is `contents: read`, so without this the asset attach 403s. Job-level
  scoping also drops `id-token` from these jobs (they don't need OIDC) and
  leaves the npm/Docker jobs untouched.
- **No `secrets: inherit`** (review, security): environment secrets resolve
  via the deploy job's `environment: maven-central` declaration in the
  same-repo called workflow — `inherit` is not needed for them, and would
  needlessly hand DockerHub/S3-cache/macOS-signing/org secrets to the
  unprotected SDK build jobs. Deliberate divergence from the docker-image
  jobs (which genuinely need those secrets).
- No `needs:` — SDK failures must not block npm/Docker publishing and
  vice-versa.

### 4. Environment + repo configuration (admin, not code)

**Decisions (Ivan, 2026-07-25):** no required reviewers — Maven Central
publishes automatically once the release is published (the deploy job still
declares `environment: maven-central` so the secrets stay environment-scoped);
no repository tag ruleset (out of scope — no changes to general release
logic); SDK jobs run for **all** published releases including dev/alpha.

1. `maven-central` environment: **deployment tag policy `v*`** (restricts
   which refs can access the environment's secrets), no required reviewers.
2. The five secrets as **environment secrets on `maven-central`** only:
   `JRELEASER_MAVENCENTRAL_SONATYPE_USERNAME`,
   `JRELEASER_MAVENCENTRAL_SONATYPE_PASSWORD`, `JRELEASER_GPG_SECRET_KEY`,
   `JRELEASER_GPG_PUBLIC_KEY`, `JRELEASER_GPG_PASSPHRASE`.
   **No repository- or organization-scoped copies** — that would let any
   unprotected job read them.
3. Ivan runs the `gh secret set --env maven-central` commands himself so
   values never enter this transcript; tag policy set via `gh api`.

### 5. Docs & process updates

- Fold #4220's runbook into `packages/kotlin-sdk/PUBLISHING.md`, updated for
  platform tags (incl. the dispatch-at-tag-ref requirement).
- **Update `.claude/skills/release/SKILL.md`** (review must-fix — it's the
  doc a maintainer actually follows on release day): publishing the GitHub
  release now also builds/attaches the AAR + xcframework and queues a
  Maven Central deploy that **someone must go approve** (or reject); note
  where to see the pending deployment.
- Close #4193 and #4220 as superseded, with credit
  (`Co-authored-by: bfoss765`) in the commit message.

### 6. Action pinning (security, while the files are open)

Pin `softprops/action-gh-release` (handles the token that attaches release
assets) to a full commit SHA in both new workflows. Other actions keep their
existing tag pins (broader repo-wide SHA-pinning is out of scope).

## Alternatives rejected

- **Merge #4193 first, then consolidate on top** — two churns of the same
  file, and #4193's `kotlin-sdk-v*` namespace would be dead on arrival.
- **Extra per-SDK tags pushed by the release process** — exactly the drift
  this consolidation exists to kill.
- **Inline the SDK jobs into `release.yml`** — loses the SDK-only re-run
  path and fights the repo's reusable-workflow pattern.

## Failure modes & mitigations

- **SDK build broken at the release tag** (review must-fix — the biggest
  operational gap): PR CI does *not* exercise the release-profile arm64
  Kotlin build, and the Swift build is path-filtered off typical release
  PRs. A break surfaces only after the release is public; the tag is
  immutable, so that version simply ships without the SDK artifact and the
  fix rides the next prerelease. Mitigations: (a) the frequent beta/rc
  cadence itself exercises the SDK release builds continuously — a break is
  caught by the next prerelease, not by a stable; (b) release SKILL.md gains
  a pre-flight note for **stable** releases: confirm the SDK jobs succeeded
  on the preceding rc for the same code. A blocking pre-publish CI gate was
  considered and rejected (a ~3h release-profile build on every release PR).
- **Maven Central deploy is irrevocable** → reviewer gate + SHA-pinned
  deploy checkout + Gradle staging guards.
- **Partial secret configuration** → hard-fail; zero secrets → skip with
  notice (AAR still attaches).
- **Release notes/name/prerelease-flag clobbering** → both SDK jobs pass
  only `tag_name` + `files`.
- **Kotlin build (~up to 3 h) fails/times out** → re-run via the SDK-only
  dispatch path; npm/Docker unaffected.
- **Old draft release published later** → fires `published` for an old tag →
  idempotency guards make it safe (assets skip-not-overwrite; Maven gate
  requires approval).
- **Same-tag re-publish** → concurrency cancels the in-flight run (group is
  per-ref; different tags never cancel each other).

## Scope decisions

- **Which releases run the SDK jobs**: **all** published releases including
  every prerelease (Ivan's decision — single-version story, no special
  cases).
- Maven Central gets every version the SDK jobs run for (prereleases are
  explicit-opt-in for Maven consumers; immutability is fine).
- `kotlin-sdk-nightly.yml`, `kotlin-sdk-build.yml`, `swift-sdk-build.yml`,
  `swift-example-app-ui-smoke.yml` are CI, not release — untouched.
- Committed `Package.swift` keeps its local-path binaryTarget; updating it to
  URL+checksum per release is impossible in the release PR (checksum unknown
  pre-build) — inherited limitation, unchanged.

## Test / verification plan

1. `actionlint` on all changed workflow files.
2. Dry-run against the existing `v4.1.0-rc.1` tag via each reusable
   workflow's dispatch (started **at the tag ref**): AAR + xcframework
   attach to that release; release body/name/prerelease flag unchanged;
   Maven deploy pauses at the gate (skip-with-notice if secrets not yet
   installed; full publish once they are and a reviewer approves).
3. Verify the build job's token saw no publishing secrets (job log audit).
4. Next real prerelease exercises the `release: published` path end-to-end.

## Resolved questions (Ivan, 2026-07-25)

1. SDK jobs run for **all** published releases (incl. dev/alpha).
2. **No required reviewers** — Maven Central publishes automatically.
3. **No tag ruleset** — out of scope, general release logic unchanged.
4. Secrets: Ivan runs the `gh secret set --env maven-central` commands
   himself.
5. #4193/#4220 closed as superseded with bfoss credited (default accepted).
