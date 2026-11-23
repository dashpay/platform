---
name: release
description: Cut a Dash Platform release (stable or prerelease) from start to finish - the `yarn release` PR, its merge, the GitHub release, the release.yml assets, signing the dashmate SHA256SUMS, and master after a stable release. Use when asked to release/publish a new platform version, bump versions, or cut a beta/rc/dev/alpha prerelease (e.g. "release 4.1.0-beta.1").
argument-hint: "[target-version | type]"
---

# Cut a Platform release

A release goes through the seven steps of **Release flow** in order. A **stable** release is a final version (`5.1.0`, `5.0.4`); a **prerelease** carries an id (`5.1.0-dev.3`, `-alpha`, `-beta`, `-rc`). Each step says where the two differ. The sections after the flow are reference for the steps.

**Every write to GitHub needs the user's explicit go immediately before it:** pushing a branch, opening or merging a PR, publishing the release, setting or deleting a repository variable, dispatching or re-running a workflow, uploading or deleting a release asset, and pushing `master`. Being asked to "release X" is not that go.

## Release flow

### 1. Cut the release PR

1. Confirm the target version and base branch with the user, and resolve the version the params will produce (dry-check with the [derivation rules](#-t-derivation-rules-from-bump_versionjs-when--v-is-not-given)).
2. **Stable only:** work out now whether it will be the newest stable version. Steps 3 and 6 depend on it.
   ```sh
   git fetch origin --tags
   git tag -l 'v*' --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -n 1
   ```
   It is the newest stable version when it sorts above the tag this prints. A `5.0.4` cut after `5.1.0` is not. Run the same check again right before step 3 and step 6, since another release may come out in between.
3. Verify the [preconditions](#preconditions--check-before-running). If the tree is dirty, surface it and ask how to handle it; never fold stray files into the release.
4. Run `yarn release -v=<target-version>`, only after the user's go: it pushes `release_<version>` and opens a PR against the branch you ran it from.
5. Fix the [offset package versions](#known-gotcha-offset-package-versions-fix-on-the-pr) on the release branch and push.
6. Report the new version, the `release_<version>` branch, the changelog-from tag used and the PR URL. Check that the bump landed in `package.json` and `Cargo.toml` and that the versions match.

### 2. Merge the release PR

The `vX.Y-dev` branches require the `PR Hygiene` check and no approvals. The heavy E2E suites (`test:browsers`, `test:suite`, functional, dashmate E2E) and the Swift SDK build often only run once the Docker images build, and can be red for **environmental** reasons (devnet bring-up, self-hosted-runner Keychain perms) unrelated to the release. Report their state; merging is the user's decision. Note the merge commit: the release is created from it.

### 3. Publish the GitHub release

Publishing a GitHub **release** creates the tag and starts `.github/workflows/release.yml` (`on: release: published`). It builds and attaches the dashmate packages and `SHA256SUMS`, the Docker images, the Kotlin SDK AAR (`dash-sdk-android-<version>.aar`, also published to Maven Central as `org.dashj:dash-sdk-android:<version>` via the `maven-central` environment) and the Swift `DashSDKFFI-<version>.xcframework.zip` with its checksum.

1. Confirm the release PR is merged and the dev-branch tip carries the version bump.
2. Write concise, **non-technical** release notes: a short highlights list (main features), a bug-fixes/hardening summary, and a link to the changelog. Summarize from the version's `CHANGELOG.md` section (its `### Features` / `### Bug Fixes` blocks); don't paste the raw commit list. Changelog link: `https://github.com/dashpay/platform/blob/v<version>/CHANGELOG.md` (or the compare URL `.../compare/v<prev>...v<version>`).
   - **Upgrade notices are not in the changelog.** `CHANGELOG.md` is regenerated from commit titles, so what operators and app developers must do lives in the book and has to be carried into the notes' **Upgrading** section by hand. For a release line that brings a new protocol version, check the book for an "Updating Apps for Protocol Version N" chapter and state its requirement under **Upgrading → App developers**, with a link. For 5.1 (protocol version 15) that is `book/src/sdk/protocol-version-15-upgrade.md`: wallets and apps must update to a 5.1 SDK before protocol version 15 activates, because a 5.0.x client can no longer read DPNS, DashPay, contracts created at protocol version 15, document history or 128-bit sums. Include it in every 5.1 prerelease's notes too, since testnet activates first.
3. Create the release from the merge commit. Confirm the notes and the command with the user first.

```sh
gh release create v<version> --target <merge-commit-sha> \
  --title "Dash Platform v<version>" --notes-file <notes.md> <marker>
```

| Release | `<marker>` |
|---|---|
| Prerelease | `--prerelease` |
| Stable, the newest stable version (step 1) | `--latest` |
| Stable on an older line (e.g. `5.0.4` after `5.1.0`) | `--latest=false` |

GitHub marks a stable release "Latest" by default, and the install docs download the deb from the latest release, so a patch on an older line must say `--latest=false`. Tell the user before publishing such a patch that `release.yml` still moves npm's `latest` dist-tag and the Docker `latest` image to it.

### 4. Wait for `release.yml` and check the assets

The run takes hours: the Kotlin native build alone can run about 3 h. Wait in the background rather than blocking, for example:

```sh
gh run list --repo dashpay/platform --workflow release.yml --event release --branch v<version> \
  --json databaseId,status,conclusion,url
gh run watch <run-id> --repo dashpay/platform --exit-status --interval 60
```

When it has finished, check that the release carries the dashmate packages and `SHA256SUMS`, and that the `Release Dashmate checksums` job is green:

```sh
gh release view v<version> --repo dashpay/platform --json assets --jq '.assets[].name'
```

Expect two debs, `dashmate_5.1.0-1_amd64.deb` and `_arm64.deb` (for a prerelease the `-` before its id becomes `.`: `dashmate_5.1.0.beta.1-1_amd64.deb`), `dashmate-v<version>-<sha>-linux-x64.tar.gz` and `-linux-arm64.tar.gz`, `-x64.exe`, `-x64.pkg` and `-arm64.pkg`, and `SHA256SUMS`. For a stable release, check which release is now Latest: `gh api repos/dashpay/platform/releases/latest --jq .tag_name`. A failed job is re-run only with the user's go.

The Kotlin and Swift SDK jobs fill the release page in as they finish. The Maven Central deploy runs automatically after the Kotlin build (no approval gate); note that PR CI does **not** exercise the release-profile arm64 SDK builds, so an SDK build break surfaces here first — for a stable release, confirm the SDK jobs were green on the preceding rc. To re-run just one SDK release: `gh workflow run release-kotlin-sdk.yml --ref v<version> -f tag=v<version>` (same for `release-swift-sdk.yml`) — must be dispatched **at the tag ref**, with the **plain tag name** as input (no `refs/tags/` prefix). **Confirm with the release owner immediately before dispatching**: the re-run attaches public release assets and the Kotlin one can publish an irrevocable Maven Central version (no approval gate). Works only for tags that already contain these workflow files (post-consolidation); for older tags, dispatch at the dev branch — assets attach, Maven deploy skips — and use the manual runbook in `packages/kotlin-sdk/PUBLISHING.md` for Maven.

### 5. Sign `SHA256SUMS`

`SHA256SUMS` lists the dashmate packages only, not the SDK assets. The apt repository's `stable` suite takes a release only with a maintainer's detached signature of it, `SHA256SUMS.asc`.

- **Stable:** signing is required.
- **Prerelease:** ask the maintainer whether to sign it.

One signature per release is enough. The keys that may sign are listed, by primary key fingerprint, in `ALLOWED_SIGNERS` at the top of `scripts/release/sign_dashmate_checksums.sh`, and the script refuses any other: today Ivan Shumkov's, with QuantumExplorer's added once his key exists. The apt publisher accepts signatures from these keys only.

`scripts/release/sign_dashmate_checksums.sh` runs where the maintainer's key is, and gpg asks the maintainer for the passphrase. Run it there, or hand the maintainer the command. It refuses until every `release.yml` run of the tag has finished, so sign after step 4. Don't re-run any job of that run until the signature is attached: a release job checks for a signature before it uploads, so one that is already running can still replace packages. It checks that `SHA256SUMS` is the asset the run's `Release Dashmate checksums` job attached, by the record that job keeps with the run, downloads the packages and the file into a new empty directory, checks every line, signs the downloaded file and verifies the signature. Without `--upload` it stops there and prints the upload command:

```sh
scripts/release/sign_dashmate_checksums.sh --tag=v<version> --key=<full fingerprint, quoted if it has spaces>
```

Ask the user, then attach the signature. With `--upload` the script checks the release again right before the upload and reads the signature back. If a check fails after the upload, the signature is still attached: the script names it and the command that deletes it, which has to run before any retry.

```sh
scripts/release/sign_dashmate_checksums.sh --tag=v<version> --key=<full fingerprint> --upload
```

**To rebuild a signed release:** once a release carries `SHA256SUMS.asc` (or any `SHA256SUMS.*`), release jobs started afterwards refuse to replace its packages or its `SHA256SUMS` and fail instead. To change either, delete the signature first (`gh release delete-asset v<version> SHA256SUMS.asc --repo dashpay/platform`), re-run the jobs, and sign the new file.

### 6. Stable only: update `master`

`master` tracks the newest stable release. Skip this step for a prerelease, and for a stable release that is not the newest stable version (step 1): a patch on an older line must not move `master` back.

Merge the release **tag**, not the moving dev-branch tip, and push the merge commit straight to `master`. `master` has only ever moved by merge commits, never by a reset (`git log --first-parent origin/master`), and it cannot be force-pushed (`allow_force_pushes: false`; `required_linear_history: false` allows the merge commit), so a wrong merge stays. Run it in a temporary worktree:

```sh
git fetch origin --force --tags master
git tag -l 'v*' --sort=-v:refname | grep -E '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -n 1   # must print v<version>
git merge-base --is-ancestor v<version> origin/master && echo "v<version> is already in master"
gh api repos/dashpay/platform/rulesets/501248 --jq .current_user_can_bypass   # must print "always"
worktree="$(mktemp -d)" && echo "${worktree}"
git worktree add --detach "${worktree:?}" origin/master
git -C "${worktree:?}" merge --no-ff -m "Merge tag 'v<version>' into master" v<version>
git -C "${worktree:?}" diff --quiet v<version> HEAD && echo "the merge has exactly the tree of v<version>"
```

Stop, and skip the rest of this step, if the first check prints another tag or the second says the tag is already in `master`. If the merge conflicts or the tree differs from the tag's, run `git -C <worktree> merge --abort`, remove the worktree and ask the user; don't push. Otherwise, after the user's go, push and remove the worktree, using the path printed above (a shell variable does not survive into a later command):

```sh
git -C <worktree> push origin HEAD:master
git worktree remove <worktree>
```

**The ruleset, not classic protection, is what blocks direct pushes.** `master`, `main`, `develop` and every `v*` branch are covered by a repo **ruleset** (`Block deletions, require PR`, `id 501248`) that requires a PR. It is **separate** from classic branch protection: a repo admin does **not** automatically bypass it, and `gh api repos/.../branches/<b>/protection` does **not** show it. Classic protection on `master` asks for one approval but does not enforce it for admins. Only users in the ruleset's `bypass_actors` can push directly (currently shumkov `24296` and QuantumExplorer `11468583`, both `always`). To grant a user standing bypass, fetch the ruleset, append them, and PUT it back:

```sh
gh api repos/dashpay/platform/rulesets/501248 > r.json
jq '{name,target,enforcement,bypass_actors:(.bypass_actors+[{actor_id:<userId>,actor_type:"User",bypass_mode:"always"}]),conditions,rules}' r.json > put.json
gh api --method PUT repos/dashpay/platform/rulesets/501248 --input put.json   # find <userId>: gh api users/<login> --jq .id
```

For a stable `X.Y.0`, continue with [Graduate the dev branch](#post-stable-release-graduate-the-dev-branch) before reporting.

### 7. Report

Tell the user:

- the version and the release PR;
- the release URL, and which release is Latest (`gh api repos/dashpay/platform/releases/latest --jq .tag_name`);
- the `release.yml` run and its result, including anything still running or re-run;
- whether the dashmate packages and `SHA256SUMS` are attached;
- the signature: attached by whom, or why not (a prerelease the maintainer chose not to sign, or still waiting for the run);
- `master`: the merge commit pushed, or why it was skipped;
- for `X.Y.0`, the graduation steps done;
- anything left pending.

## `yarn release`

`yarn release` (`scripts/release/release.sh`) bumps every workspace version, regenerates the changelog, and opens a release PR. It does **not** tag or publish anything.

### What one run does

1. Bumps the version in **all npm `package.json` files**, the **root `package.json`**, and the **`[workspace.package]` version in `Cargo.toml`** (all Rust crates inherit it).
2. Runs `cargo metadata` to refresh `Cargo.lock`.
3. Regenerates `CHANGELOG.md` (conventional-changelog, `dash` preset) from the changelog-base tag to HEAD.
4. Creates branch `release_<new-version>`, commits `chore(release): update changelog and version to <new-version>`.
5. Pushes the branch and opens a PR **against the branch you ran it from** (e.g. `v4.1-dev`), body from `scripts/release/pr_description.md`, assigned to a milestone.

### Command

```sh
yarn release -v=<target-version>     # explicit exact version — preferred, unambiguous
yarn release -t=<type>               # derive next version from current + type
yarn release -c=<tag>                # override the changelog-from tag (optional)
```

- `-v` / `--version` — set the exact version (validated as semver). The type is inferred from the prerelease id. **Prefer this** — no guessing.
- `-t` / `--type` — `release`, or a prerelease id: `dev`, `alpha`, `beta`, `rc` (any id works). Version is derived from the current version (see rules below).
- `-c` / `--changelog-from` — tag to build the changelog from. Omit to auto-detect via `find_latest_tag.js`.

#### `-t` derivation rules (from `bump_version.js`, when `-v` is not given)

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

## Rebuilding the dashmate deb of a published release

The dashmate deb's Debian version comes from the tag: `5.1.0-1`, `5.1.0~rc.1-1`, `5.1.0~~dev.1-1` (`packages/dashmate/scripts/deb-version.mjs`). A rebuild from the same tag produces the same version, which apt reports as already installed, so operators never receive it. A rebuild re-runs the release run's deb build, which needs the run's `js-build-<sha>` artifact, so it only works within 7 days of the release. To ship a rebuilt deb:

1. Set the repository variable `DASHMATE_DEB_REVISION` to the next revision (`2`, then `3`, ...). If the release carries `SHA256SUMS.asc`, delete it now: the release jobs refuse to replace a signed release's packages.
2. In the release's `release.yml` run, re-run the `Build Dashmate packages (deb, ubuntu-24.04)` job (`gh run rerun --job <job-id> --repo dashpay/platform`) and wait for that job to finish.
3. Delete `DASHMATE_DEB_REVISION` as soon as the build job has finished, whether it succeeded or failed. It is a repository variable, so any other release built while it is set gets that revision too. Only the build job reads it: the later steps don't need it, and it has to be set again only to re-run the build.
4. Go on only if the build succeeded. After a failed build the release job still publishes the debs the earlier build made, and `Release Dashmate checksums` is skipped; fix the cause, set the variable again and re-run the build. Then check that `Release Dashmate packages (deb, ubuntu-24.04)` ran after the build and attached debs whose names carry the new revision (`dashmate_5.1.0-2_amd64.deb`); re-run that job if it did not. `Release Dashmate checksums` runs after it and fails while the earlier debs are still attached.
5. Delete every other `dashmate_*_amd64.deb` and `dashmate_*_arm64.deb` asset of the release, keeping only the two that carry the new revision. Earlier builds have different names, so they stay attached, and the install docs and deploy tooling take the first matching asset.
6. Re-run the `Release Dashmate checksums` job by its id, not with `--failed`, which also re-runs unrelated jobs, so `SHA256SUMS` lists the new debs; then sign it as in [step 5](#5-sign-sha256sums). The id is not the number in the job's URL:

```sh
gh run view <run-id> --repo dashpay/platform --json jobs \
  --jq '.jobs[] | select(.name == "Release Dashmate checksums") | .databaseId'
gh run rerun --job <job-id> --repo dashpay/platform
```

Never set `DASHMATE_DEB_EPOCH` except to re-release a version that was published under oclif's old `<version>.<git sha>-1` scheme: once a package carries an epoch, every later package needs it too. Re-releasing that way also leaves the old `<version>.<git sha>-1` debs attached, so delete them the same way.

## Post-stable-release: graduate the dev branch

After a **stable** `vX.Y.0` release PR merges into `vX.Y-dev`, promote the branches so `vX.Y-dev` becomes the `X.Y.x` patch line and the next dev line takes over. Only do this for a **stable** `X.Y.0`, not for prereleases or patches. (Example below is the 4.1.0 run: current dev `v4.1-dev`, next dev `v4.2-dev`. If the next release is a **major** bump, the user says so — e.g. `v5.0-dev`.) Pushing to a `v*` branch directly needs the same ruleset bypass as `master` ([step 6](#6-stable-only-update-master)).

Steps (skip whatever is already done — e.g. the next dev branch and default may have been set early):

1. **Ensure the next dev branch exists** — `v(X.Y+1)-dev`. If it exists but is behind current dev, it just needs the sync in step 3.
2. **`master`** is updated in [step 6](#6-stable-only-update-master) of the release flow, from the release tag.
3. **Fast-forward next dev to current dev** (carry the release commits into the next line). It is typically strictly behind → a clean fast-forward:
   ```sh
   git push origin origin/vX.Y-dev:v(X.Y+1)-dev
   ```
   The next dev branch **stays at version `X.Y.0`** until its first `v(X.Y+1).0-dev.1` release is cut — do **not** bump it now.
4. **Make next dev the default branch** if it isn't already (repo Settings → Branches, or `gh api`).
5. **Milestones** — ensure `vX.Y.x` (patches for the just-released line) and `v(X.Y+1).0` (next dev prereleases) exist. Create with `gh api repos/dashpay/platform/milestones -f title="vX.Y.x" -f state=open`. (Stable-release milestone convention is `vX.Y.x`; prerelease is `vX.Y.0`.)
6. **Re-target open PRs** from current dev → next dev: `gh pr edit <n> --base v(X.Y+1)-dev`. Leave genuine `vX.Y.x` patch PRs on `vX.Y-dev`. Note the `milestone.yml` workflow auto-assigns a PR's milestone by its base branch.
