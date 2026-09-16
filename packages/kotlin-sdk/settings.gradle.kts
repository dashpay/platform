pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "dash-kotlin-sdk"

include(":sdk")

// The example app lands in a later milestone; include it once present so
// :sdk builds stand alone in the meantime.
if (file("KotlinExampleApp/app/build.gradle.kts").exists()) {
    include(":app")
    project(":app").projectDir = file("KotlinExampleApp/app")
}
