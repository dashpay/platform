package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.put
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the DPP effective-capability resolution in [documentTypeCapabilities]
 * that gates the Replace/Delete actions (DocumentActionsScreen) and the
 * capability labels (DocumentTypeDetailsScreen). Drive treats a submit
 * against a disabled capability as a PAID invalid transition, so an explicit
 * `canBeDeleted: false` — or a contract config default of false — must win,
 * and the non-existent `documentsCanBeDeleted` schema key must never
 * re-enable it (the original OR-based gate did, leaving Delete live for a
 * guaranteed on-chain rejection).
 */
class DocumentTypeCapabilitiesTest {

    @Test
    fun explicitFalseOnSchemaDisables() {
        val schema = buildJsonObject {
            put("documentsMutable", false)
            put("canBeDeleted", false)
        }
        val caps = documentTypeCapabilities(schema, config = null)
        assertFalse(caps.documentsMutable)
        assertFalse(caps.canBeDeleted)
    }

    @Test
    fun phantomDocumentsCanBeDeletedKeyDoesNotReEnableDelete() {
        // Regression for the original gate, which OR-ed in the non-existent
        // `documentsCanBeDeleted` key and so read a `canBeDeleted: false`
        // type as deletable. The alias must be ignored.
        val schema = buildJsonObject {
            put("canBeDeleted", false)
            put("documentsCanBeDeleted", true)
        }
        assertFalse(documentTypeCapabilities(schema, config = null).canBeDeleted)
    }

    @Test
    fun absentSchemaFlagFallsBackToContractConfigDefault() {
        val config = buildJsonObject {
            put("documentsMutableContractDefault", false)
            put("documentsCanBeDeletedContractDefault", false)
        }
        val caps = documentTypeCapabilities(schema = JsonObject(emptyMap()), config = config)
        assertFalse(caps.documentsMutable)
        assertFalse(caps.canBeDeleted)
    }

    @Test
    fun absentEverywhereDefaultsToTrue() {
        val caps = documentTypeCapabilities(schema = JsonObject(emptyMap()), config = null)
        assertTrue(caps.documentsMutable)
        assertTrue(caps.canBeDeleted)
    }

    @Test
    fun schemaValueOverridesContractConfigDefault() {
        val schema = buildJsonObject { put("canBeDeleted", true) }
        val config = buildJsonObject { put("documentsCanBeDeletedContractDefault", false) }
        assertTrue(documentTypeCapabilities(schema, config).canBeDeleted)
    }
}
