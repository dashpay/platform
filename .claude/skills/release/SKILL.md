---
name: release
description: Cut a Dash Platform release (stable or prerelease) with `yarn release`. Use when asked to release/publish a new platform version, bump versions, or cut a beta/rc/dev/alpha prerelease (e.g. "release 4.1.0-beta.1").
argument-hint: "[target-version | type]"
---

# Cut a Platform release

`yarn release` (→ `scripts/release/release.sh`) bumps every workspace version, regenerates the changelog, and opens a release PR. It does **not** publish artifacts or tag — a maintainer merges the PR, then publishes a GitHub release, which creates the tag and triggers the asset build (see *Publishing the GitHub release* below).

Full flow: **`yarn release` → fix offset versions on the PR → merge → publish the GitHub release (tags + builds assets)**.

## What one run does

1. Bumps the version in **all npm `package.json` files**, the **root `package.json`**, and the **`[workspace.package]` version in `Cargo.toml`** (all Rust crates inherit it).
2. Runs `cargo metadata` to refresh `Cargo.lock`.
3. Regenerates `CHANGELOG.md` (conventional-changelog, `dash` preset) from the changelog-base tag to HEAD.
4. Creates branch `release_<new-version>`, commits `chore(release): update changelog and version to <new-version>`.
5. Pushes the branch and opens a PR **against the branch you ran it from** (e.g. `v4.1-dev`), body from `scripts/release/pr_description.md`, assigned to a milestone.

## Command

```sh
yarn release -v=<target-version>     # explicit exact version — preferred, unambiguous
yarn release -t=<type>               # derive next version from current + type
yarn release -c=<tag>                # override the changelog-from tag (optional)
```

- `-v` / `--version` — set the exact version (validated as semver). The type is inferred from the prerelease id. **Prefer this** — no guessing.
- `-t` / `--type` — `release`, or a prerelease id: `dev`, `alpha`, `beta`, `rc` (any id works). Version is derived from the current version (see rules below).
- `-c` / `--changelog-from` — tag to build the changelog from. Omit to auto-detect via `find_latest_tag.js`.

### `-t` derivation rules (from `bump_version.js`, when `-v` is not given)

Given current root version and its type:

- release → same type `release`: **patch** bump (`4.0.0` → `4.0.1`).
- release → a prerelease type: **minor** bump + `.1` (`4.0.0` `-t=beta` → `4.1.0-beta.1`).
- prerelease → `release`: **minor** bump, drop prerelease (`4.1.0-beta.3` `-t=release` → `4.1.0`).
- prerelease → different prerelease type: retarget `X.Y.0-<newtype>.1` (`4.1.0-dev.5` `-t=beta` → `4.1.0-beta.1`).
- prerelease → same prerelease type: **prerelease** bump (`4.1.0-beta.1` `-t=beta` → `4.1.0-beta.2`).

**Milestone:** `release` → `vX.Y.x`; prerelease → `vX.Y.0`.

## Preconditions — check before running

