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
    val network by appState.currentNetwork.collectAsStateWithLifecycle()
    val idBytes = remember(identityIdHex) { identityIdHex.hexToBytes() }

    var label by remember { mutableStateOf("") }
    // The answer of the FFI availability check for the current label; null
    // while it is pending, after a label change, and after a registration
    // (the label just registered is no longer available).
    var availability by remember { mutableStateOf<NameAvailability?>(null) }
    // Contested-ness of the entered label, decided by the FFI (`isContested`
    // in the `dash_sdk_dpns_check_availability` JSON) — ← RegisterNameView's
    // `isNameContested` (`dash_sdk_dpns_is_contested_username`). Drives the
    // contest warning + the contested success wording.
    var isContested by remember { mutableStateOf(false) }
    var isSubmitting by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    var success by remember { mutableStateOf<String?>(null) }

    // Debounced availability check (300ms) — ← RegisterNameView's async check.
    LaunchedEffect(label) {
        availability = null
        isContested = false
        // A new label starts a new registration: drop the previous one's
        // success line, which also held the Register button disabled.
        success = null
        if (label.isBlank()) return@LaunchedEffect
        delay(300)
        val activeSdk = sdk ?: return@LaunchedEffect
        val checked = runCatching {
            parseNameAvailability(activeSdk.dpns.checkAvailability(label.trim()))
        }.getOrElse { NameAvailability("Check failed: ${it.message}", available = false) }
        isContested = checked.isContested
        availability = checked
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
                    // Held while a registration runs, so its result always
                    // belongs to the label shown.
                    enabled = !isSubmitting,
                    modifier = Modifier.fillMaxWidth().testTag("registerName.label"),
                )
                availability?.let {
                    Text(
                        it.text,
                        style = MaterialTheme.typography.bodySmall,
                        color = if (it.available) {
                            MaterialTheme.colorScheme.primary
                        } else {
                            MaterialTheme.colorScheme.onSurfaceVariant
                        },
                        modifier = Modifier.padding(top = 6.dp).testTag("registerName.availability"),
                    )
                }
            }

            // Contest status — ← RegisterNameView's "Contest Status"
            // section. Contested-ness comes from the FFI availability check
            // (`isContested`); the drill-in reads live vote state.
            if (label.isNotBlank()) {
                FormSection(title = "Contest Status") {
                    Text(
                        if (isContested) "Contested" else "Regular",
                        style = MaterialTheme.typography.bodyMedium,
                        color = if (isContested) {
                            MaterialTheme.colorScheme.tertiary
                        } else {
                            MaterialTheme.colorScheme.primary
                        },
                        modifier = Modifier.testTag("registerName.contestStatus"),
                    )
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

            // Contest warning — ← RegisterNameView's "Contest Warning"
            // section, shown only for contested labels. Registration still
            // goes through the same single wallet FFI entry; Platform routes
            // a contested name through preorder + a masternode-vote contest.
            if (isContested && label.isNotBlank()) {
                FormSection(title = "Contested Name") {
                    Text(
                        "This name is under 20 characters with only letters " +
                            "(a-z), digits (0, 1), and hyphens. It requires a " +
                            "masternode vote contest to register.",
                        style = MaterialTheme.typography.bodySmall,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.testTag("registerName.contestWarning"),
                    )
                }
            }

            success?.let {
                Text(
                    it,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.testTag("registerName.success"),
                )
            }

            SubmitButton(
                text = "Register",
                isLoading = isSubmitting,
                // ← RegisterNameView's `.disabled(!isValidUsername ||
                // isAvailable != true || isRegistering)`: only a label the
                // FFI check reported available can be paid for.
                enabled = canRegisterName(label, availability, isSubmitting),
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
                        // ← RegisterNameView.registerName's contested vs.
                        // direct success wording. A contested name only
                        // *starts* a masternode-vote contest here; it isn't
                        // owned until the vote resolves.
                        success = if (isContested) {
                            val window = if (network == org.dashfoundation.dashsdk.Network.MAINNET) {
                                "14 days"
                            } else {
                                "90 minutes"
                            }
                            "Started contest for ${label.trim()}. Awaiting masternode " +
                                "vote — resolution in ~$window. View Contest Details to track."
                        } else {
                            "Registered $full"
                        }
                        // The label is taken now; the next label typed
                        // gets a fresh check.
                        availability = null
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

/**
 * The availability check's answer for one label: the text to show and
 * whether the FFI said it can be registered. [available] is Rust's
 * `available` flag as is; Kotlin adds no rule of its own.
 */
internal data class NameAvailability(
    val text: String,
    val available: Boolean,
    val isContested: Boolean = false,
)

/**
 * Read the `dash_sdk_dpns_check_availability` JSON. Anything other than a
 * JSON `"available": true` counts as not available, so a missing or
 * unreadable answer never enables Register.
 */
internal fun parseNameAvailability(json: String?): NameAvailability {
    if (json == null) return NameAvailability("Unknown", available = false)
    val obj = Json.parseToJsonElement(json).jsonObject
    val available = obj["available"]?.jsonPrimitive?.content == "true"
    // `isContested` is a JSON boolean the FFI computes from the normalized
    // (homograph-safe) label: 3-19 chars, only [a-z0-1-].
    val isContested = obj["isContested"]?.jsonPrimitive?.content == "true"
    val text = when {
        available -> "Available"
        obj["valid"]?.jsonPrimitive?.content == "false" -> "Invalid label"
        else -> obj["message"]?.jsonPrimitive?.content ?: "Taken"
    }
    return NameAvailability(text, available, isContested)
}

/** Register is offered only for a non-blank label the check reported available. */
internal fun canRegisterName(
    label: String,
    availability: NameAvailability?,
    isSubmitting: Boolean,
): Boolean = label.isNotBlank() && availability?.available == true && !isSubmitting
