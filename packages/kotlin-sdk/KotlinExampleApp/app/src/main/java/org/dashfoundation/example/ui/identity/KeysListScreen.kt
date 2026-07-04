package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.Card
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.navigation.KeyDetail
import org.dashfoundation.example.ui.components.FormSection

/**
 * All public keys of an identity — port of `KeysListView.swift`. Rows from
 * the Room [PublicKeyDao]; tapping opens [KeyDetailScreen]. Add-key and
 * disable-key actions need the `updateIdentity` FFI (a named deferral to the
 * key-management milestone); the [org.dashfoundation.example.services.KeyDisableGate]
 * that guards disabling is ported and unit-testable already.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun KeysListScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val keys by container.database.publicKeyDao()
        .observeByIdentityId(identityIdHex)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Identity Keys") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        LazyColumn(
            modifier = Modifier.fillMaxSize().padding(padding),
            contentPadding = androidx.compose.foundation.layout.PaddingValues(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            if (keys.isEmpty()) {
                item {
                    FormSection {
                        Text(
                            "No keys recorded for this identity.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
            items(keys, key = { it.keyId }) { key ->
                Card(
                    modifier = Modifier
                        .testTag("keysList.row.${key.keyId}")
                        .clickable { navController.navigate(KeyDetail(identityIdHex, key.keyId)) },
                ) {
                    ListItem(
                        headlineContent = { Text("Key #${key.keyId} · ${key.purpose}") },
                        supportingContent = {
                            Text(
                                buildString {
                                    append(key.securityLevel)
                                    append(" · ")
                                    append(key.keyType)
                                    if (key.disabledAt != null) append(" · disabled")
                                },
                            )
                        },
                    )
                }
            }
        }
    }
}
