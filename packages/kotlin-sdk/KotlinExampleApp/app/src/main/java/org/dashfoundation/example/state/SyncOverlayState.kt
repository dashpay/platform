package org.dashfoundation.example.state

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow

/**
 * SPV progress snapshot for the global overlay — Kotlin mirror of the
 * fields `GlobalSyncIndicator` reads from `walletManager.spvProgress`.
 */
data class SpvProgress(
    val phaseTitle: String,
    val overallPercentage: Float,
    val isSyncing: Boolean,
)

/**
 * The single publication point for fast-cadence SPV progress. The wallet
 * manager layer feeds [publish]; only leaf composables collect [progress]
 * (see the leaf-isolation note in `ContentView.swift`).
 */
class SyncOverlayState {

    private val _progress = MutableStateFlow<SpvProgress?>(null)
    val progress: StateFlow<SpvProgress?> = _progress.asStateFlow()

    fun publish(progress: SpvProgress?) {
        _progress.value = progress
    }
}
