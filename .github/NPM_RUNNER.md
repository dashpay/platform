# NPM release runners

NPM release compilation uses `[self-hosted, Linux, X64, npm-build]`. Publishing
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
image digest, not a moving tag. Register new capacity with `npm-build` only after
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
