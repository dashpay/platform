# Publishing `org.dashj:dash-sdk-android` to Maven Central

The SDK publishes under the existing **`org.dashj`** namespace (same as
`dashj-core`), so no new Central namespace verification is needed. This is a
Maven-coordinates decision only — the Kotlin source packages remain
`org.dashfoundation.dashsdk`. Deployment uses **JReleaser**, mirroring
[dashj/core/build.gradle](https://github.com/dashpay/dashj/blob/master/core/build.gradle):
release versions go to Maven Central via the Central Portal; `-SNAPSHOT`
versions go to the Sonatype snapshots repository.

## CI release path (preferred): the `kotlin-sdk-vX.Y.Z` tag

Pushing a `kotlin-sdk-vX.Y.Z` tag runs
`.github/workflows/kotlin-sdk-release.yml`, which is split into two jobs:

1. **`build-and-release`** builds the native library and the release AAR and
   attaches the AAR to the GitHub release for the tag (existing behavior). It
   derives `-PsdkVersion` **from the tag** (`kotlin-sdk-v0.1.0` → `0.1.0`) and
   decides whether a Maven Central deploy is warranted.
2. **`maven-central-deploy`** stages the signed Maven artifacts and runs
   `jreleaserDeploy` under the tag-derived version. It runs only when job 1
   decided a deploy is warranted, and it is gated by the **`maven-central`
   environment** (see the human gate below).

In CI the version is never passed by hand — it comes only from the tag — so
the Maven Central artifact and the GitHub-release AAR are always the same
commit under the same version (the deploy job checks out the exact commit SHA
the build job built — not the mutable tag ref — and re-stages the native
libraries that build produced, rather than rebuilding them; the tag is
re-validated against that SHA after approval, and once more inline immediately
before the deploy). Tags that would derive a `-SNAPSHOT` version are rejected. A
`kotlin-sdk-*` tag without the `vX.Y.Z` form still gets its GitHub-release AAR
but skips the Maven deploy (there is no version to derive). The workflow's
manual `workflow_dispatch` path is for re-runs/emergencies only: it takes an
*existing* tag name — either the short `kotlin-sdk-vX.Y.Z` form or a full
`refs/tags/kotlin-sdk-vX.Y.Z` ref — resolves it to the commit it currently
points at, checks out that commit, and derives the version from the tag the
same way; it has no version input.

### The human gate: the `maven-central` environment (required before secrets)

A push of any `kotlin-sdk-v*` tag would otherwise publish **irrevocably** to
Maven Central with no human review. To gate it, the `maven-central-deploy`
job declares `environment: maven-central`. That means the deploy is only as
safe as the environment's protection rules — **the environment must be
created in repository settings with required reviewers *before* the Central
secrets are installed**:

1. Repo **Settings → Environments → New environment** named exactly
   `maven-central`.
2. Enable **Required reviewers** and add the people allowed to approve a
   release (e.g. the `org.dashj` publisher). This is the human gate: the
   workflow pauses at the deploy job until a reviewer approves, and the
   approval is the last chance to stop an irrevocable publish.
3. Only then install the five Central/GPG secrets listed below as
   **environment secrets on `maven-central`**. Do not create repository- or
   organization-scoped copies: those scopes would make the credentials
   available to jobs that have not passed this environment's reviewer gate.
If a `kotlin-sdk-v*` tag is pushed while no reviewer approves, the deploy job
simply waits and can be rejected — the GitHub-release AAR is already attached
by job 1 regardless.

**Secrets gate:** the Maven deploy steps only run when the Sonatype Central
Portal credentials are configured as secrets on the **`maven-central`
environment**. That Portal token is currently a personal token (held by
HashEngineering, the `org.dashj` publisher), so until it is installed there
the deploy steps **skip with a notice** instead of failing — the tag still
produces the GitHub-release AAR, and the Maven release can be done manually
with the runbook below, **from the tag's commit with the tag's version**. Required
secrets (names match the env vars in the next section):
`JRELEASER_MAVENCENTRAL_SONATYPE_USERNAME`,
`JRELEASER_MAVENCENTRAL_SONATYPE_PASSWORD`, `JRELEASER_GPG_PUBLIC_KEY`,
`JRELEASER_GPG_SECRET_KEY`, `JRELEASER_GPG_PASSPHRASE`. The GPG secret
key/passphrase pair is also fed to the Gradle staging signature
(`ORG_GRADLE_PROJECT_signingKey` / `signingPassword`). A *partially*
configured secret set fails the run rather than guessing. Both publish guards
below (`verifyJniLibsForRemotePublish`, `verifyStagedAarForRemotePublish`)
run in the CI path via the same task dependencies as locally.

## Prerequisites (manual fallback path)

The runbook below is the fallback for while the Portal token is personal (or
if CI is unavailable). When releasing a tagged version manually, check out
the `kotlin-sdk-vX.Y.Z` tag and pass exactly its `X.Y.Z` as `-PsdkVersion`,
so the manual Maven deploy cannot drift from the GitHub-release AAR.

- **Sonatype Central Portal token** for the `org.dashj` namespace (held by
  HashEngineering — the same account that publishes `dashj-core`).
- **GPG signing key** (the dashj release key works), with the public key on the
  keyservers.
- **Native toolchain**: Android NDK r28+ (`28.1.13356709`), `cargo-ndk`, rustup
  targets `aarch64-linux-android` and `x86_64-linux-android`, JDK 17.

## Credentials — environment only, never committed

Gradle signing (signs artifacts during the staging publish; the classic
`signing.keyId` / `signing.password` / `signing.secretKeyRingFile` entries in
`~/.gradle/gradle.properties`, as used for dashj-core, also work):

```bash
export ORG_GRADLE_PROJECT_signingKey="$(cat private-key.asc)"   # ASCII-armored
export ORG_GRADLE_PROJECT_signingPassword="…"
```

JReleaser deploy (read only when a `jreleaser*` task runs):

```bash
export JRELEASER_MAVENCENTRAL_SONATYPE_USERNAME="…"   # Central Portal token name
export JRELEASER_MAVENCENTRAL_SONATYPE_PASSWORD="…"   # Central Portal token value
export JRELEASER_NEXUS2_SNAPSHOTS_USERNAME="…"        # same token (snapshots repo)
export JRELEASER_NEXUS2_SNAPSHOTS_PASSWORD="…"
export JRELEASER_GPG_PUBLIC_KEY="$(cat public-key.asc)"
export JRELEASER_GPG_SECRET_KEY="$(cat private-key.asc)"
export JRELEASER_GPG_PASSPHRASE="…"
```

## Release steps (from `packages/kotlin-sdk/`)

```bash
# 1. Build the native JNI library (arm64-v8a + x86_64) and verify its exports.
./build_android.sh --verify

# 2. Stage the signed release AAR + sources/javadoc jars into sdk/build/staging-deploy.
./gradlew :sdk:publishReleasePublicationToStagingRepository -PsdkVersion=0.1.0
# (the staging dir sdk/build/staging-deploy is wiped automatically before every
#  staging publish — cleanStagingDeploy — so stale versions from earlier runs
#  can never reach JReleaser)

# 3. Upload the staged artifacts. Release versions -> Maven Central;
#    -SNAPSHOT versions -> Sonatype snapshots.
./gradlew :sdk:jreleaserDeploy -PsdkVersion=0.1.0
```

Then approve/verify the deployment at <https://central.sonatype.com> and
confirm the artifact appears on search.maven.org (sync takes ~30 min).

**Versioning:** the default is `0.1.0-SNAPSHOT`; pass the same
`-PsdkVersion=x.y.z` to steps 2 and 3. The first release should be a real
version (e.g. `0.1.0`) — the Android wallet's CI must depend on a pinned
release, never a snapshot.

## The JNI guard

`sdk/src/main/jniLibs/` is gitignored, so a clean checkout has no
`libdash_sdk_jni.so` — and an AAR published without it installs fine but dies
at runtime with `UnsatisfiedLinkError`. The `verifyJniLibsForRemotePublish`
task therefore **hard-fails** every staging/remote/jreleaser publish task when
the `.so` is missing for `arm64-v8a` or `x86_64`. There is no override for
remote publishes; `-PallowMissingJni` only downgrades the check for
`publishToMavenLocal` (metadata-only dry runs into `~/.m2`). If the guard
fires, run step 1.

That check inspects the *source* tree; a second, deploy-time guard —
`verifyStagedAarForRemotePublish` — inspects what JReleaser will actually
upload. Immediately before any `jreleaser*` deploy/upload/release task it
opens the staged AAR in `sdk/build/staging-deploy` and hard-fails unless:
(a) the staged artifacts live under exactly the intended
`org/dashj/dash-sdk-android/<version>/` path, with nothing staged outside it;
(b) exactly one release AAR is staged, named for that coordinate; and
(c) the AAR contains both `jni/arm64-v8a/libdash_sdk_jni.so` and
`jni/x86_64/libdash_sdk_jni.so`. A stale or malformed staging dir therefore
cannot be deployed even when the current source tree is valid. If it fires,
re-run steps 1–2, then deploy. `publishToMavenLocal` never triggers this
guard.

## Consuming the artifact

```kotlin
repositories { mavenCentral() }
dependencies { implementation("org.dashj:dash-sdk-android:0.1.0") }
```

Requires `minSdk 29`. The AAR bundles the native library for `arm64-v8a` and
`x86_64`; `armeabi-v7a` is intentionally unsupported.
