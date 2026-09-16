package org.dashfoundation.example.ui.sync

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.statusBarsPadding
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import org.dashfoundation.example.di.LocalAppContainer

/**
 * Global SPV sync progress overlay — port of `GlobalSyncIndicatorOverlay`
 * in `ContentView.swift` (lines 585–653): a 2dp bar filled to the overall
 * sync percentage, hidden on the Sync tab (which renders its own detail).
 *
 * Deliberately the ONLY composable that collects the fast-cadence sync
 * progress flow, mirroring the iOS leaf-isolation pattern: progress ticks
 * recompose this thin bar, never the tab content. The flow is fed by the
 * wallet-manager layer via [org.dashfoundation.example.state.SyncOverlayState];
 * while no sync runs it is null and nothing renders.
 */
@Composable
fun GlobalSyncIndicator(
    isSyncTab: Boolean,
    modifier: Modifier = Modifier,
) {
    val container = LocalAppContainer.current
    val progress by container.syncOverlayState.progress.collectAsStateWithLifecycle()

    val current = progress ?: return
    if (isSyncTab || !current.isSyncing) return

    Box(
        modifier = modifier
            .statusBarsPadding()
            .fillMaxWidth(current.overallPercentage.coerceIn(0f, 1f))
            .height(2.dp)
            .background(MaterialTheme.colorScheme.primary),
    )
}
