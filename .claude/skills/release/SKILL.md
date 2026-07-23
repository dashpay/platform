---
name: release
description: Cut a Dash Platform release (stable or prerelease) with `yarn release`. Use when asked to release/publish a new platform version, bump versions, or cut a beta/rc/dev/alpha prerelease (e.g. "release 4.1.0-beta.1").
argument-hint: "[target-version | type]"
---

# Cut a Platform release

`yarn release` (→ `scripts/release/release.sh`) bumps every workspace version, regenerates the changelog, and opens a release PR. It does **not** publish artifacts or tag — a maintainer merges the PR and CI/tagging takes it from there.

## What one run does

1. Bumps the version in **all npm `package.json` files**, the **root `package.json`**, and the **`[workspace.package]` version in `Cargo.toml`** (all Rust crates inherit it).
2. Runs `cargo metadata` to refresh `Cargo.lock`.
3. Regenerates `CHANGELOG.md` (conventional-changelog, `dash` preset) from the changelog-base tag to HEAD.
4. Creates branch `release_<new-version>`, commits `chore(release): update changelog and version to <new-version>`.
5. Pushes the branch and opens a PR **against the branch you ran it from** (e.g. `v4.1-dev`), body from `scripts/release/pr_description.md`, assigned to a milestone.

## Command

```
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

```
yarn release -v=4.1.0-beta.1
```

`-t=beta` yields the same result here (release→beta = minor bump + `.1`), but `-v` is explicit. Changelog auto-detects `v4.0.0` as the base (no prior `v4.1.0-beta.*` tags exist yet), so `-c` is unnecessary.
