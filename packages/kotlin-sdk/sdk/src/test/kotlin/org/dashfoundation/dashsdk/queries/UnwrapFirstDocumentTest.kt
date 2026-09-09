package org.dashfoundation.dashsdk.queries

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Regression coverage for [unwrapFirstDocument], the response unwrapper behind
 * `Documents.fetch`. The prior implementation assumed a bare JSON array and so
 * always returned null against the `dash_sdk_document_search` object envelope
 * (`{"documents":[...],"total_count":N}`), which broke single-document fetch
 * and disabled DOC-03/04/05 (DocumentActionsScreen) and DOC-06/07
 * (DocumentWithPriceScreen).
 */
class UnwrapFirstDocumentTest {

    @Test
    fun unwrapsFirstDocumentFromSearchEnvelope() {
        val response =
            """{"documents":[{"${'$'}id":"FKYiQ3pY","label":"one"},{"${'$'}id":"other"}],"total_count":2}"""
        val doc = unwrapFirstDocument(response)
        assertEquals("one", Json.parseToJsonElement(doc!!).jsonObject["label"]?.jsonPrimitive?.content)
        assertEquals("FKYiQ3pY", Json.parseToJsonElement(doc).jsonObject["\$id"]?.jsonPrimitive?.content)
    }

    @Test
    fun emptyDocumentsArrayInEnvelopeIsNull() {
        assertNull(unwrapFirstDocument("""{"documents":[],"total_count":0}"""))
    }

    @Test
    fun bareArrayShapeStillAccepted() {
        val doc = unwrapFirstDocument("""[{"${'$'}id":"abc"}]""")
        assertEquals("abc", Json.parseToJsonElement(doc!!).jsonObject["\$id"]?.jsonPrimitive?.content)
    }

    @Test
    fun bareEmptyArrayIsNull() {
        assertNull(unwrapFirstDocument("[]"))
    }

    @Test
    fun nullBlankAndMalformedAreNull() {
        assertNull(unwrapFirstDocument(null))
        assertNull(unwrapFirstDocument("   "))
        assertNull(unwrapFirstDocument("not json"))
        assertNull(unwrapFirstDocument("""{"total_count":0}"""))
    }
}
