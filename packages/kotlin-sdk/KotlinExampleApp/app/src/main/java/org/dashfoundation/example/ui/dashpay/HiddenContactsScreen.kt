package org.dashfoundation.example.ui.dashpay

import androidx.compose.runtime.Composable
import androidx.navigation.NavHostController

/**
 * Hidden established-contacts list (reversible, cross-device) — placeholder
 * for the port of `HiddenContactsView.swift`. Replaced by the K3 slice C
 * implementation; the signature + route
 * ([org.dashfoundation.example.navigation.DashPayHidden]) are fixed here so
 * ContactsScreen's "Hidden contacts" link and the hub entry compile now.
 */
@Composable
fun HiddenContactsScreen(ownerIdentityIdHex: String, navController: NavHostController) {
    DashPayStubScaffold(
        title = "Hidden",
        rootTestTag = "dashpay.hidden.root",
        navController = navController,
    )
}
