package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.longOrNull
import org.dashfoundation.example.ui.components.FormSection
import org.dashfoundation.example.ui.components.LabeledContent

/**
 * Whether a document type is private (protocol version 15): its schema declares the `private`
 * keyword (← Swift `PersistentDocumentType.isPrivate`). A private type's documents are
 * fixed-size encrypted entries, appended by an anonymous transition paid from the shielded
 * pool; the type takes no ordinary document and refuses document queries. Read off the stored
 * schema for display, as [documentTypeImmutability] reads `immutable`; DPP validated it when
 * the contract was registered.
 */
internal fun isPrivateDocumentType(schema: JsonObject?): Boolean = schema?.containsKey("private") == true

/**
 * The `entrySize` of a private type's `"private": { "entrySize": n }` declaration, or null when
 * the type is not private or the declaration holds no integer `entrySize` (← Swift
 * `PersistentDocumentType.privateEntrySize`). Read as declared, not validated.
 */
internal fun privateEntrySize(schema: JsonObject?): Long? {
    val value = schema?.objectField("private")?.get("entrySize") as? JsonPrimitive ?: return null
    if (value.isString) return null
    return value.longOrNull
}

private val PrivateIndigo = Color(0xFF3F51B5)

/** The "Private" badge on a private type's row (← `DocumentTypeRowView`'s badge). */
@Composable
internal fun PrivateTypeBadge() {
    Text(
        "Private",
        style = MaterialTheme.typography.labelSmall,
        color = PrivateIndigo,
        modifier = Modifier
            .background(PrivateIndigo.copy(alpha = 0.15f), RoundedCornerShape(4.dp))
            .padding(horizontal = 6.dp, vertical = 2.dp)
            .testTag("documentType.privateBadge"),
    )
}

/**
 * What a private type is, and the way to read the entries this app's wallets can open (←
 * `DocumentTypeDetailsView.privateTypeSection`). It stands in for the New Document and query
 * actions, which a private type does not take.
 */
@Composable
internal fun PrivateTypeSection(schema: JsonObject, onReadEntries: () -> Unit) {
    FormSection(title = "Private") {
        Column(Modifier.testTag("documentType.privateSection")) {
            Text(
                "Private document type",
                style = MaterialTheme.typography.titleSmall,
                fontWeight = FontWeight.SemiBold,
                color = PrivateIndigo,
            )
            privateEntrySize(schema)?.let { size ->
                LabeledContent(
                    "Entry size",
                    "$size bytes",
                    modifier = Modifier.testTag("documentType.privateEntrySize"),
                )
            }
            Text(
                "Each entry is a fixed-size encrypted blob, appended by an anonymous transition " +
                    "paid from the shielded pool. Nobody owns, replaces or deletes an entry, and " +
                    "nothing on chain says who it is for: a reader finds its own by trying to " +
                    "open each one.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )
            Text(
                "The type takes no ordinary documents and refuses document queries.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurfaceVariant,
                modifier = Modifier.padding(top = 4.dp),
            )
        }
        TextButton(
            onClick = onReadEntries,
            modifier = Modifier.testTag("documentType.privateEntriesLink"),
        ) { Text("Read Entries") }
    }
}
