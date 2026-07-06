package org.dashfoundation.example

import android.os.Bundle
import android.view.WindowManager
import androidx.activity.compose.setContent
import androidx.activity.enableEdgeToEdge
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.testTagsAsResourceId
import androidx.fragment.app.FragmentActivity
import org.dashfoundation.example.di.AppContainer
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.di.LocalAppUiState
import org.dashfoundation.example.services.auth.AuthPrompt
import org.dashfoundation.example.ui.AppRoot
import org.dashfoundation.example.ui.theme.ExampleTheme

/**
 * Toggle for FLAG_SECURE on screens that display secrets (seed backup, key
 * reveal) — blocks screenshots/recents thumbnails while active.
 */
val LocalSecureScreen = staticCompositionLocalOf<(Boolean) -> Unit> { {} }

/**
 * [FragmentActivity] (not plain ComponentActivity) because androidx
 * BiometricPrompt requires one — the [AuthPrompt] gate is bound to this
 * activity's lifecycle.
 */
class MainActivity : FragmentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge()

        val container: AppContainer = (application as ExampleApplication).container

        // Bind the Activity-bound biometric prompt into the app-scoped
        // delegating gate (KeystoreSigner + the seed/recovery flows all
        // route through it). Re-binding on recreation is intentional.
        container.biometricGate.delegate = AuthPrompt(this)

        setContent {
            // Expose Compose testTags as uiautomator resource-ids so the
            // TEST_PLAN's on-device flows are drivable via adb (the Android
            // analog of iOS's accessibility identifiers being queryable).
            androidx.compose.foundation.layout.Box(
                androidx.compose.ui.Modifier.semantics { testTagsAsResourceId = true },
            ) {
            ExampleTheme {
                CompositionLocalProvider(
                    LocalAppContainer provides container,
                    LocalAppState provides container.appState,
                    LocalAppUiState provides container.appUiState,
                    LocalSecureScreen provides ::setSecureScreen,
                ) {
                    AppRoot()
                }
            }
            }
        }
    }

    private fun setSecureScreen(secure: Boolean) {
        if (secure) {
            window.setFlags(
                WindowManager.LayoutParams.FLAG_SECURE,
                WindowManager.LayoutParams.FLAG_SECURE,
            )
        } else {
            window.clearFlags(WindowManager.LayoutParams.FLAG_SECURE)
        }
    }
}
