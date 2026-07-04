package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material.icons.filled.QrCodeScanner
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import androidx.navigation.NavHostController
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.navigation.IdentityDetail
import org.dashfoundation.example.navigation.QrScanner
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes

/**
 * Load an existing identity by id — port of `LoadIdentityView.swift`. Paste
 * or scan an identity id, fetch it via `sdk.identities.fetch`, and persist a
 * Room [IdentityEntity] with `isLocal = false` (matching the Swift
 * `PersistentIdentity(isLocal: false)` on load). Key-material import (voting /
 * owner / payout / user private keys) rides the key-management milestone.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun LoadIdentityScreen(navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()

    var identityIdText by remember { mutableStateOf("") }
    var alias by remember { mutableStateOf("") }
    var isLoading by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }

    // QR scanner hand-back.
    val savedStateHandle = navController.currentBackStackEntry?.savedStateHandle
    LaunchedEffect(savedStateHandle) {
        savedStateHandle
            ?.getStateFlow<String?>(QrScanner.RESULT_KEY, null)
            ?.collect { raw ->
                if (raw != null) {
                    identityIdText = raw.trim()
                    savedStateHandle[QrScanner.RESULT_KEY] = null
                }
            }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Load Identity") },
                navigationIcon = {
                    IconButton(onClick = { navController.popBackStack() }) {
                        Icon(Icons.AutoMirrored.Filled.ArrowBack, contentDescription = "Back")
                    }
                },
            )
        },
    ) { padding ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(padding)
                .verticalScroll(rememberScrollState())
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = "Identity") {
                OutlinedTextField(
                    value = identityIdText,
                    onValueChange = { identityIdText = it },
                    label = { Text("Identity ID (hex or Base58)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("loadIdentity.idField"),
                )
                Row(
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                    horizontalArrangement = Arrangement.End,
                ) {
                    OutlinedButton(
                        onClick = { navController.navigate(QrScanner) },
                        modifier = Modifier.testTag("loadIdentity.scan"),
                    ) {
                        Icon(Icons.Default.QrCodeScanner, contentDescription = null)
                        Text("Scan")
                    }
                }
                OutlinedTextField(
                    value = alias,
                    onValueChange = { alias = it },
                    label = { Text("Alias (optional)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().padding(top = 8.dp),
                )
            }

            SubmitButton(
                text = "Load Identity",
                isLoading = isLoading,
                enabled = identityIdText.isNotBlank(),
                modifier = Modifier.fillMaxWidth().testTag("loadIdentity.submit"),
            ) {
                val activeSdk = sdk ?: return@SubmitButton
                val id = identityIdText.trim()
                isLoading = true
                scope.launch {
                    try {
                        val json = activeSdk.identities.fetch(id)
                            ?: throw IllegalStateException("Identity not found on ${network.displayName}")
                        val balance = runCatching {
                            Json.parseToJsonElement(json).jsonObject["balance"]
                                ?.jsonPrimitive?.longOrNull ?: 0L
                        }.getOrDefault(0L)

                        // Decode the id to raw 32 bytes for the primary key. Hex
                        // ids are 64 chars; Base58 falls back to the SDK id in
                        // the fetched JSON if present.
                        val idBytes = if (id.length == 64 && id.all { it.isHexDigit() }) {
                            id.hexToBytes()
                        } else {
                            decodeIdFromJson(json) ?: throw IllegalStateException(
                                "Could not decode identity id — paste the 64-char hex form.",
                            )
                        }

                        container.database.identityDao().upsert(
                            IdentityEntity(
                                identityId = idBytes,
                                balance = balance,
                                isLocal = false,
                                alias = alias.trim().ifBlank { null },
                                networkRaw = network.ffiValue,
                            ),
                        )
                        navController.navigate(IdentityDetail(idBytes.toHexString())) {
                            popUpTo(org.dashfoundation.example.navigation.LoadIdentity) { inclusive = true }
                        }
                    } catch (e: Exception) {
                        error = e.message ?: "Failed to load identity"
                    } finally {
                        isLoading = false
                    }
                }
            }

            Text(
                "Loaded identities are read-only (no keys imported). Import keys from the " +
                    "identity's Keys screen once key management lands.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
            )
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}

private fun Char.isHexDigit(): Boolean = this in '0'..'9' || this in 'a'..'f' || this in 'A'..'F'

/** Pull a hex/id field out of the fetched identity JSON as a fallback. */
private fun decodeIdFromJson(json: String): ByteArray? = runCatching {
    val obj = Json.parseToJsonElement(json).jsonObject
    val idStr = obj["id"]?.jsonPrimitive?.content ?: return null
    if (idStr.length == 64 && idStr.all { it in "0123456789abcdefABCDEF" }) {
        idStr.hexToBytes()
    } else {
        null
    }
}.getOrNull()

@OptIn(ExperimentalStdlibApi::class)
private fun ByteArray.toHexString(): String = toHexString(HexFormat.Default)
