package org.dashfoundation.example.ui.identity

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
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
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.ui.components.ErrorAlertDialog
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton
import org.dashfoundation.example.util.hexToBytes

/**
 * Register a DPNS name for an identity — port of `RegisterNameView.swift`.
 * The label is availability-checked (debounced) via
 * `sdk.dpns.checkAvailability`; registration itself calls the single wallet
 * FFI entry `platform_wallet_register_dpns_name_with_signer` (the watch-only
 * capable path the Swift `registerDpnsName` uses) through
 * `IdentityRegistration.registerDpnsName`.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun RegisterNameScreen(identityIdHex: String, navController: NavHostController) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()
    val manager by container.walletManagerStore.activeManager.collectAsStateWithLifecycle()
    val scope = rememberCoroutineScope()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    var label by remember { mutableStateOf("") }
    var availability by remember { mutableStateOf<String?>(null) }
    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var success by remember { mutableStateOf<String?>(null) }

    // Debounced availability check (300ms) — ← RegisterNameView's async check.
    LaunchedEffect(label) {
        availability = null
        if (label.isBlank()) return@LaunchedEffect
        delay(300)
        val activeSdk = sdk ?: return@LaunchedEffect
        availability = runCatching {
            val json = activeSdk.dpns.checkAvailability(label.trim()) ?: return@runCatching "Unknown"
            val obj = Json.parseToJsonElement(json).jsonObject
            when {
                obj["available"]?.jsonPrimitive?.content == "true" -> "Available"
                obj["valid"]?.jsonPrimitive?.content == "false" -> "Invalid label"
                else -> obj["message"]?.jsonPrimitive?.content ?: "Taken"
            }
        }.getOrElse { "Check failed: ${it.message}" }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text("Register Name") },
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
            FormSection(title = "Name") {
                OutlinedTextField(
                    value = label,
                    onValueChange = { label = it.trim() },
                    label = { Text("Label (without .dash)") },
                    singleLine = true,
                    modifier = Modifier.fillMaxWidth().testTag("registerName.label"),
                )
                availability?.let {
                    Text(
                        it,
                        style = MaterialTheme.typography.bodySmall,
                        color = if (it == "Available") {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        modifier = Modifier.padding(top = 6.dp).testTag("registerName.availability"),
                    )
                }
            }

            // Contest status — ← RegisterNameView's "Contest Status"
            // section. iOS decides contested-ness via
            // `dash_sdk_dpns_is_contested_username` (not bridged), so the
            // drill-in is offered for any entered label; ContestDetailScreen
            // names the pending exports.
            if (label.isNotBlank()) {
                FormSection(title = "Contest Status") {
                    androidx.compose.material3.TextButton(
                        onClick = {
                            navController.navigate(
                                org.dashfoundation.example.navigation.ContestDetail(
                                    contestName = label,
                                    identityIdHex = identityIdHex,
                                ),
                            )
                        },
                        modifier = Modifier.testTag("registerName.viewContest"),
                    ) {
                        Text("View Contest Details")
                    }
                }
            }

            success?.let {
                Text("Registered $it", color = MaterialTheme.colorScheme.primary)
            }

            SubmitButton(
                text = "Register",
                isLoading = isSubmitting,
                enabled = label.isNotBlank() && success == null,
                modifier = Modifier.fillMaxWidth().testTag("registerName.submit"),
            ) {
                val mgr = manager ?: run { error = "No active wallet manager"; return@SubmitButton }
                // The owning wallet drives the signer; resolve it from the
                // identity's stored walletId.
                scope.launch {
                    val identity = container.database.identityDao().getByIdentityId(idBytes)
                    val walletId = identity?.walletId
                    val wallet = walletId?.let { mgr.wallet(it) }
                    if (wallet == null) {
                        error = "This identity has no wallet on this device — DPNS registration " +
                            "needs the owning wallet's signer."
                        return@launch
                    }
                    isSubmitting = true
                    try {
                        val full = mgr.identityRegistration.registerDpnsName(
                            walletHandle = wallet.handle,
                            identityId = idBytes,
                            label = label.trim(),
                            signerHandle = mgr.signerHandle,
                        )
                        success = full
                    } catch (e: Exception) {
                        error = e.message ?: "Registration failed"
                    } finally {
                        isSubmitting = false
                    }
                }
            }
        }
    }

    ErrorAlertDialog(message = error, onDismiss = { error = null })
}
