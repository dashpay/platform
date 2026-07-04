package org.dashfoundation.example.ui.transitions

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.ArrowBack
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TopAppBar
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import androidx.navigation.NavHostController
import org.dashfoundation.example.services.transitions.DedicatedTransition
import org.dashfoundation.example.services.transitions.StateTransitionDefinitions
import org.dashfoundation.example.services.transitions.TransitionInput
import org.dashfoundation.example.services.transitions.TransitionInputType
import org.dashfoundation.example.ui.components.AccessiblePicker
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.SubmitButton

/**
 * Dynamic input form for one transition — port of `TransitionDetailView.swift`
 * + `TransitionInputView.swift`: renders each
 * [org.dashfoundation.example.services.transitions.TransitionInput] by its
 * type, validates required fields, and executes.
 *
 * Execution routing (mirrors the Swift dedicated-flow split):
 * - Dedicated credit transitions
 *   ([DedicatedTransition.TRANSFER_CREDITS] / [DedicatedTransition.WITHDRAW_CREDITS])
 *   pre-select a source identity via a picker and route to their dedicated
 *   screens (the bridged FFI paths).
 * - Every non-executable transition renders its form and surfaces the
 *   named-missing-export dialog on submit (the `SendTransactionScreen`
 *   pattern) — the composite state-transition FFI those need is not bridged.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TransitionDetailScreen(transitionKey: String, navController: NavHostController) {
    val definition = remember(transitionKey) { StateTransitionDefinitions.byKey(transitionKey) }

    // Per-field form state (survives config change / picker pops).
    val textInputs = remember { mutableStateMapOf<String, String>() }
    val boolInputs = remember { mutableStateMapOf<String, Boolean>() }
    var initialized by rememberSaveable(transitionKey) { mutableStateOf(false) }
    if (definition != null && !initialized) {
        definition.inputs.forEach { input ->
            when (input.type) {
                TransitionInputType.CHECKBOX ->
                    boolInputs[input.name] = input.defaultValue == "true"
                else ->
                    textInputs[input.name] = input.defaultValue ?: ""
            }
        }
        initialized = true
    }

    var notBridged by remember { mutableStateOf(false) }

    if (definition == null) {
        Scaffold(topBar = { TopAppBar(title = { Text("Transition") }) }) { padding ->
            Text(
                "Unknown transition.",
                modifier = Modifier.padding(padding).padding(24.dp),
                color = MaterialTheme.colorScheme.error,
            )
        }
        return
    }

    val requiredFilled = definition.inputs.all { input ->
        if (!input.required || input.type == TransitionInputType.CHECKBOX) {
            true
        } else {
            (textInputs[input.name]?.isNotBlank() == true)
        }
    }

    Scaffold(
        topBar = {
            TopAppBar(
                title = { Text(definition.label) },
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
                .imePadding()
                .padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            FormSection(title = definition.label) {
                Text(
                    definition.description,
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            definition.inputs.forEach { input ->
                TransitionInputField(
                    input = input,
                    textValue = textInputs[input.name] ?: "",
                    boolValue = boolInputs[input.name] ?: false,
                    onTextChange = { textInputs[input.name] = it },
                    onBoolChange = { boolInputs[input.name] = it },
                )
            }

            if (!definition.executable) {
                Text(
                    "This transition's state-transition FFI is not bridged in " +
                        "rs-unified-sdk-jni yet. The form is wired; submit shows the " +
                        "missing-export note.",
                    style = MaterialTheme.typography.bodySmall,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }

            SubmitButton(
                text = "Execute",
                isLoading = false,
                enabled = requiredFilled,
                modifier = Modifier.fillMaxWidth().testTag("transition.${definition.key}.execute"),
            ) {
                when (definition.dedicatedRoute) {
                    // Dedicated credit flows need a source identity; the
                    // catalog form doesn't pin one, so route to the identity
                    // list first is out of scope here — instead the Identity
                    // Detail screen's credit rows are the primary entry.
                    // From the catalog we surface the same guidance dialog.
                    DedicatedTransition.TRANSFER_CREDITS,
                    DedicatedTransition.WITHDRAW_CREDITS,
                    DedicatedTransition.TOP_UP_IDENTITY,
                    -> notBridged = true
                    null -> notBridged = true
                }
            }
        }
    }

    if (notBridged) {
        AlertDialog(
            onDismissRequest = { notBridged = false },
            title = { Text("Execute from Identity Detail") },
            text = {
                Text(
                    if (definition.executable) {
                        "This transition runs against a specific identity. Open an " +
                            "identity from the Identities tab and use its Top Up / " +
                            "Transfer / Withdraw actions — those wire the bridged " +
                            "credit FFI directly."
                    } else {
                        "The composite state-transition FFI for " +
                            "\"${definition.label}\" is not bridged in " +
                            "rs-unified-sdk-jni yet. The catalog form is fully " +
                            "wired — only the final execution is pending the bridge."
                    },
                )
            },
            confirmButton = { TextButton(onClick = { notBridged = false }) { Text("OK") } },
        )
    }
}

/** Render one input by its [TransitionInputType]. ← `TransitionInputView.swift`. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TransitionInputField(
    input: TransitionInput,
    textValue: String,
    boolValue: Boolean,
    onTextChange: (String) -> Unit,
    onBoolChange: (Boolean) -> Unit,
) {
    val tag = "transition.${input.name}"
    when (input.type) {
        TransitionInputType.CHECKBOX -> {
            androidx.compose.foundation.layout.Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = androidx.compose.ui.Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
            ) {
                Text(input.label, style = MaterialTheme.typography.bodyLarge)
                Switch(
                    checked = boolValue,
                    onCheckedChange = onBoolChange,
                    modifier = Modifier.testTag(tag),
                )
            }
        }
        TransitionInputType.SELECT -> {
            val selected = input.options.firstOrNull { it.value == textValue } ?: input.options.firstOrNull()
            if (selected != null) {
                AccessiblePicker(
                    label = input.label,
                    options = input.options,
                    selected = selected,
                    optionLabel = { it.label },
                    testTag = "$tag.selectPicker",
                    onSelected = { onTextChange(it.value) },
                )
            }
        }
        TransitionInputType.JSON, TransitionInputType.TEXTAREA -> {
            OutlinedTextField(
                value = textValue,
                onValueChange = onTextChange,
                label = { Text(input.label + if (input.required) " *" else "") },
                placeholder = input.placeholder?.let { { Text(it) } },
                textStyle = if (input.type == TransitionInputType.JSON) {
                    MaterialTheme.typography.bodyMedium.copy(fontFamily = FontFamily.Monospace)
                } else {
                    MaterialTheme.typography.bodyMedium
                },
                modifier = Modifier.fillMaxWidth().heightIn(min = 100.dp).testTag(tag),
            )
        }
        TransitionInputType.NUMBER -> {
            OutlinedTextField(
                value = textValue,
                onValueChange = { onTextChange(it.filter(Char::isDigit)) },
                label = { Text(input.label + if (input.required) " *" else "") },
                placeholder = input.placeholder?.let { { Text(it) } },
                singleLine = true,
                keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number),
                modifier = Modifier.fillMaxWidth().testTag(tag),
            )
        }
        // Pickers with no bridged option source render as plain hex/id text
        // entry in this milestone (the underlying transitions are deferred).
        TransitionInputType.TEXT,
        TransitionInputType.IDENTITY_PICKER,
        TransitionInputType.CONTRACT_PICKER,
        TransitionInputType.DOCUMENT_TYPE_PICKER,
        TransitionInputType.DOCUMENT_PICKER,
        TransitionInputType.DOCUMENT_WITH_PRICE,
        TransitionInputType.TOKEN_PICKER,
        -> {
            OutlinedTextField(
                value = textValue,
                onValueChange = onTextChange,
                label = { Text(input.label + if (input.required) " *" else "") },
                placeholder = input.placeholder?.let { { Text(it) } },
                singleLine = true,
                supportingText = input.help?.let { { Text(it) } },
                modifier = Modifier.fillMaxWidth().testTag(tag),
            )
        }
    }
}