1. **On the right base branch.** The PR targets the current branch. For 4.1 work that's `v4.1-dev`. `git branch --show-current`.
2. **Clean working tree.** The script aborts if `git status -su` is non-empty. Stash or set aside unrelated changes first — do **not** sweep them into the release commit. (`.serena/project.yml` is a common stray; confirm with the user how to handle it, don't just commit it.)
3. **`gh` authenticated** (`gh auth status`) — the script calls `gh auth login` interactively otherwise.
4. **`cargo` available** — the script runs `cargo metadata`.

## Steps

1. Confirm the target version and base branch with the user, and resolve the version the params will produce (dry-check with the derivation rules above).
2. Verify all preconditions. If the tree is dirty, surface it and ask how to handle — never fold stray files into the release.
3. Run `yarn release -v=<target-version>` (this pushes a branch and opens a PR — an outward-facing action; confirm before running unless already told to proceed).
4. Report back: new version, the `release_<version>` branch, the changelog-from tag used, and the PR URL. Verify the bump landed in `package.json` + `Cargo.toml` and that the versions match.

**Changelog base tag.** `find_latest_tag.js` resolves the tag the changelog is generated from and `release.sh` prints it (`Changelog base tag : <tag>`) for verification before generating — an interactive run also pauses for confirmation. The base must be the **immediately-preceding release** (by creation date); a too-far-back base regenerates already-present `CHANGELOG.md` sections and duplicates them. The resolver bases a first prerelease of a new id (e.g. the first `rc` after betas) off the newest prerelease on that `X.Y.0` line, and warns if existing tags would be duplicated. If the base is ever wrong, abort and re-run with `-c=<correct-tag>`.

## First 4.1 beta example

Current version `4.0.0` on branch `v4.1-dev`, cutting the first 4.1 beta:

```sh
yarn release -v=4.1.0-beta.1
```

`-t=beta` yields the same result here (release→beta = minor bump + `.1`), but `-v` is explicit. Changelog auto-detects `v4.0.0` as the base (no prior `v4.1.0-beta.*` tags exist yet), so `-c` is unnecessary.

## Known gotcha: offset package versions (fix on the PR)

The bump sets **every** npm package to the same version, but `yarn.config.cjs` requires three packages to lead the root major by a fixed offset: `@dashevo/dash-spv` (+1), `dash` / js-dash-sdk (+3), `@dashevo/wallet-lib` (+7). The release script does **not** apply these, so the **JS dependency versions check** fails on the release PR. Fix on the release branch:

```sh
yarn constraints --fix
```

then commit (`chore(release): restore offset major versions via yarn constraints`) and push. E.g. for `4.1.0-beta.1`: dash-spv `5.1.0-beta.1`, dash `7.1.0-beta.1`, wallet-lib `11.1.0-beta.1`.

Note also that the heavy E2E suites (`test:browsers`, `test:suite`, functional, dashmate E2E) and the Swift SDK build often only run once the Docker images build, and can be red for **environmental** reasons (devnet bring-up, self-hosted-runner Keychain perms) unrelated to the release. Branch protection has **0 required status checks**, so these don't block the merge — only human review does.

## Publishing the GitHub release (after the PR merges)

Tagging is a **separate manual step after the release PR is merged** into the dev branch. Publishing a GitHub **release** (with the version tag) is what triggers the asset pipeline: `.github/workflows/release.yml` runs `on: release: published` and builds + attaches the dashmate packages, Docker images, the Kotlin SDK AAR (`dash-sdk-android-<version>.aar`, also published to Maven Central as `org.dashj:dash-sdk-android:<version>` via the `maven-central` environment) and the Swift `DashSDKFFI-<version>.xcframework.zip` + checksum.

1. Confirm the release PR is merged and the dev-branch tip carries the version bump.
2. Write concise, **non-technical** release notes: a short highlights list (main features) + a bug-fixes/hardening summary + a link to the changelog. Summarize from the version's `CHANGELOG.md` section (its `### Features` / `### Bug Fixes` blocks) — don't paste the raw commit list.
3. Create the release, pinned to the merge commit, marked `--prerelease` for `-beta`/`-rc`/`-dev`/`-alpha` (drop it for a stable release). This is outward-facing and triggers builds — confirm the notes with the user first.

```sh
gh release create v<version> --target <merge-commit-sha> \
  --title "Dash Platform v<version>" --prerelease \
  --notes-file <notes.md>
```

Changelog link for the notes: `https://github.com/dashpay/platform/blob/v<version>/CHANGELOG.md` (or the compare URL `.../compare/v<prev>...v<version>`).

4. Publishing fires `release.yml` — verify the release build succeeds and the dashmate / Docker assets attach to the release. The Kotlin and Swift SDK jobs run in the same workflow but take longer (the Kotlin native build can run ~3 h) — the release page fills in as they finish. The Maven Central deploy runs automatically after the Kotlin build (no approval gate); note that PR CI does **not** exercise the release-profile arm64 SDK builds, so an SDK build break surfaces here first — for a stable release, confirm the SDK jobs were green on the preceding rc. To re-run just one SDK release: `gh workflow run release-kotlin-sdk.yml --ref v<version> -f tag=v<version>` (same for `release-swift-sdk.yml`) — must be dispatched **at the tag ref**, with the **plain tag name** as input (no `refs/tags/` prefix). **Confirm with the release owner immediately before dispatching**: the re-run attaches public release assets and the Kotlin one can publish an irrevocable Maven Central version (no approval gate). Works only for tags that already contain these workflow files (post-consolidation); for older tags, dispatch at the dev branch — assets attach, Maven deploy skips — and use the manual runbook in `packages/kotlin-sdk/PUBLISHING.md` for Maven.

## Known gotcha: a new npm package without a trusted publisher breaks the publish

`release.yml`'s **Release NPM packages** job publishes with `yarn workspaces foreach --all --no-private --parallel npm publish --tolerate-republish --access public` over **npm trusted-publishers OIDC**. OIDC can only publish to a package that has a **trusted publisher configured** on npm for it. Two ways a new `@dashevo/*` package trips this:

- **Never published** — the package does not exist on npm; OIDC cannot bootstrap it (`404`).
- **Exists but has no trusted publisher** — e.g. it was first-published manually with a token. The package exists, but OIDC has nothing to authenticate against, so publishing a *new version* fails. This is the subtle one: a one-off manual publish is **not** a permanent fix — the publish job keeps failing on that package **every** release until the trusted publisher is configured.

`--access public` and `--tolerate-republish` are already set — they are **not** the fix.

**Symptom:** the run's "Release NPM packages" job fails; `yarn` reports `The command failed in workspace @dashevo/<pkg> ... exit code 1`. The per-package error is `YN0033: No authentication configured for request` (no trusted publisher) or a `404`/`E404` (never published). Confirm which package and how it was last published:

```sh
# what published the latest version — GitHub Actions (OIDC) has a trustedPublisher field; a
# human username means it was a manual token publish and OIDC is NOT wired up:
curl -s https://registry.npmjs.org/@dashevo/<pkg>/<version> | python3 -c "import sys,json;print(json.load(sys.stdin).get('_npmUser'))"
```

**Permanent fix (a human with owner rights on the package, via the npm web UI):** configure the trusted publisher, then re-run the failed job.

**Bootstrap first if the package was _never published_.** Trusted-publisher configuration needs an existing package page — a `404` package has no **Access** settings to configure. So you must create the package before you can wire up OIDC:

0. Publish one version with a token (`npm publish --access public` from the package dir, or the Stopgap command below). This creates the package on npm.

Then, for **both** cases (never-published-now-bootstrapped, and exists-without-a-publisher):

1. npmjs.com → the package → **Access** → **Trusted Publisher** → **GitHub Actions**: organization/user `dashpay`, repository `platform`, workflow filename `release.yml`, environment **blank** (the `release-npm` job uses no environment). Save.
2. Re-run the failed job — OIDC now publishes the missing version and `--tolerate-republish` skips the rest → green:

```sh
gh run rerun <release-run-id> --repo dashpay/platform --failed
```

**Stopgap only (does not stop the recurrence):** publish the exact release version once with a token, then re-run the failed job (`--tolerate-republish` skips it). The dist-tag matches CI's — prerelease `<major>.<minor>-<suffix>` (e.g. `4.1-rc`), stable `latest` (`npm view @dashevo/dpns-contract dist-tags`). This unblocks the current release but the next one fails the same way until the trusted publisher is configured.

**Prevention:** whenever you add a new publishable npm package, bootstrap it **before** cutting the release that ships it — publish one version with a token so the package exists, then configure its npm trusted publisher (org `dashpay`, repo `platform`, workflow `release.yml`) on the freshly created package. The publisher cannot be configured until the package exists, so the token publish is the required first step, not an optional stopgap. `release.sh` prints this reminder after opening the release PR.

## Post-stable-release: graduate the dev branch

After a **stable** `vX.Y.0` release PR merges into `vX.Y-dev`, promote the branches so `vX.Y-dev` becomes the `X.Y.x` patch line and the next dev line takes over. Only do this for a **stable** release, not prereleases. (Example below is the 4.1.0 run: current dev `v4.1-dev`, next dev `v4.2-dev`. If the next release is a **major** bump, the user says so — e.g. `v5.0-dev`.)

**Read first — the ruleset, not classic protection, is what blocks direct pushes.** `master`, `main`, `develop`, and every `v*` branch are covered by a repo **ruleset** (`Block deletions, require PR`, `id 501248`) that requires a PR and is **separate** from classic branch protection — a repo admin does **not** automatically bypass it, and `gh api repos/.../branches/<b>/protection` does **not** show it. Only users in the ruleset's `bypass_actors` can push directly (currently shumkov `24296` and QuantumExplorer `11468583`, both `always`). To grant a user standing bypass, fetch the ruleset, append them, and PUT it back:

```sh
gh api repos/dashpay/platform/rulesets/501248 > r.json
jq '{name,target,enforcement,bypass_actors:(.bypass_actors+[{actor_id:<userId>,actor_type:"User",bypass_mode:"always"}]),conditions,rules}' r.json > put.json
gh api --method PUT repos/dashpay/platform/rulesets/501248 --input put.json   # find <userId>: gh api users/<login> --jq .id
```

Steps (skip whatever is already done — e.g. the next dev branch and default may have been set early):

1. **Ensure the next dev branch exists** — `v(X.Y+1)-dev`. If it exists but is behind current dev, it just needs the sync in step 3.
2. **Merge current dev → `master`** (master tracks the latest stable). master is usually far behind and its only unique commits are **auto-generated** (e.g. `.github/grpc-queries-cache.json`, committed by `tests-rs-sdk-grpc-coverage.yml` as `chore: update gRPC queries cache [skip ci]`) — disposable. **Merge, don't force-reset** (master sets `allow_force_pushes:false`, and `required_linear_history:false` so a merge commit is allowed); resolve the generated cache toward the dev branch:
   ```sh
   git checkout master && git reset --hard origin/master
   git merge origin/vX.Y-dev                                    # conflicts only on generated caches
   git checkout --theirs .github/grpc-queries-cache.json && git add -A
   git commit --no-edit && git push origin master
   ```
3. **Fast-forward next dev to current dev** (carry the release commits into the next line). It is typically strictly behind → a clean fast-forward:
   ```sh
   git push origin origin/vX.Y-dev:v(X.Y+1)-dev
   ```
   The next dev branch **stays at version `X.Y.0`** until its first `v(X.Y+1).0-dev.1` release is cut — do **not** bump it now.
4. **Make next dev the default branch** if it isn't already (repo Settings → Branches, or `gh api`).
5. **Milestones** — ensure `vX.Y.x` (patches for the just-released line) and `v(X.Y+1).0` (next dev prereleases) exist. Create with `gh api repos/dashpay/platform/milestones -f title="vX.Y.x" -f state=open`. (Stable-release milestone convention is `vX.Y.x`; prerelease is `vX.Y.0`.)
6. **Re-target open PRs** from current dev → next dev: `gh pr edit <n> --base v(X.Y+1)-dev`. Leave genuine `vX.Y.x` patch PRs on `vX.Y-dev`. Note the `milestone.yml` workflow auto-assigns a PR's milestone by its base branch.
