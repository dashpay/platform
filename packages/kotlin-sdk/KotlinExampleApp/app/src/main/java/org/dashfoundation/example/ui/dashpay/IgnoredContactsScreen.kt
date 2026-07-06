package org.dashfoundation.example.ui.dashpay

import androidx.compose.runtime.Composable
import androidx.navigation.NavHostController

/**
 * Ignored-senders list (per-sender mute, reversible) — placeholder for the
 * port of `IgnoredContactsView.swift`. Replaced by the K3 slice C
 * implementation; the signature + route
 * ([org.dashfoundation.example.navigation.DashPayIgnored]) are fixed here so
 * the hub's "Ignored" entry compiles now.
 */
@Composable
fun IgnoredContactsScreen(identityIdHex: String, navController: NavHostController) {
    DashPayStubScaffold(
        title = "Ignored",
        rootTestTag = "dashpay.ignored.root",
        navController = navController,
    )
}
