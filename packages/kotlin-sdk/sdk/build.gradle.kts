import java.util.concurrent.Callable
import java.util.zip.ZipFile
import org.gradle.api.publish.maven.tasks.PublishToMavenLocal
import org.gradle.api.publish.maven.tasks.PublishToMavenRepository
import org.jreleaser.model.Active

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.ksp)
    `maven-publish`
    id("signing")
    alias(libs.plugins.jreleaser)
}

// Published Maven coordinate group. Per HashEngineering's call:
// publish under the existing `org.dashj` group — Dash already owns dashj.org and
// ships its legacy Core/Platform SDKs there, so this needs no separate domain
// verification for Maven Central (`org.dashfoundation` would have, since
// dashfoundation.org is held by a third party). The Kotlin source packages stay
// `org.dashfoundation.dashsdk`; this is a Maven-coordinates decision only.
// `maven-publish` handles local / internal-repo publishing; jreleaser (bottom of
// file) handles Maven Central releases and Sonatype SNAPSHOTs, mirroring
// dashj/core/build.gradle.
group = "org.dashj"
version = project.findProperty("sdkVersion")?.toString() ?: "0.1.0-v42int4e"

// Single source of truth for the published coordinate — consumed by the
// `release` publication and by verifyStagedAarForRemotePublish (both below).
val publishGroupId = "org.dashj"
val publishArtifactId = "dash-sdk-android"

