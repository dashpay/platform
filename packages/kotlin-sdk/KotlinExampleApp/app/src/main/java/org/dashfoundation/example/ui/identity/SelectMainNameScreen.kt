package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.selection.selectable
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.util.hexToBytes

/**
 * Pick the identity's main DPNS name — port of `SelectMainNameView.swift`.
 * The main-name selection is a local display preference (the Swift view
 * updates `PersistentIdentity.mainDpnsName`); this writes the same
 * `mainDpnsName` column on the Room [IdentityEntity], no state transition.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SelectMainNameScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    val identity by container.database.identityDao()
        .observeByIdentityId(idBytes)
        .collectAsStateWithLifecycle(initialValue = null)
    val names by container.database.dpnsNameDao()
        .observeByIdentity(idBytes)
        .collectAsStateWithLifecycle(initialValue = emptyList())

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Main Name") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier.fillMaxSize().padding(padding).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Main Name") {
                if (names.isEmpty()) {
                    Text(
                        "No names to choose from — register one first.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                } else {
                    NameRow(
                        label = "None",
                        selected = identity?.mainDpnsName == null,
                        onSelect = { setMain(container, scope, idBytes, null) },
                    )
                    names.forEach { name ->
                        NameRow(
                            label = name.label,
                            selected = identity?.mainDpnsName == name.label,
                            onSelect = { setMain(container, scope, idBytes, name.label) },
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun NameRow(label: String, selected: Boolean, onSelect: () -> Unit) {
    androidx.compose.foundation.layout.Row(
        modifier = Modifier
            .fillMaxWidth()
            .selectable(selected = selected, onClick = onSelect)
            .padding(vertical = 8.dp)
            .testTag("selectMainName.row.$label"),
        verticalAlignment = Alignment.CenterVertically,
    ) {
        RadioButton(selected = selected, onClick = onSelect)
        Text(label, modifier = Modifier.padding(start = 8.dp))
    }
}

private fun setMain(
    container: org.dashfoundation.example.di.AppContainer,
    scope: kotlinx.coroutines.CoroutineScope,
    idBytes: ByteArray,
    label: String?,
) {
    scope.launch {
        val existing = container.database.identityDao().getByIdentityId(idBytes) ?: return@launch
        container.database.identityDao().upsert(existing.copy(mainDpnsName = label))
    }
}
