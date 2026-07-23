package org.dashfoundation.example.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountBalanceWallet
import androidx.compose.material.icons.filled.Group
import androidx.compose.material.icons.filled.Person
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Sync
import androidx.compose.material3.Icon
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.testTag
import androidx.navigation.NavDestination.Companion.hasRoute
import androidx.navigation.NavGraph.Companion.findStartDestination
import androidx.navigation.compose.currentBackStackEntryAsState
import androidx.navigation.compose.rememberNavController
import org.dashfoundation.example.navigation.AppNavHost
import org.dashfoundation.example.navigation.RootTab
import org.dashfoundation.example.ui.sync.GlobalSyncIndicator

/**
 * Root scaffold — port of the `TabView` in `ContentView.swift`: five tabs
 * with per-tab back stacks (saveState/restoreState = iOS per-tab
 * NavigationStacks) and the global sync overlay drawn above the content,
 * top-aligned, isolated from tab recomposition.
 */
@Composable
fun MainScreen() {
    val navController = rememberNavController()
    val backStackEntry by navController.currentBackStackEntryAsState()
    val currentDestination = backStackEntry?.destination

    fun isSelected(tab: RootTab): Boolean =
        currentDestination?.hierarchy()?.any { it.hasRoute(tab.routeClass) } == true

    Scaffold(
        bottomBar = {
            NavigationBar {
                RootTab.entries.forEach { tab ->
                    NavigationBarItem(
                        modifier = Modifier.testTag(tab.testTag),
                        selected = isSelected(tab),
                        onClick = {
                            navController.navigate(tab.route) {
                                popUpTo(navController.graph.findStartDestination().id) {
                                    saveState = true
                                }
                                launchSingleTop = true
                                restoreState = true
                            }
                        },
                        icon = { Icon(tab.icon, contentDescription = tab.label) },
                        label = { Text(tab.label) },
                    )
                }
            }
        },
    ) { innerPadding ->
        Box(Modifier.fillMaxSize()) {
            AppNavHost(
                navController = navController,
                modifier = Modifier
                    .fillMaxSize()
                    .padding(innerPadding),
            )
            // Fast-cadence SPV progress is collected only inside the
            // indicator (← the leaf-isolation note in ContentView.swift).
            GlobalSyncIndicator(
                isSyncTab = isSelected(RootTab.SYNC),
                modifier = Modifier.align(Alignment.TopCenter),
            )

            // Orphan-mnemonic detection + recovery dialogs — runs once on
            // the first composition (← ContentView.checkForOrphanMnemonic).
            org.dashfoundation.example.ui.wallet.OrphanRecoveryHost()
        }
    }
}

private fun androidx.navigation.NavDestination.hierarchy() =
    generateSequence(this) { it.parent }

private val RootTab.icon: ImageVector
    get() = when (this) {
        RootTab.SYNC -> Icons.Default.Sync
        RootTab.WALLETS -> Icons.Default.AccountBalanceWallet
        RootTab.IDENTITIES -> Icons.Default.Person
        RootTab.DASHPAY -> Icons.Default.Group
        RootTab.SETTINGS -> Icons.Default.Settings
    }

private val RootTab.label: String
    get() = when (this) {
        RootTab.SYNC -> "Sync"
        RootTab.WALLETS -> "Wallets"
        RootTab.IDENTITIES -> "Identities"
        RootTab.DASHPAY -> "DashPay"
        RootTab.SETTINGS -> "Settings"
    }
