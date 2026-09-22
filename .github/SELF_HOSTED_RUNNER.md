# Self-hosted runner contract

## Reproducible Linux image

Source, locked dependencies, deployment examples and publishing workflow:
**[dashpay/dash-selfhosted-image](https://github.com/dashpay/dash-selfhosted-image)**.
Use its `linux/amd64` contract-1 image for persistent Linux `kotlin-ci` / `rust-ci`
runners. The shared Rust action requires `/opt/ci/contract-version` to be `1` on
self-hosted Linux and fails early if an old/native runner picks up the job.

Platform's desired versions and checksums now live in
[.github/runner-requirements.json](runner-requirements.json). This includes an immutable image-recipe
commit. Persistent Linux jobs verify **both** the installed lock and recipe
revision; a contract-1 marker by itself is not sufficient. The earlier published
bootstrap image must be replaced by a matching candidate before these changes
can be merged.

The image locks Ubuntu 24.04 by digest, apt to a signed archive snapshot, and
downloaded toolchains to exact URLs and SHA-256 hashes. It includes:

| Toolchain | Contract |
| --- | --- |
| Native build | build-essential, clang/LLVM, Snappy, CMake, GMP, OpenSSL, pkg-config |
| Rust | rustup plus the repository baseline; exact repo-selected toolchains may be installed user-locally |
| Cargo helpers | llvm-cov **0.9.1**, nextest **0.9.144**, machete **0.9.2**, ndk **4.1.2** |
| Protobuf / Java | protoc **32.0**, JDK **17** |
| Android | API **35**, build-tools **35.0.0**, NDK **28.1.13356709**, pinned emulator/system image |
| Other job tools | Git, GitHub CLI, jq, Python 3, gpg, zip/unzip |

The SDK is image-owned. The persistent Kotlin workflow uses the image's
`ci-android-emulator` wrapper to create user-writable AVDs, boot with KVM, and stop
the emulator after testing. It does **not** run setup-android, sdkmanager, or an
emulator action that upgrades image packages at job runtime. Lockscreen/PIN and
unlocked-device checks remain in `.github/scripts/kotlin-instrumented-tests.sh`.

## Runtime privileges

- Non-root uid/gid **1001:1001**, no sudo, no Docker CLI or host Docker socket.
- Drop **all** capabilities; enable **no-new-privileges**. Keep default Docker
  seccomp/AppArmor policies, with no privileged mode or host namespaces.
- Dedicated registration/work volumes only. Kotlin adds **`/dev/kvm:rw`** and its
  numeric host group; Rust-only runners do not need that device.
- The operator configures `/dev/kvm` as `root:kvm`, mode **0660**. Jobs only check
  access; they never modify host udev rules, permissions or system packages.
- Preserve runner-group selected-repository access and the existing fork guards.
  Persistent job data is not isolation between mutually untrusted repositories.

Building/publishing the image uses Docker on an ephemeral GitHub-hosted builder;
that privilege is not passed into the resulting persistent runner.

## Publish, prove, then roll out

For Platform requirements changes, use the PR-first lifecycle below. The
standalone image publishing workflow remains useful for recipe development,
but its default lock is not a separate source of Platform requirements.

1. Use a successful [image publishing run](https://github.com/dashpay/dash-selfhosted-image/actions/workflows/image.yml).
   Publication requires non-root compiler/confinement checks, `KVM_CREATE_VM`, and
   a real API 35 emulator boot. Retrieve `image-reference.txt` from the run.
2. Set `RUNNER_IMAGE=dashpay/dash-selfhosted-image@sha256:<published-digest>` in
   the operator's deployment. Do not use a floating image or a locally inherited
   `github-runner-runner:latest` parent. The image repo's Compose files enforce the
   runtime boundary above; the KVM overlay is optional.
3. Drain the old runner before migration. Register a new name in the **existing
   group**, retaining its selected repositories, with only the required labels.
   Use a short-lived registration-token file, not a PAT stored in Compose.
4. Prove a real Rust job and Kotlin job on that exact runner/digest before retiring
   the old instance. Keep the previous registration/image for rollback. Rebuilding
   or pushing source does not replace any live runner automatically.

Deploy and prove the contract-1 image **before merging the consuming workflows**.
Record the selected digest and real-job evidence with the deployment; do not infer
runtime health from YAML validation or the image tag alone. Rebuild/repin when
dependencies change, including runner updates required by GitHub's update policy.

## Requirements changes: candidate before merge, promotion after

1. Change .github/runner-requirements.json in the Platform PR, including exact download URLs,
   checksums and package metadata. A change to this file is the automatic build
   flag; no separate label is required. Change the pinned recipe commit only
   when recipe/image code changes. Ordinary user-local Rust toolchain updates
   still follow rust-toolchain.toml.
2. The trusted base-branch publisher builds and smoke-tests a candidate on a
   disposable VM. A separate VM publishes it without executing PR image/code
   with Docker Hub credentials.
3. The normal Rust and Kotlin workflows wait for the candidate, then request
   temporary runners labelled for **this PR head, exact digest and job kind**.
   A host-side controller creates one-job non-root containers, with KVM only
   for Kotlin. Real application jobs must pass; skipped fork jobs do not count.
4. After merge, the publisher verifies the merged/current requirements and both
   real candidate jobs, then promotes **the same tested digest**, without a
   rebuild. Each base branch gets a platform-<branch> channel; main advances only
   for Platform's actual GitHub default branch.
5. Promotion does not restart production runners. The operator drains and
   switches ordinary runner capacity to the reviewed digest using the rollout
   procedure above. Requirements checks fail explicitly until capacity matches.

The trusted caller must land separately before a PR can use this flow.
See the image repository's
[bootstrap, GitHub App and candidate-controller setup](https://github.com/dashpay/dash-selfhosted-image/blob/feat/platform-pr-images/docs/platform-pr-images.md).
No App key, Docker socket, registry credential or host workspace enters a job.
The existing trusted-fork restrictions are unchanged; the controller's
exact-head approvals do not override workflow-side guards.

Run the routing checks with:

~~~sh
python3 -m unittest discover -s .github/scripts/tests -v
~~~

## Hosted Linux and native macOS remain distinct

The shared Rust action branches on `runner.environment`: persistent Linux verifies
the image's native libraries and protoc, while GitHub-hosted release/nightly/book/
JavaScript consumers retain apt provisioning and the user-local protoc cache.
Both select clang through `CC`/`CXX`, without mutating system alternatives.

The Linux image does not provision macOS. Native macOS `rust-ci` runners still
need the existing Homebrew dependencies plus llvm-cov 0.9.1, nextest 0.9.144 and
machete 0.9.2; the wallet fast path needs machete 0.9.2. Provision and verify these
separately before rollout. Do not silently install tools or swallow failures in
persistent jobs.

Docker publication and hosted Kotlin release/nightly jobs remain on ephemeral
GitHub-hosted runners. The latter explicitly install cargo-ndk 4.1.2. Any future
self-hosted job that genuinely needs Docker must use separately isolated capacity;
the persistent Rust/Kotlin runner must not regain the host Docker socket.
