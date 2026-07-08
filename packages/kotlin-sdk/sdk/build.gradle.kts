plugins {
    alias(libs.plugins.android.library)
    alias(libs.plugins.kotlin.android)
    alias(libs.plugins.kotlin.serialization)
    alias(libs.plugins.ksp)
    `maven-publish`
}

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
    implementation(libs.kotlinx.coroutines.core)
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
                }
            }
        }
    }
}
