# Publishing `org.dashj:dash-sdk-android` to Maven Central

The SDK publishes under the existing **`org.dashj`** namespace (same as
`dashj-core`), so no new Central namespace verification is needed. This is a
Maven-coordinates decision only — the Kotlin source packages remain
`org.dashfoundation.dashsdk`. Deployment uses **JReleaser**, mirroring
[dashj/core/build.gradle](https://github.com/dashpay/dashj/blob/master/core/build.gradle):
release versions go to Maven Central via the Central Portal; `-SNAPSHOT`
versions go to the Sonatype snapshots repository.

## Prerequisites

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

## Consuming the artifact

```kotlin
repositories { mavenCentral() }
dependencies { implementation("org.dashj:dash-sdk-android:0.1.0") }
```

Requires `minSdk 29`. The AAR bundles the native library for `arm64-v8a` and
`x86_64`; `armeabi-v7a` is intentionally unsupported.
