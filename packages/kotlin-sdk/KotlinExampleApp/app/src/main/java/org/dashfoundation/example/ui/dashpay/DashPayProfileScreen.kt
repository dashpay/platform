package org.dashfoundation.example.ui.dashpay

import androidx.compose.runtime.Composable
import androidx.navigation.NavHostController

/**
 * Read-only own-profile view + DIP-15 auto-accept QR — placeholder for the
 * port of `DashPayProfileView.swift`. Replaced by the K3 slice C
 * implementation; the signature + route
 * ([org.dashfoundation.example.navigation.DashPayProfile]) are fixed here so
 * the hub's "Your Profile" entry compiles now.
 */
@Composable
fun DashPayProfileScreen(identityIdHex: String, navController: NavHostController) {
    DashPayStubScaffold(
        title = "Your Profile",
        rootTestTag = "dashpay.profile.root",
        navController = navController,
    )
}
