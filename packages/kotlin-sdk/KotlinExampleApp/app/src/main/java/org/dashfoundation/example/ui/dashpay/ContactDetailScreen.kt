package org.dashfoundation.example.ui.dashpay

import androidx.navigation.NavHostController

/**
 * One contact's detail + payments — placeholder for the port of
 * `ContactDetailView.swift`. Replaced by the K3 slice C implementation
 * (header / Send Dash / payment history / local alias-note-hide settings);
 * the signature and route ([org.dashfoundation.example.navigation.DashPayContactDetail])
 * are fixed here so ContactsScreen's row navigation compiles now.
 */
@androidx.compose.runtime.Composable
fun ContactDetailScreen(
    identityIdHex: String,
    contactIdHex: String,
    navController: NavHostController,
) {
    DashPayStubScaffold(
        title = "Contact",
        rootTestTag = "dashpay.detail.root",
        navController = navController,
    )
}
