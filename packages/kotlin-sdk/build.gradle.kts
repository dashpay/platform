import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    kotlin("jvm") version "1.9.22"
    kotlin("plugin.serialization") version "1.9.22"
    `java-library`
    `maven-publish`
}

group = "com.dash"
version = "1.0.0-SNAPSHOT"

repositories {
    mavenCentral()
}

dependencies {
    implementation(kotlin("stdlib"))
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.6.2")
    
    // JNA for FFI
    implementation("net.java.dev.jna:jna:5.14.0")
    
    // Logging
    implementation("io.github.microutils:kotlin-logging-jvm:3.0.5")
    implementation("ch.qos.logback:logback-classic:1.4.14")
    
    // Testing
    testImplementation(kotlin("test"))
    testImplementation("org.junit.jupiter:junit-jupiter:5.10.1")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.7.3")
    testImplementation("io.mockk:mockk:1.13.8")
    testImplementation("org.assertj:assertj-core:3.24.2")
}

tasks.test {
    useJUnitPlatform()
}

tasks.withType<KotlinCompile> {
    kotlinOptions.jvmTarget = "11"
}

java {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
    withSourcesJar()
    withJavadocJar()
}

// Configure native library loading
tasks.withType<Test> {
    systemProperty("java.library.path", "${projectDir}/native")
    environment("LD_LIBRARY_PATH", "${projectDir}/native")
    environment("DYLD_LIBRARY_PATH", "${projectDir}/native")
}

// Task to build the native library
tasks.register<Exec>("buildNative") {
    workingDir = file("${projectDir}/../../rs-sdk-ffi")
    commandLine("cargo", "build", "--release")
}

// Copy native library after build
tasks.register<Copy>("copyNativeLibrary") {
    dependsOn("buildNative")
    from("${projectDir}/../../rs-sdk-ffi/target/release") {
        include("libdash_sdk_ffi.so")
        include("libdash_sdk_ffi.dylib")
        include("dash_sdk_ffi.dll")
    }
    into("${projectDir}/native")
}

tasks.compileKotlin {
    dependsOn("copyNativeLibrary")
}

publishing {
    publications {
        create<MavenPublication>("maven") {
            from(components["java"])
            
            pom {
                name.set("Dash Platform Kotlin SDK")
                description.set("Kotlin SDK for Dash Platform")
                url.set("https://github.com/dashpay/platform")
                
                licenses {
                    license {
                        name.set("MIT License")
                        url.set("https://opensource.org/licenses/MIT")
                    }
                }
                
                developers {
                    developer {
                        id.set("dashpay")
                        name.set("Dash Core Group")
                        email.set("contact@dash.org")
                    }
                }
                
                scm {
                    connection.set("scm:git:git://github.com/dashpay/platform.git")
                    developerConnection.set("scm:git:ssh://github.com/dashpay/platform.git")
                    url.set("https://github.com/dashpay/platform")
                }
            }
        }
    }
}