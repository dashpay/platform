# Publishing `org.dashj:dash-sdk-android` to Maven Central

The SDK publishes under the existing **`org.dashj`** namespace (same as
`dashj-core`), so no new Central namespace verification is needed. This is a
Maven-coordinates decision only — the Kotlin source packages remain
`org.dashfoundation.dashsdk`. Deployment uses **JReleaser**, mirroring
[dashj/core/build.gradle](https://github.com/dashpay/dashj/blob/master/core/build.gradle):
release versions go to Maven Central via the Central Portal; `-SNAPSHOT`
versions go to the Sonatype snapshots repository.

## CI release path (preferred): the platform release

The Kotlin SDK releases **with the platform version** — there is no separate
tag namespace. Publishing the platform GitHub release for `vX.Y.Z[-pre.N]`
runs `.github/workflows/release.yml`, which calls
`.github/workflows/release-kotlin-sdk.yml` with that tag. It has two jobs:

1. **`build-and-release`** builds the native library and the release AAR from
   the tag's commit and attaches `dash-sdk-android-X.Y.Z[-pre.N].aar` to the
   platform GitHub release. The Maven version is derived from the tag and
   ONLY from the tag (`v4.1.0-rc.1` → `4.1.0-rc.1`), so the GitHub asset and
   the Maven Central artifact can never drift. An already-attached AAR is
   never overwritten.
2. **`maven-central-deploy`** stages the signed Maven artifacts and runs
   `jreleaserDeploy` under the tag-derived version, re-using the native
   libraries job 1 built. It declares `environment: maven-central`, which is
   the ONLY place the publishing secrets exist.

Every published platform release (prereleases included) publishes the SDK.
Tags that would derive a `-SNAPSHOT` version are rejected — snapshots are
never released from tags.

To re-run just the SDK release for an existing platform release (without
rebuilding the Docker images / npm packages), dispatch the workflow **at the
tag ref**:

```bash
gh workflow run release-kotlin-sdk.yml --ref vX.Y.Z -f tag=vX.Y.Z
```

Dispatching at the tag ref matters: the `maven-central` environment's tag
policy matches the *run's ref*, so a branch-ref dispatch cannot reach the
publishing secrets. The dispatch input must be the plain tag name (no
`refs/tags/` prefix — aliases are rejected so concurrency locks and asset
guards key on one spelling) for an existing tag that already has a published
GitHub release; the workflow refuses branches and never creates a release.
**Confirm with the release owner before dispatching** — the re-run can attach
public release assets and publish an irrevocable Maven Central version.

Note: a dispatch loads the workflow file *from the selected ref*, so this
re-run path only works for tags created after the consolidation landed. For
an older tag, dispatch at the dev branch (`--ref vX-dev -f tag=vX.Y.Z`):
build and asset attach still run against the tag's commit, while the Maven
deploy job skips on the branch ref — publish those to Maven with the manual
runbook below instead.

### The `maven-central` environment (where the secrets live)

The five publishing secrets exist **only as environment secrets on the
`maven-central` environment** — never repository- or organization-scoped:
`JRELEASER_MAVENCENTRAL_SONATYPE_USERNAME`,
`JRELEASER_MAVENCENTRAL_SONATYPE_PASSWORD`, `JRELEASER_GPG_SECRET_KEY`,
`JRELEASER_GPG_PUBLIC_KEY`, `JRELEASER_GPG_PASSPHRASE`. Environment scoping
means only the `maven-central-deploy` job (which declares the environment)
can read them; the unprotected build job — and any other workflow — cannot.
The environment carries a deployment tag policy of `v*`, restricting it to
platform release tags. No required reviewers are configured: the deploy runs
automatically once the platform release is published. (Reviewers can be added
later in Settings → Environments → maven-central for a human approval gate —
Maven Central publishes are irrevocable.)

While the secrets are missing entirely the deploy job **skips with a
notice** (the AAR still attaches to the release, and the Maven release can be
done manually with the runbook below — from the tag's commit with the tag's
version). A *partially* configured secret set fails the run rather than
guessing. Both publish guards (`verifyJniLibsForRemotePublish`,
`verifyStagedAarForRemotePublish`) run in the CI path via the same task
dependencies as locally.

## Prerequisites (manual fallback path)

The runbook below is the fallback if CI is unavailable. When releasing a
tagged version manually, check out the `vX.Y.Z` tag and pass exactly its
`X.Y.Z` as `-PsdkVersion`, so the manual Maven deploy cannot drift from the
GitHub-release AAR.

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
