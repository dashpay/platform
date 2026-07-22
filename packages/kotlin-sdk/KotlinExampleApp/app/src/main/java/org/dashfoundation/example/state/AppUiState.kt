package org.dashfoundation.example.state

import kotlinx.coroutines.flow.MutableStateFlow

/**
 * UI-only flags — port of `AppUIState.swift`: the wallets-tab sync-banner
 * toggle plus the invitation deep-link plumbing
 * (`pendingInviteURL` / `invitationClaimInFlight`).
 */
class AppUiState {
    val showWalletsSyncDetails = MutableStateFlow(false)

    /**
     * A `dashpay://invite` (or legacy applink) URI captured from an incoming
     * intent, parked until the claim sheet can consume it. Deliberately NOT
     * cleared by the no-wallet guard (deviation from iOS, which drops the
     * link walletless — flagged as an upstream bug): the fresh-install
     * onboarding tap must survive until a wallet exists. Bearer secret —
     * never logged.
     */
    val pendingInviteUri = MutableStateFlow<String?>(null)

    /**
     * True while a claim is running; a second incoming invite link stays
     * parked in [pendingInviteUri] until the current claim resolves
     * (mirror of the iOS mid-claim deferral gate).
     */
    val invitationClaimInFlight = MutableStateFlow(false)
}
