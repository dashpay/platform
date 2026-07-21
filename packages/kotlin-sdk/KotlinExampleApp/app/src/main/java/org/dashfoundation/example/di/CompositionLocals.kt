package org.dashfoundation.example.di

import androidx.compose.runtime.staticCompositionLocalOf
import org.dashfoundation.example.state.AppState
import org.dashfoundation.example.state.AppUiState

/**
 * Composition locals mirroring the `environmentObject` injections of the
 * iOS app. Provided once in MainActivity's `setContent`.
 */
val LocalAppContainer = staticCompositionLocalOf<AppContainer> {
    error("AppContainer not provided")
}

val LocalAppState = staticCompositionLocalOf<AppState> {
    error("AppState not provided")
}

val LocalAppUiState = staticCompositionLocalOf<AppUiState> {
    error("AppUiState not provided")
}
