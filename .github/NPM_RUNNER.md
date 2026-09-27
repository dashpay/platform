# NPM release runners

NPM release compilation uses a fresh single-job runner with a unique
`platform-release-<run-id>-<attempt>-npm` label in the `platform-release-builds`
runner group. Kotlin releases use the same lifecycle with a `-kotlin` label. Publishing
continues on GitHub-hosted Ubuntu with OIDC; the builder receives no publishing
credentials. The `npm-release-build` action is shared by releases and image
validation so both compile and pack with the same setup.

## Image contract

`.github/runner-requirements.json` pins the complete Linux image requirements and
recipe commit. The job runs `ci-image-contract verify` before compiling. The
runner must have `/opt/client-codegen` matching `packages/dapi-grpc/codegen.json`.
Its protobuf 3.18.1 compiler is intentionally separate from Rust's protoc 32.0.
TypeScript generation comes from the workspace's pinned `ts-protoc-gen` dependency.

Image-owned native dependencies are verified, never installed using sudo. Rust,
Node and the pinned WASM tools use writable runner/user locations. Cargo targets
remain in job-local HOME; each release starts with fresh runner, HOME and workspace state. The
runner needs no Docker CLI/socket or KVM device.

## Provisioning and promotion

Use the reviewed `dashpay/dash-selfhosted-image` recipe and a tested immutable
image digest, not a moving tag. Deploy the host-side disposable release controller
only after the NPM validation workflow succeeds on that image. Do not add generic
release labels to persistent CI registrations. See the
[controller installation and cleanup runbook](https://github.com/dashpay/dash-selfhosted-image/blob/main/docs/disposable-releases.md).
Old release tags
still contain their original workflows and do not automatically gain this fix.

Requirements-changing PRs select a candidate label bound to the complete PR head
and image digest. The image controller must support the `npm` job kind and
`.github/workflows/npm-runner-validation.yml`. Manifests requesting native client
generation require successful Rust, Kotlin and NPM candidate jobs for promotion;
skipped fork jobs do not qualify. Existing same-repository/trusted-fork guards
remain in effect.

The trusted `runner-image-candidate.yml` bootstrap, controller and Rust/Kotlin
candidate routing must be installed on each consuming branch before candidate
promotion can work. Platform PRs #4702 and #4912 establish those pieces; reconcile
their requirements/selector files with this NPM extension when landing them. In
particular, update both the bootstrap's reusable-workflow SHA and its
`control_revision` to a reviewed image-repository revision supporting
`client_codegen` and `npm`. Merely changing `recipe_revision` is insufficient.
The default `v4.2-dev` and `v4.3-dev` branches must each use an explicit compatible
manifest; this change does not alter an existing release tag or deploy a runner.

## Verification

`npm-runner-validation.yml` runs the real release build and packing action, DAPI
unit tests, and a byte-for-byte check that packed Node/web clients match the
freshly generated files. It uploads tarballs but never publishes them.
`test-client-codegen.yml` also builds the native compilers on hosted Linux/macOS,
checks committed generated output, tests failure recovery and validates packing.

Local setup and generator test commands are in `packages/dapi-grpc/README.md`.

After installing the controller, use the `release.yml` dispatch with
`tag=npm-test:v<package.json version>` on a protected development branch for a non-publishing
NPM build. For Kotlin, dispatch `release-kotlin-sdk.yml` from the protected branch
with an existing published `tag` and `dry_run=true`: compilation/artifact upload
run, but release attachment and Maven publication are both skipped. Check that
the image contract matches the selected source. Neither controller unit tests
nor an image smoke test establishes that these end-to-end jobs pass.

## Separate PR and release state

The `platform-release-builds` organization runner group selects only
`dashpay/platform` and contains only controller-created one-job registrations.
Each build requests:

```yaml
runs-on:
  group: platform-release-builds
  labels: [self-hosted, Linux, X64, 'platform-release-${{ github.run_id }}-${{ github.run_attempt }}-npm']
```

There is no fallback to `npm-build`, `rust-ci` or `kotlin-ci`. Without the
controller, builds stay queued. Runtime markers reject accidental routing to an
ordinary runner; they are not cryptographic attestation. The host controller
independently checks repository, event, workflow, run, attempt and commit before
creating fresh JIT capacity. Only one job can consume each registration; the host
destroys its container/processes, HOME, registration and workspace afterward.

**Ordinary PR caching is unchanged.** PR validation keeps its own persistent
Cargo/Gradle/Yarn caches. Releases reuse the prebaked image/toolchains but never
mount PR state or restore shared executable dependency caches. Yarn caching is
opted out only for the release runtime; Kotlin release build/publication disable
Gradle cache restores. A cold release compile is the intentional tradeoff; do not
reintroduce shared caches to speed it up without reviewing their writer trust.

`release.yml` calls its local reusable workflow, so the workflow travels with the
release source. Port it and the matching image requirements to 4.3; the host
controller needs no branch-specific allowlist. Branch protection, trusted tags,
fork approvals and hosted publishing authorization remain necessary. Optional
selected-workflow group restrictions are defense in depth, not the mechanism
that erases prior-job state. Labels alone are not authorization.

This assumes a trusted host and pinned image. A fresh container does not repair
host compromise or retroactively secure old release tags/artifacts. Merge/deploy
the controller before relying on this workflow change for a release.
