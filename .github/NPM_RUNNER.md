# NPM release runners

NPM release compilation uses `[self-hosted, Linux, X64, npm-build]` in the
restricted `platform-npm-releases` runner group. Publishing
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
remain outside the checkout; each release starts with a fresh workspace. The
runner needs no Docker CLI/socket or KVM device.

## Provisioning and promotion

Use the reviewed `dashpay/dash-selfhosted-image` recipe and a tested immutable
image digest, not a moving tag. Register dedicated release capacity with `npm-build` only after
the NPM validation workflow succeeds on that image. Drain old registrations before
replacement; retain their image/configuration for rollback. Old release tags
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

## Separate PR and release state

The `platform-npm-releases` organization runner group must select only the
`dashpay/platform` repository and restrict execution to:

```text
dashpay/platform/.github/workflows/release-npm-build.yml@refs/heads/v4.2-dev
```

Protect that branch with the normal maintainer review policy. `release.yml`
invokes that protected reusable workflow; the reusable workflow rejects PR
callers and arbitrary branch dispatches before checkout. A PR cannot select the
release group by changing its own workflow to request the `npm-build` label.
The group-level selected-workflow restriction is a required operator setting,
not something a repository workflow can grant itself.

Use separate runner containers/VMs and separate registration, workspace, HOME,
Cargo registry and target-cache storage for PR and release pools. Do not mount
the same cache volumes into both pools. Release caches can persist between
releases; no PR may write them. Ordinary PR validation uses `npm-pr`; image
candidates continue to use fresh one-job registrations and volumes. Do not add
`npm-pr`, `rust-ci` or `kotlin-ci` to the release registration.

For another maintained branch, create its reviewed protected reusable-workflow
ref and corresponding group policy explicitly. Do not wildcard the workflow
restriction or allow PR refs. Validate the policy by attempting a PR job that
requests the release group: it must be rejected, while a permitted release dry
run succeeds. The workflow guard is defense in depth; enabling the release
pool without its group restriction does not establish this boundary.
