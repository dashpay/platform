import org.gradle.api.publish.maven.tasks.PublishToMavenLocal
import org.gradle.api.publish.maven.tasks.PublishToMavenRepository

plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.ksp)
    `maven-publish`
}

// NOTE (Maven Central, future): the `org.dashfoundation` groupId can't be
// published to Maven Central yet — that requires owning the matching domain,
// and dashfoundation.org is held by a third party (HashEngineering, PR #4045).
// This coordinate is for `publishToMavenLocal` / an internal repo for now; a
// Maven Central move is a separate groupId/hosting decision.
group = "org.dashfoundation"
version = project.findProperty("sdkVersion")?.toString() ?: "0.1.0-SNAPSHOT"

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
// Local: ./gradlew :sdk:publishToMavenLocal   Coordinates: org.dashfoundation:dash-sdk-android:<version>
// Override the version with -PsdkVersion=x.y.z (defaults to 0.1.0-SNAPSHOT).
afterEvaluate {
    publishing {
        publications {
            create<MavenPublication>("release") {
                from(components["release"])
                groupId = "org.dashfoundation"
                artifactId = "dash-sdk-android"
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
                            url.set("https://github.com/dashpay/platform/blob/master/LICENSE")
                        }
                    }
                    // scm + developers: required by Maven Central (and most
                    // remote repos) for POM validation. Pointed at the GitHub
                    // repo rather than a vanity domain (see the group-id note
                    // above re: dashfoundation.org ownership).
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
}

// Never publish a coordinate whose AAR lacks the native JNI library
// (libdash_sdk_jni.so). ../build_android.sh (cargo-ndk) produces it into
// src/main/jniLibs/<abi>/; that directory is gitignored and absent in a clean
// checkout, so without this guard a publish from a fresh tree yields an artifact
// that installs cleanly but dies at runtime with UnsatisfiedLinkError
// (NativeLoader.ensureLoaded → System.loadLibrary("dash_sdk_jni")). Run
// ../build_android.sh first, or pass -PallowMissingJni to publish anyway (e.g. a
// POM/metadata-only dry run).
val requiredJniAbis = listOf("arm64-v8a", "x86_64") // build_android.sh ABI policy
val verifyJniLibsPresent = tasks.register("verifyJniLibsPresent") {
    doLast {
        val missing = requiredJniAbis.filterNot { abi ->
            file("src/main/jniLibs/$abi/libdash_sdk_jni.so").isFile
        }
        if (missing.isNotEmpty()) {
            val message = "Native JNI library libdash_sdk_jni.so is missing for ABI(s) " +
                "${missing.joinToString()} under src/main/jniLibs/ — run ./build_android.sh " +
                "before publishing so the coordinate isn't broken at runtime."
            if (project.hasProperty("allowMissingJni")) {
                logger.warn("WARNING: $message (continuing anyway: -PallowMissingJni is set)")
            } else {
                throw GradleException("$message Pass -PallowMissingJni to override.")
            }
        }
    }
}

tasks.withType<PublishToMavenRepository>().configureEach { dependsOn(verifyJniLibsPresent) }
tasks.withType<PublishToMavenLocal>().configureEach { dependsOn(verifyJniLibsPresent) }
