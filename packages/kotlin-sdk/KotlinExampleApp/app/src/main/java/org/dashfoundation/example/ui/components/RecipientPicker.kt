package org.dashfoundation.example.ui.components

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import androidx.lifecycle.compose.collectAsStateWithLifecycle
import kotlinx.coroutines.delay
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.dashfoundation.example.di.LocalAppContainer
import org.dashfoundation.example.di.LocalAppState
import org.dashfoundation.example.util.Base58
import org.dashfoundation.example.util.LenientJson
import org.dashfoundation.example.util.toHex
import org.dashfoundation.example.util.truncateMiddle

/**
 * A 32-byte identity selection produced by [RecipientPicker] —
 * port of `RecipientSelection` (RecipientPickerView.swift).
 * [identityIdHex] is the hex form (nav-arg / FFI friendly); [label] is
 * the human-readable form for the confirmation summary.
 */
data class RecipientSelection(
    val identityIdHex: String,
    val label: String,
)

/**
 * Reusable recipient slot for the token action forms — port of
 * `RecipientPickerView.swift`, with all three of its source modes:
 * local identities (IdentityDao, network-scoped), DPNS name resolution
 * (debounced `sdk.dpns.resolve`, matching the iOS `scheduleDpnsResolve`
 * flow), and manual base58 entry with on-the-fly validation.
 *
 * testTags reuse the iOS accessibility identifiers
 * (`recipient.identityPicker`, `recipient.identity.<base58>`).
 */
