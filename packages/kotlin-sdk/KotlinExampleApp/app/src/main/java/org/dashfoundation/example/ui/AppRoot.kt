package org.dashfoundation.example.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.CircularProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.rememberCoroutineScope
import kotlinx.coroutines.launch
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.CancellationException
import org.dashfoundation.example.di.AppContainer.BootstrapState
import org.dashfoundation.example.di.LocalAppContainer

/**
 * Bootstrap gate — port of the `isInitialized / bootstrapError / Retry`
 * branch of `ContentView.swift` (lines 53–81) plus the `.task { bootstrap() }`
 * from `SwiftExampleAppApp`. Once ready, hands off to [MainScreen].
 *
 * The network-change and wallet-set rebind observers from
 * `SwiftExampleAppApp.body` attach here as the wallet-manager layer lands.
 */
@Composable
fun AppRoot() {
    val container = LocalAppContainer.current
    val bootstrap by container.bootstrapState.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    LaunchedEffect(Unit) {
        if (bootstrap is BootstrapState.Loading) {
            container.bootstrap()
        }
    }

    // Port of the SwiftExampleAppApp.body onChange(currentNetwork) observer:
    // every SDK rebuild (network switch, devnet reconfigure) re-activates
    // the per-network wallet manager. activate() is idempotent per live
    // SDK handle, so the bootstrap-time activation isn't duplicated.
    LaunchedEffect(Unit) {
        container.appState.sdk.collect { sdk ->
            if (sdk != null && container.bootstrapState.value is BootstrapState.Ready) {
                try {
                    container.activateManager()
                } catch (error: CancellationException) {
                    throw error
                } catch (error: Exception) {
                    android.util.Log.e(
                        "AppRoot",
                        "Failed to restore the active network's wallet manager",
                        error,
                    )
                }
            }
        }
    }

    when (val state = bootstrap) {
        is BootstrapState.Ready -> MainScreen()

        is BootstrapState.Loading -> Column(
            modifier = Modifier.fillMaxSize(),
            verticalArrangement = Arrangement.spacedBy(20.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            CircularProgressIndicator()
            Text("Initializing...")
        }

        is BootstrapState.Failed -> Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            verticalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterVertically),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Text("Initialization Error", style = MaterialTheme.typography.titleMedium)
            Text(
                state.error.message ?: "Unknown error",
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.error,
                textAlign = TextAlign.Center,
            )
            Button(onClick = { scope.launch { container.bootstrap() } }) {
                Text("Retry")
            }
        }
    }
}
