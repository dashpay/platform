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

## Known gotcha: a brand-new npm package breaks the publish

`release.yml`'s **Release NPM packages** job publishes with `yarn workspaces foreach --all --no-private --parallel npm publish --tolerate-republish --access public` over **npm trusted-publishers OIDC**. OIDC trusted publishing can **only publish to a package that already exists** on npm with a trusted publisher configured — it **cannot create (bootstrap) a brand-new package**. So the first release after someone adds a new `@dashevo/*` package fails: the job publishes every existing package fine, then exits 1 on the new one (which is `404 Not Found` on the registry). `--access public` and `--tolerate-republish` are already set — they are **not** the fix.

**Symptom:** the release run's "Release NPM packages" job fails; `yarn` reports `The command failed in workspace @dashevo/<new-pkg> ... exit code 1`; `npm view @dashevo/<new-pkg>` → `E404`, while the other contracts show the release version.

**Fix (needs `@dashevo` publish rights + a 2FA OTP — a human with npm access must do it):**

1. Check out the package at the released version and publish it once with a token (not OIDC). The npm dist-tag matches CI's: for a prerelease it is `<major>.<minor>-<suffix>` (e.g. `4.1-beta`); for a stable release it is `latest`. Read it from an already-published package: `npm view @dashevo/dpns-contract dist-tags`.

```sh
git checkout origin/<dev-branch> -- packages/<new-pkg>          # get the released version
cd packages/<new-pkg>
npm publish --access public --tag <major.minor-suffix> --otp=<code>
git checkout HEAD -- packages/<new-pkg>                          # restore working tree
```

2. Re-run the failed job — `--tolerate-republish` now skips every package (all already at the release version) → green:

```sh
gh run rerun <release-run-id> --repo dashpay/platform --failed
```

After this first publish the package exists on npm, so **every later release publishes it automatically**. Note: right after publishing, `npm view` may still `404` for a few minutes (npm CDN negative-cache); the `http fetch PUT 200` line in `~/.npm/_logs/…` confirms the publish landed.

**Prevention:** whenever you add a new publishable npm package to the monorepo, first-publish it manually (or set up its trusted publisher on npm) **before** cutting the release that would ship it.
