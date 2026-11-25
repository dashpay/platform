package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

/**
 * Decoding of the JSON `platform_wallet_preview_data_contract_update` and
 * `platform_wallet_split_contract_update_source` write: the Kotlin side of Swift's
 * `DataContractUpdatePreviewDecodingTests`, with the same payloads (the shapes
 * `preview_to_json` and `source_to_json` in `rs-platform-wallet-ffi/src/data_contract.rs`
 * write). Pure JVM: no native library is loaded.
 */
class DataContractUpdatePreviewTest {

    @Test
    fun shouldDecodeAPreviewWithEveryField() {
        val preview = DataContractUpdatePreview.fromJson(
            """{"currentVersion":1,"newVersion":2,"hasChanges":true,""" +
                """"documentTypes":{"added":["comment"],"changed":["note"]},""" +
                """"tokens":{"added":["2","10"],"changed":[]},""" +
                """"groups":{"added":[],"changed":["0"]},""" +
                """"keywordsChanged":true,"descriptionChanged":false,"configChanged":true,""" +
                """"configChangedFields":["moderation","readonly"],""" +
                """"moderatorsAdded":["4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"],""" +
                """"moderatorsRemoved":["GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"]}""",
        )
        assertEquals(
            DataContractUpdatePreview(
                currentVersion = 1,
                newVersion = 2,
                hasChanges = true,
                documentTypes = DataContractUpdatePreview.SectionChanges(listOf("comment"), listOf("note")),
                tokens = DataContractUpdatePreview.SectionChanges(listOf("2", "10"), emptyList()),
                groups = DataContractUpdatePreview.SectionChanges(emptyList(), listOf("0")),
                keywordsChanged = true,
                descriptionChanged = false,
                configChanged = true,
                configChangedFields = listOf("moderation", "readonly"),
                moderatorsAdded = listOf("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"),
                moderatorsRemoved = listOf("GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"),
            ),
            preview,
        )
    }

    /** `hasChanges` is read as Rust wrote it, never recomputed from the sections. */
    @Test
    fun shouldTakeHasChangesAsWritten() {
        val preview = DataContractUpdatePreview.fromJson(
            """{"currentVersion":4,"newVersion":5,"hasChanges":false,""" +
                """"documentTypes":{"added":["x"],"changed":[]},""" +
                """"tokens":{"added":[],"changed":[]},"groups":{"added":[],"changed":[]},""" +
                """"keywordsChanged":false,"descriptionChanged":false,"configChanged":false,""" +
                """"configChangedFields":[],"moderatorsAdded":[],"moderatorsRemoved":[]}""",
        )
        assertFalse(preview.hasChanges)
        assertEquals(4L, preview.currentVersion)
        assertEquals(5L, preview.newVersion)
    }

    @Test
    fun shouldRejectAMalformedPreview() {
        listOf(
            "",
            "not json",
            // A section is missing
            """{"currentVersion":1,"newVersion":2,"hasChanges":true,""" +
                """"documentTypes":{"added":[],"changed":[]},"tokens":{"added":[],"changed":[]},""" +
                """"keywordsChanged":false,"descriptionChanged":false,"configChanged":false,""" +
                """"configChangedFields":[],"moderatorsAdded":[],"moderatorsRemoved":[]}""",
            // A version is not a number
            """{"currentVersion":"1","newVersion":2,"hasChanges":true,""" +
                """"documentTypes":{"added":[],"changed":[]},"tokens":{"added":[],"changed":[]},""" +
                """"groups":{"added":[],"changed":[]},""" +
                """"keywordsChanged":false,"descriptionChanged":false,"configChanged":false,""" +
                """"configChangedFields":[],"moderatorsAdded":[],"moderatorsRemoved":[]}""",
        ).forEach { json ->
            assertThrows(json, DashSdkError.SerializationError::class.java) {
                DataContractUpdatePreview.fromJson(json)
            }
        }
    }

    @Test
    fun shouldDecodeASourceWithEverySection() {
        val source = DataContractUpdateSource.fromJson(
            """{"documents":"{\"note\":{}}","tokens":"{\"0\":{}}","groups":"{\"0\":{}}",""" +
                """"keywords":"[\"a\"]","description":"text","config":"{\"readonly\":false}",""" +
                """"carriesSharedDefinitions":true}""",
        )
        assertEquals(
            DataContractUpdateSource(
                documents = """{"note":{}}""",
                tokens = """{"0":{}}""",
                groups = """{"0":{}}""",
                keywords = """["a"]""",
                description = "text",
                config = """{"readonly":false}""",
                carriesSharedDefinitions = true,
            ),
            source,
        )
    }

    @Test
    fun shouldDecodeASourceWithNullSections() {
        val source = DataContractUpdateSource.fromJson(
            """{"documents":"{}","tokens":null,"groups":null,"keywords":null,""" +
                """"description":null,"config":null,"carriesSharedDefinitions":false}""",
        )
        assertEquals("{}", source.documents)
        assertNull(source.tokens)
        assertNull(source.description)
        assertFalse(source.carriesSharedDefinitions)
    }

    @Test
    fun shouldRejectAMalformedSource() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            DataContractUpdateSource.fromJson("""{"documents":{},"carriesSharedDefinitions":false}""")
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            DataContractUpdateSource.fromJson("""{"documents":"{}"}""")
        }
    }
}