@Composable
fun RecipientPicker(
    selection: RecipientSelection?,
    onSelectionChange: (RecipientSelection?) -> Unit,
    networkRaw: Int,
    excludeIdentityIdHex: String? = null,
) {
    val container = LocalAppContainer.current
    val appState = LocalAppState.current
    val sdk by appState.sdk.collectAsStateWithLifecycle()

    var mode by rememberSaveable { mutableStateOf(0) } // 0 = Identities, 1 = DPNS, 2 = Manual
    var pastedBase58 by rememberSaveable { mutableStateOf("") }
    var dpnsInput by rememberSaveable { mutableStateOf("") }
    // null = idle/blank; "resolving"/"notFound"/"failed:<msg>" are transient
    // status strings surfaced under the DPNS field, mirroring the iOS
    // `dpnsState` enum. A successful resolution reports via onSelectionChange
    // and clears the status.
    var dpnsStatus by remember { mutableStateOf<String?>(null) }

    val identitiesFlow = remember(networkRaw) {
        container.database.identityDao().observeWalletOwnedByNetwork(networkRaw)
    }
    val identities by identitiesFlow.collectAsStateWithLifecycle(initialValue = emptyList())
    val candidates = identities.filter { it.identityId.toHex() != excludeIdentityIdHex }

    // Debounced DPNS resolution — mirrors RecipientPickerView.swift's
    // `scheduleDpnsResolve` (250ms debounce, late-arrival tolerant via the
    // LaunchedEffect key re-keying on every keystroke). Uses the already-
    // bridged `dash_sdk_dpns_resolve` (QueriesNative.dpnsResolve → Sdk.dpns).
    LaunchedEffect(dpnsInput, mode, sdk) {
        if (mode != 1) return@LaunchedEffect
        dpnsStatus = null
        val name = dpnsInput.trim()
        if (name.isEmpty()) {
            onSelectionChange(null)
            return@LaunchedEffect
        }
        val activeSdk = sdk ?: run {
            dpnsStatus = "failed:SDK not ready"
            return@LaunchedEffect
        }
        delay(250)
        dpnsStatus = "resolving"
        val outcome = runCatching { activeSdk.dpns.resolve(name) }
        val json = outcome.getOrNull()
        if (json.isNullOrBlank()) {
            // resolve() throws "not found" as an exception; a null/blank JSON
            // is the same "unregistered" outcome.
            onSelectionChange(null)
            dpnsStatus = outcome.exceptionOrNull()?.let { "notFound" } ?: "notFound"
            return@LaunchedEffect
        }
        val base58 = runCatching {
            LenientJson.parseToJsonElement(json).jsonObject["identityId"]?.jsonPrimitive?.content
        }.getOrNull()
        val decoded = base58?.let { Base58.decodeIdentifier(it) }
        if (decoded == null || decoded.toHex() == excludeIdentityIdHex) {
            onSelectionChange(null)
            dpnsStatus = if (decoded != null) "failed:That's the sender" else "notFound"
        } else {
            onSelectionChange(RecipientSelection(decoded.toHex(), name))
            dpnsStatus = null
        }
    }

    Column {
        SingleChoiceSegmentedButtonRow(modifier = Modifier.fillMaxWidth()) {
            listOf("Identities", "DPNS", "Manual").forEachIndexed { index, label ->
                SegmentedButton(
                    selected = mode == index,
                    onClick = {
                        if (mode != index) {
                            mode = index
                            dpnsStatus = null
                            onSelectionChange(null)
                        }
                    },
                    shape = SegmentedButtonDefaults.itemShape(index = index, count = 3),
                    modifier = Modifier.testTag("recipient.mode.$label"),
                ) { Text(label) }
            }
        }

        if (mode == 0) {
            if (candidates.isEmpty()) {
                Text(
                    "No other local identities on this network",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.padding(vertical = 8.dp),
                )
            } else {
                data class Option(val idHex: String?, val label: String)

                val options = listOf(Option(null, "Choose…")) + candidates.map { identity ->
                    Option(
                        idHex = identity.identityId.toHex(),
                        label = identity.mainDpnsName
                            ?: identity.alias
                            ?: truncateMiddle(Base58.encode(identity.identityId), 6, 4),
                    )
                }
                val selected = options.firstOrNull { it.idHex == selection?.identityIdHex }
                    ?: options.first()
                AccessiblePicker(
                    label = "Identity",
                    options = options,
                    selected = selected,
                    optionLabel = { it.label },
                    testTag = "recipient.identityPicker",
                    modifier = Modifier.padding(vertical = 8.dp),
                ) { option ->
                    onSelectionChange(
                        option.idHex?.let { RecipientSelection(it, option.label) },
                    )
                }
            }
        } else if (mode == 1) {
            OutlinedTextField(
                value = dpnsInput,
                onValueChange = { dpnsInput = it },
                label = { Text("DPNS name (e.g. alice.dash)") },
                singleLine = true,
                isError = dpnsStatus?.let { it == "notFound" || it.startsWith("failed:") } == true,
                supportingText = {
                    when (val status = dpnsStatus) {
                        null -> {}
                        "resolving" -> Text("Resolving…")
                        "notFound" -> Text("Name not registered")
                        else ->
                            if (status.startsWith("failed:")) {
                                Text("Lookup failed: ${status.removePrefix("failed:")}")
                            }
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp)
                    .testTag("recipient.dpnsField"),
            )
        } else {
            val trimmed = pastedBase58.trim()
            val decoded = if (trimmed.isEmpty()) null else Base58.decodeIdentifier(trimmed)
            val isExcluded = decoded != null && decoded.toHex() == excludeIdentityIdHex
            OutlinedTextField(
                value = pastedBase58,
                onValueChange = { newValue ->
                    pastedBase58 = newValue
                    val newTrimmed = newValue.trim()
                    val id = if (newTrimmed.isEmpty()) null else Base58.decodeIdentifier(newTrimmed)
                    onSelectionChange(
                        id?.takeIf { it.toHex() != excludeIdentityIdHex }?.let {
                            RecipientSelection(it.toHex(), truncateMiddle(newTrimmed, 6, 4))
                        },
                    )
                },
                label = { Text("Identity ID (base58)") },
                singleLine = true,
                isError = trimmed.isNotEmpty() && selection == null,
                supportingText = {
                    if (trimmed.isNotEmpty() && selection == null) {
                        Text(
                            when {
                                isExcluded -> "That's the sender — pick a different recipient."
                                decoded != null -> "Identity id is invalid."
                                else -> "Identity id is invalid base58."
                            },
                        )
                    }
                },
                modifier = Modifier
                    .fillMaxWidth()
                    .padding(vertical = 8.dp)
                    .testTag("recipient.manualField"),
            )
        }

        if (selection != null) {
            HorizontalDivider()
            Row(modifier = Modifier.padding(vertical = 8.dp)) {
                Column {
                    Text(selection.label, style = MaterialTheme.typography.bodyMedium)
                    Text(
                        truncateMiddle(selection.identityIdHex, 8, 6),
                        style = MaterialTheme.typography.bodySmall,
                        fontFamily = FontFamily.Monospace,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
            }
        }
    }
}
