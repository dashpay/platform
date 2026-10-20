// Root build file — plugin versions come from gradle/libs.versions.toml.
plugins {
    alias(libs.plugins.android.library) apply false
    alias(libs.plugins.android.application) apply false
    alias(libs.plugins.kotlin.android) apply false
    alias(libs.plugins.kotlin.compose) apply false
    alias(libs.plugins.kotlin.serialization) apply false
    alias(libs.plugins.ksp) apply false
}

// When the repo lives on a non-APFS volume (exFAT), macOS materializes
// xattrs as ._* AppleDouble files inside build outputs, which breaks AAPT
// resource merging. Set DASH_GRADLE_BUILD_ROOT to relocate all build dirs
// onto an APFS volume (see CLAUDE.md); unset leaves the default layout.
System.getenv("DASH_GRADLE_BUILD_ROOT")?.let { buildRoot ->
    allprojects {
        layout.buildDirectory.set(
            File(buildRoot, project.path.replace(":", "/").ifEmpty { "root" })
        )
    }
}
