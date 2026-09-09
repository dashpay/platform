package org.dashfoundation.example.ui.storage

import org.dashfoundation.dashsdk.persistence.UInt64Value
import org.junit.Assert.assertEquals
import org.junit.Test

class StorageModelsTest {

    @Test
    fun `token balance subtitle decodes unsigned blob`() {
        val model = STORAGE_MODELS.single { it.name == "tokenBalances" }
        val row = StorageRow(
            rowId = 1,
            columns = mapOf("balance" to UInt64Value(ULong.MAX_VALUE).toBigEndianBytes()),
        )

        assertEquals("18446744073709551615", model.subtitle(row))
    }
}
