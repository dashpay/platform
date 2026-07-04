package org.dashfoundation.example.state

import kotlinx.coroutines.flow.MutableStateFlow

/**
 * UI-only flags — port of `AppUIState.swift` (which holds exactly one
 * published property: whether the wallets tab shows the detailed sync
 * banner).
 */
class AppUiState {
    val showWalletsSyncDetails = MutableStateFlow(false)
}
