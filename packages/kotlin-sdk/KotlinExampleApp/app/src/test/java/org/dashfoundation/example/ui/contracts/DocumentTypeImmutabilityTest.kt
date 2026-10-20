package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.buildJsonArray
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.add
import kotlinx.serialization.json.put
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the protocol-v14 `immutable` / `immutableAllowSetting` helpers
 * ([documentTypeImmutability], [immutablePropertyLock]) behind the
 * DocumentTypeDetailsScreen labels and the DocumentActionsScreen replace-form
 * locks. Consensus rejects a replace that touches a frozen property as a PAID
 * invalid transition (code 40128), so the form must lock exactly the
 * properties DPP freezes: every `immutable` entry, except an
 * `immutableAllowSetting` entry on a document that has no value for it yet.
 */
class DocumentTypeImmutabilityTest {

    private val schema = buildJsonObject {
        put("documentsMutable", true)
        put(
            "immutable",
            buildJsonArray {
                add("mood")
                add("author")
            },
        )
        put("immutableAllowSetting", buildJsonArray { add("mood") })
    }

    @Test
    fun parsesBothLists() {
        val immutability = documentTypeImmutability(schema)
        assertEquals(setOf("author", "mood"), immutability.immutable)
        assertEquals(setOf("mood"), immutability.allowSetting)
    }

    @Test
    fun absentKeywordsFreezeNothing() {
        val immutability = documentTypeImmutability(buildJsonObject { put("type", "object") })
        assertTrue(immutability.isEmpty)
        assertTrue(immutability.allowSetting.isEmpty())
        assertTrue(documentTypeImmutability(null).isEmpty)
    }

    @Test
    fun allowSettingOutsideImmutableIsDropped() {
        // DPP refuses such a contract at registration; hand-edited JSON is the
        // only way to see it, and the allowance must not unlock anything.
        val immutability = documentTypeImmutability(
            buildJsonObject {
                put("immutable", buildJsonArray { add("author") })
                put("immutableAllowSetting", buildJsonArray { add("body") })
            },
        )
        assertEquals(setOf("author"), immutability.immutable)
        assertTrue(immutability.allowSetting.isEmpty())
    }

    @Test
    fun nonStringEntriesAreIgnored() {
        val immutability = documentTypeImmutability(
            buildJsonObject {
                put(
                    "immutable",
                    buildJsonArray {
                        add("author")
                        add(7)
                    },
                )
            },
        )
        assertEquals(setOf("author"), immutability.immutable)
    }

    @Test
    fun frozenPropertyIsLockedRegardlessOfStoredValue() {
        val immutability = documentTypeImmutability(schema)
        assertEquals(
            ImmutablePropertyLock.FROZEN,
            immutablePropertyLock("author", immutability, hasStoredValue = true),
        )
        assertEquals(
            ImmutablePropertyLock.FROZEN,
            immutablePropertyLock("author", immutability, hasStoredValue = false),
        )
    }

    @Test
    fun settableOncePropertyOpensOnlyWhileAbsent() {
        val immutability = documentTypeImmutability(schema)
        assertEquals(
            ImmutablePropertyLock.SETTABLE_ONCE,
            immutablePropertyLock("mood", immutability, hasStoredValue = false),
        )
        assertEquals(
            ImmutablePropertyLock.FROZEN,
            immutablePropertyLock("mood", immutability, hasStoredValue = true),
        )
    }

    @Test
    fun otherPropertiesStayEditable() {
        val immutability = documentTypeImmutability(schema)
        assertEquals(
            ImmutablePropertyLock.EDITABLE,
            immutablePropertyLock("body", immutability, hasStoredValue = true),
        )
        assertEquals(
            ImmutablePropertyLock.EDITABLE,
            immutablePropertyLock("body", DocumentTypeImmutability.NONE, hasStoredValue = false),
        )
    }
}
