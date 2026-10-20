package org.dashfoundation.example.ui.components

import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExposedDropdownMenuBox
import androidx.compose.material3.ExposedDropdownMenuDefaults
import androidx.compose.material3.MenuAnchorType
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics

/**
 * Accessible dropdown picker — port of `AccessiblePicker.swift`: a labeled
 * picker whose options carry content descriptions and test tags for UI
 * automation (tags reuse the iOS accessibility identifiers verbatim).
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun <T> AccessiblePicker(
    label: String,
    options: List<T>,
    selected: T,
    optionLabel: (T) -> String,
    modifier: Modifier = Modifier,
    testTag: String? = null,
    enabled: Boolean = true,
    onSelected: (T) -> Unit,
) {
    var expanded by remember(enabled) { mutableStateOf(false) }
    val effectiveExpanded = expanded && enabled

    ExposedDropdownMenuBox(
        expanded = effectiveExpanded,
        onExpandedChange = { expanded = it },
        modifier = modifier.let { m -> testTag?.let { m.testTag(it) } ?: m },
    ) {
        OutlinedTextField(
            value = optionLabel(selected),
            onValueChange = {},
            readOnly = true,
            enabled = enabled,
            label = { Text(label) },
            trailingIcon = {
                ExposedDropdownMenuDefaults.TrailingIcon(expanded = effectiveExpanded)
            },
            modifier = Modifier
                .menuAnchor(MenuAnchorType.PrimaryNotEditable, enabled = enabled)
                .fillMaxWidth()
                .semantics { contentDescription = label },
        )
        ExposedDropdownMenu(
            expanded = effectiveExpanded,
            onDismissRequest = { expanded = false },
        ) {
            options.forEach { option ->
                DropdownMenuItem(
                    text = { Text(optionLabel(option)) },
                    onClick = {
                        onSelected(option)
                        expanded = false
                    },
                )
            }
        }
    }
}