android {
    namespace = "org.dashfoundation.dashsdk"
    compileSdk = 35
    ndkVersion = "28.1.13356709"

    defaultConfig {
        minSdk = 29

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")

        // Opt-in gate for live testnet integration tests: forwarded to the
        // instrumentation as the `testnet` argument and enforced by
        // `TestnetGuard`. Enable with `-Ptestnet=true`.
        testInstrumentationRunnerArguments["testnet"] =
            (project.findProperty("testnet")?.toString() ?: "false")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    // libdash_sdk_jni.so is produced by ../build_android.sh (cargo-ndk) into
    // src/main/jniLibs/<abi>/ — AGP packages it from there. NDK r28 emits
    // 16 KB-aligned ELF segments by default (Android 15+ requirement).
    publishing {
        singleVariant("release") {
            withSourcesJar()
            // Maven Central's Publisher Portal rejects any non-POM component that
            // lacks a javadoc jar, and the snapshot deployer sets
            // applyMavenCentralRules(true) which enforces it before upload.
            // AGP emits a (Kotlin-appropriate) javadoc jar here — no Dokka
            // dependency required.
            withJavadocJar()
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }

    testOptions {
        unitTests {
            isIncludeAndroidResources = true
        }
    }

    sourceSets {
        // Exported Room schemas for migration tests / CI schema checks.
        getByName("androidTest").assets.srcDir("$projectDir/schemas")
    }
}

ksp {
    arg("room.schemaLocation", "$projectDir/schemas")
    arg("room.incremental", "true")
}

dependencies {
    // `api`, not `implementation`: the public SDK surface returns coroutines
    // types (Room DAO `Flow<…>`, `PlatformWalletManager.pendingIdentityKeys:
    // StateFlow<…>`, etc.), so a consumer of the published coordinate needs
    // coroutines-core on its own compile classpath.
    api(libs.kotlinx.coroutines.core)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.kotlinx.serialization.json)

    api(libs.room.runtime)
    api(libs.room.ktx)
    ksp(libs.room.compiler)

    api(libs.datastore)
    api(libs.datastore.preferences)
    api(libs.biometric)

    testImplementation(libs.junit)
    testImplementation(libs.androidx.test.core)
    testImplementation(libs.robolectric)
    testImplementation(libs.kotlinx.coroutines.test)
    testImplementation(libs.androidx.test.ext.junit)
    testImplementation(libs.room.testing)

    androidTestImplementation(libs.androidx.test.ext.junit)
    androidTestImplementation(libs.androidx.test.runner)
    androidTestImplementation(libs.espresso.core)
    androidTestImplementation(libs.room.testing)
}

// Publishes the release AAR (incl. libdash_sdk_jni.so if ../build_android.sh ran first) with a
// POM carrying the Room/DataStore/Biometric api dependencies.
// Local:  ./gradlew :sdk:publishToMavenLocal   Coordinates: org.dashj:dash-sdk-android:<version>
// Remote: `:sdk:publishReleasePublicationToStagingRepository` stages the signed
//         artifacts into build/staging-deploy, then `:sdk:jreleaserDeploy` uploads
//         them to Maven Central (release) / Sonatype snapshots (see jreleaser block).
// Override the version with -PsdkVersion=x.y.z (defaults to 0.1.0-v42int4e).
afterEvaluate {
    publishing {
        repositories {
            // Local staging dir that jreleaser reads from and uploads (below); it is
            // not itself a network repo, so staging never needs credentials.
            maven {
                name = "staging"
                url = uri(layout.buildDirectory.dir("staging-deploy"))
            }
        }

        // The staging dir is reused across runs and maven-publish never removes
        // other versions' directories — a release deploy after an earlier
        // snapshot/release staging run would hand JReleaser stale coordinates.
        // Wipe it before every staging publish so each deploy starts clean.
        val cleanStagingDeploy = tasks.register<Delete>("cleanStagingDeploy") {
            delete(layout.buildDirectory.dir("staging-deploy"))
        }
        tasks.withType<PublishToMavenRepository>().configureEach {
            if (name.endsWith("ToStagingRepository")) {
                dependsOn(cleanStagingDeploy)
            }
        }
        publications {
            create<MavenPublication>("release") {
                from(components["release"])
                groupId = publishGroupId
                artifactId = publishArtifactId
                pom {
                    name.set("Dash Platform Kotlin SDK")
                    description.set(
                        "Kotlin SDK for Dash Core (L1 SPV) and Dash Platform " +
                            "(identities, DPNS, DashPay, shielded balances)"
                    )
                    url.set("https://github.com/dashpay/platform")
                    licenses {
                        license {
                            name.set("MIT License")
                            url.set("https://github.com/dashpay/platform/blob/master/LICENSE.md")
                        }
                    }
                    // scm + developers: required by Maven Central (and most
                    // remote repos) for POM validation. Pointed at the GitHub repo.
                    scm {
                        url.set("https://github.com/dashpay/platform")
                        connection.set("scm:git:git://github.com/dashpay/platform.git")
                        developerConnection.set(
                            "scm:git:ssh://git@github.com/dashpay/platform.git"
                        )
                    }
                    developers {
                        developer {
                            id.set("dashpay")
                            name.set("Dash Platform Contributors")
                            url.set("https://github.com/dashpay/platform")
                        }
                    }
                }
            }
        }
    }

    // Sign published artifacts, but only for a real remote publish. `publishToMavenLocal`
    // and a bare `assemble` never put a PublishToMavenRepository task in the graph, so a
    // local build requires no GPG key. Keys come from the standard signing properties/env
    // (signing.keyId + signing.password + signing.secretKeyRingFile, or the in-memory
    // ORG_GRADLE_PROJECT_signingKey / signingPassword pair) when a remote publish runs.
    signing {
        // Wire the documented in-memory-key path: the ORG_GRADLE_PROJECT_signingKey
        // / signingPassword env vars land as the Gradle project properties
        // `signingKey` / `signingPassword`, but the signing plugin does not consume
        // them automatically — a CI release following that path would otherwise fail
        // in signReleasePublication before staging anything.
        // Absent (local dev), this is skipped and, with required=false below, no key
        // is needed. The legacy signing.keyId/password/secretKeyRingFile path is
        // untouched and still handled by Gradle's standard signing properties.
        val signingKey = project.findProperty("signingKey") as String?
        val signingPassword = project.findProperty("signingPassword") as String?
        if (!signingKey.isNullOrBlank()) {
            useInMemoryPgpKeys(signingKey, signingPassword)
        }
        setRequired(Callable {
            gradle.taskGraph.allTasks.any { it is PublishToMavenRepository }
        })
        sign(publishing.publications["release"])
    }
}

// Never publish a coordinate whose AAR lacks the native JNI library
// (libdash_sdk_jni.so). ../build_android.sh (cargo-ndk) produces it into
// src/main/jniLibs/<abi>/; that directory is gitignored and absent in a clean
// checkout, so without this guard a publish from a fresh tree yields an artifact
// that installs cleanly but dies at runtime with UnsatisfiedLinkError
// (NativeLoader.ensureLoaded → System.loadLibrary("dash_sdk_jni")).
val requiredJniAbis = listOf("arm64-v8a", "x86_64") // build_android.sh ABI policy

// A 0-byte .so (e.g. from an interrupted build_android.sh) is as broken at
// runtime as a missing one, so every check below requires real content, and the
// guard tasks capture plain serializable values (no script-object references)
// so they stay configuration-cache safe.
val jniMissingMessageTemplate =
    "Native JNI library libdash_sdk_jni.so is missing or empty for ABI(s) " +
        "%s under src/main/jniLibs/ — run ./build_android.sh " +
        "before publishing so the coordinate isn't broken at runtime."

// Remote / staging publish: ALWAYS hard-fail on a missing .so. `-PallowMissingJni`
// is deliberately NOT honored here — a nativeless AAR must never reach a network
// repo, nor jreleaser's build/staging-deploy dir (which `jreleaserDeploy` then
// uploads to Maven Central). This closes the escape where `-PallowMissingJni`
// previously downgraded the check to a warning for every publish task.
val verifyJniLibsForRemotePublish = tasks.register("verifyJniLibsForRemotePublish") {
    val jniLibsDir = file("src/main/jniLibs")
    val abis = requiredJniAbis
    val template = jniMissingMessageTemplate
    doLast {
        val missing = abis.filterNot { abi ->
            jniLibsDir.resolve("$abi/libdash_sdk_jni.so").let { so -> so.isFile && so.length() > 0L }
        }
        if (missing.isNotEmpty()) {
            throw GradleException(
                template.format(missing.joinToString()) +
                    " -PallowMissingJni is a local-only escape and is NOT honored for " +
                    "remote/staging deploys."
            )
        }
    }
}

// Local publish (`publishToMavenLocal`): a dev may pass `-PallowMissingJni` for a
// POM/metadata-only dry run into the local ~/.m2, which never leaves the machine.
val verifyJniLibsForLocalPublish = tasks.register("verifyJniLibsForLocalPublish") {
    val jniLibsDir = file("src/main/jniLibs")
    val abis = requiredJniAbis
    val template = jniMissingMessageTemplate
    val allowMissingJni = project.hasProperty("allowMissingJni")
    doLast {
        val missing = abis.filterNot { abi ->
            jniLibsDir.resolve("$abi/libdash_sdk_jni.so").let { so -> so.isFile && so.length() > 0L }
        }
        if (missing.isNotEmpty()) {
            val message = template.format(missing.joinToString())
            if (allowMissingJni) {
                logger.warn(
                    "WARNING: $message (continuing: -PallowMissingJni is set — local publish only.)"
                )
            } else {
                throw GradleException("$message Pass -PallowMissingJni for a local-only dry run.")
            }
        }
    }
}

tasks.withType<PublishToMavenRepository>().configureEach {
    dependsOn(verifyJniLibsForRemotePublish)
}
tasks.withType<PublishToMavenLocal>().configureEach {
    dependsOn(verifyJniLibsForLocalPublish)
}
// Deploy-time guard on the STAGED artifacts. verifyJniLibsForRemotePublish
// (above) checks the source jniLibs tree, but `jreleaserDeploy` uploads whatever
// already sits in build/staging-deploy — a valid current source tree could
// green-light a stale or malformed staged AAR. So, immediately before any
// jreleaser deploy/upload/release, inspect the staging repository itself:
//   1. artifacts must exist under exactly the intended
//      <group>/<artifactId>/<version> path, and nothing may be staged outside it
//      (jreleaser pushes the whole staging repository, strays included);
//   2. exactly one release AAR must be staged, named for the intended
//      artifactId + version (plain, or Maven unique-snapshot timestamped);
//   3. that AAR, opened as a zip, must contain jni/<abi>/libdash_sdk_jni.so for
//      every required ABI.
// Everything is computed in doLast, so configuration stays lazy and
// mavenLocal/plain builds never touch this. Only jreleaser tasks depend on it.
val verifyStagedAarForRemotePublish = tasks.register("verifyStagedAarForRemotePublish") {
    description = "Fails unless build/staging-deploy holds exactly the intended " +
        "GAV with a release AAR containing all required JNI libraries."
    // Capture plain values at configuration time; all I/O happens at execution.
    val stagingDirProvider = layout.buildDirectory.dir("staging-deploy")
    val expectedGroup = publishGroupId
    val expectedArtifact = publishArtifactId
    val expectedVersion = version.toString()
    val requiredAbis = requiredJniAbis
    doLast {
        val stagingRoot = stagingDirProvider.get().asFile
        val gav = "$expectedGroup:$expectedArtifact:$expectedVersion"
        val gavDir = stagingRoot
            .resolve(expectedGroup.replace('.', '/'))
            .resolve(expectedArtifact)
            .resolve(expectedVersion)
        if (!gavDir.isDirectory) {
            throw GradleException(
                "Staged repository $stagingRoot has no artifacts for $gav " +
                    "(expected $gavDir). Run " +
                    ":sdk:publishReleasePublicationToStagingRepository with this exact " +
                    "version first — do not deploy a staging dir left over from " +
                    "another version."
            )
        }

        // (1) No artifacts outside the intended GAV directory.
        val artifactExtensions = setOf("aar", "jar", "pom", "module")
        val strays = stagingRoot.walkTopDown()
            .filter { it.isFile && it.extension in artifactExtensions }
            .filterNot { it.parentFile == gavDir }
            .toList()
        if (strays.isNotEmpty()) {
            throw GradleException(
                "Staged repository $stagingRoot contains artifacts outside $gav " +
                    "(jreleaser would upload them too): " +
                    strays.joinToString { it.relativeTo(stagingRoot).path } +
                    ". Re-stage from clean (the staging publish runs cleanStagingDeploy)."
            )
        }

        // (2) Exactly one release AAR (sources/javadoc are jars, so any .aar is
        // the main artifact), named for the intended artifactId + version.
        val aars = gavDir.listFiles { f: java.io.File -> f.isFile && f.extension == "aar" }
            .orEmpty()
            .sortedBy { it.name }
        if (aars.size != 1) {
            throw GradleException(
                "Expected exactly one release AAR staged for $gav in $gavDir, found " +
                    "${aars.size}${if (aars.isEmpty()) "" else ": " + aars.joinToString { it.name }}. " +
                    "Re-stage from clean."
            )
        }
        val aar = aars.single()
        val plainName = "$expectedArtifact-$expectedVersion.aar"
        val timestampedSnapshotName = Regex(
            Regex.escape("$expectedArtifact-" + expectedVersion.removeSuffix("-SNAPSHOT")) +
                "-\\d{8}\\.\\d{6}-\\d+\\.aar"
        )
        val nameMatchesGav = aar.name == plainName ||
            (expectedVersion.endsWith("-SNAPSHOT") && timestampedSnapshotName.matches(aar.name))
        if (!nameMatchesGav) {
            throw GradleException(
                "Staged AAR ${aar.name} in $gavDir does not match the intended " +
                    "coordinate $gav (expected $plainName" +
                    (if (expectedVersion.endsWith("-SNAPSHOT")) " or a unique-snapshot timestamped variant" else "") +
                    "). Re-stage from clean."
            )
        }

        // (3) The staged AAR itself must carry the native library for every ABI.
        ZipFile(aar).use { zip ->
            // getEntry("x") falls back to a directory entry "x/", and a 0-byte
            // entry is as fatal at runtime as an absent one — require a real file
            // with content.
            val missing = requiredAbis.filter { abi ->
                val entry = zip.getEntry("jni/$abi/libdash_sdk_jni.so")
                entry == null || entry.isDirectory || entry.size <= 0L
            }
            if (missing.isNotEmpty()) {
                throw GradleException(
                    "Staged AAR ${aar.name} is missing (or has an empty) " +
                        "jni/<abi>/libdash_sdk_jni.so for ABI(s) ${missing.joinToString()} — " +
                        "it would die at runtime with UnsatisfiedLinkError. " +
                        "Run ../build_android.sh, re-stage, then deploy."
                )
            }
        }
        logger.lifecycle(
            "Verified staged AAR ${aar.name} for $gav: all required JNI ABIs present " +
                "(${requiredAbis.joinToString()})."
        )
    }
    // If a single invocation stages and deploys, verify AFTER staging has run.
    mustRunAfter(tasks.withType<PublishToMavenRepository>())
}

// jreleaser's deploy/upload/release tasks push the already-staged artifacts, so
// gate them on both remote checks — the source-tree check (a staging dir
// populated out-of-band can't be uploaded without the native library) and the
// staged-AAR inspection above (what is actually uploaded is what is verified).
tasks.matching { task ->
    task.name.startsWith("jreleaser") &&
        listOf("Deploy", "Upload", "Release").any { task.name.contains(it) }
}.configureEach {
    dependsOn(verifyJniLibsForRemotePublish)
    dependsOn(verifyStagedAarForRemotePublish)
}

// Maven Central / Sonatype deployment via jreleaser, mirroring
// dashj/core/build.gradle (https://github.com/dashpay/dashj/blob/master/core/build.gradle#L139).
// Flow: `maven-publish` stages the signed artifacts into build/staging-deploy (the
// `staging` repository configured above), then `./gradlew :sdk:jreleaserDeploy`
// uploads them — release versions to Maven Central, -SNAPSHOT versions to the
// Sonatype snapshots repo. jreleaser reads its GPG key and Sonatype credentials
// from env/props (JRELEASER_GPG_*, JRELEASER_MAVENCENTRAL_*/JRELEASER_NEXUS2_*)
// at deploy time only, so `publishToMavenLocal` and any build that never invokes a
// jreleaser task need no signing key or account.
jreleaser {
    project {
        name.set("dash-sdk-android")
        description.set(
            "Kotlin SDK for Dash Core (L1 SPV) and Dash Platform " +
                "(identities, DPNS, DashPay, shielded balances)"
        )
        links {
            homepage.set("https://github.com/dashpay/platform")
        }
        authors.set(listOf("Dash Platform Contributors"))
        license.set("MIT")
        gitRootSearch.set(true)
    }

    signing {
        active.set(Active.ALWAYS)
        armored.set(true)
    }

    deploy {
        maven {
            // Tagged release versions -> Maven Central via the Sonatype portal.
            mavenCentral {
                create("sonatype") {
                    active.set(Active.RELEASE)
                    url.set("https://central.sonatype.com/api/v1/publisher")
                    stagingRepository(
                        layout.buildDirectory.dir("staging-deploy").get().asFile.path
                    )
                }
            }
            // -SNAPSHOT versions -> Sonatype snapshots repository.
            nexus2 {
                create("snapshots") {
                    active.set(Active.SNAPSHOT)
                    url.set("https://central.sonatype.com/repository/maven-snapshots/")
                    snapshotUrl.set("https://central.sonatype.com/repository/maven-snapshots/")
                    applyMavenCentralRules.set(true)
                    snapshotSupported.set(true)
                    closeRepository.set(true)
                    releaseRepository.set(true)
                    stagingRepository(
                        layout.buildDirectory.dir("staging-deploy").get().asFile.path
                    )
                }
            }
        }
    }
}
