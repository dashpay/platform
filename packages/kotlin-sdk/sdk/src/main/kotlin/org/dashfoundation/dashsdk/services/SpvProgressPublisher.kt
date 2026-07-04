package org.dashfoundation.dashsdk.services

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.map
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.dashfoundation.dashsdk.wallet.SpvSyncProgressData
import org.dashfoundation.dashsdk.wallet.SpvSyncState

/**
 * Adapts [PlatformWalletManager.spvProgress] into the flat shape the app's
 * `SyncOverlayState` renders (`phaseTitle`, `overallPercentage`,
 * `isSyncing`) — port of the phase-title + overall-percentage logic Swift's
 * `GlobalSyncIndicator` derives from `PlatformSpvSyncProgress`.
 *
 * The publisher is a pure mapping over the manager's Rust-reflecting
 * StateFlow; it holds no state and makes no sync decisions. The app owns
 * its own overlay data class, so this exposes an equivalent [SpvProgressData]
 * the app maps 1:1.
 */
class SpvProgressPublisher(private val manager: PlatformWalletManager) {

    /** The flat SPV progress shape the sync overlay needs. */
    data class SpvProgressData(
        val phaseTitle: String,
        val overallPercentage: Float,
        val isSyncing: Boolean,
    )

    /** Mapped SPV progress, refreshed off the manager's 1 Hz poll. */
    val progress: Flow<SpvProgressData> = manager.spvProgress.map { it.toProgressData() }

    /** Snapshot the current progress without collecting. */
    fun current(): SpvProgressData = manager.spvProgress.value.toProgressData()

    private fun SpvSyncProgressData.toProgressData(): SpvProgressData =
        SpvProgressData(
            phaseTitle = phaseTitle(this),
            overallPercentage = overallPercentage.toFloat(),
            isSyncing = isSyncing,
        )

    private companion object {
        /**
         * The active phase's human title. Reports the earliest phase that is
         * present and not yet synced (headers → filter headers → filters →
         * masternodes), matching the Swift indicator's phase ordering; falls
         * back to the overall state when nothing is mid-sync.
         */
        fun phaseTitle(p: SpvSyncProgressData): String {
            val mid = sequenceOf(
                "Block Headers" to p.headers,
                "Filter Headers" to p.filterHeaders,
                "Filters" to p.filters,
                "Masternodes" to p.masternodes,
            ).firstOrNull { (_, sub) ->
                sub != null && sub.state != SpvSyncState.SYNCED
            }
            if (mid != null) return "Syncing ${mid.first}"
            return when (p.overallState) {
                SpvSyncState.WAIT_FOR_EVENTS -> "Idle"
                SpvSyncState.WAITING_FOR_CONNECTIONS -> "Connecting"
                SpvSyncState.SYNCING -> "Syncing"
                SpvSyncState.SYNCED -> "Synced"
                SpvSyncState.ERROR -> "Sync Error"
            }
        }
    }
}
