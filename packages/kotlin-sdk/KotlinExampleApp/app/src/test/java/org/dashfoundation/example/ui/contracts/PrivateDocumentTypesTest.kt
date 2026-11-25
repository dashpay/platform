package org.dashfoundation.example.ui.contracts

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Pins the private type reads behind the contract screen's Private badge and the type
 * screen's Private section ([isPrivateDocumentType], [privateEntrySize]), the Kotlin side of
 * `DocumentTypePrivateTests.swift`, and the entry screen's property rows ([propertyRows]).
 */
class PrivateDocumentTypesTest {

    private fun schema(json: String) = Json.parseToJsonElement(json) as JsonObject

    @Test
    fun shouldReadAPrivateDeclarationAndItsEntrySize() {
        val private = schema("""{"type":"object","private":{"entrySize":512},"properties":{}}""")
        assertTrue(isPrivateDocumentType(private))
        assertEquals(512L, privateEntrySize(private))
    }

    @Test
    fun shouldTreatATypeWithoutTheKeywordAsOrdinary() {
        val ordinary = schema("""{"type":"object","properties":{"note":{"type":"string"}}}""")
        assertFalse(isPrivateDocumentType(ordinary))
        assertNull(privateEntrySize(ordinary))
        assertFalse(isPrivateDocumentType(null))
    }

    /** A declaration DPP would refuse shows no size rather than a wrong one. */
    @Test
    fun shouldShowNoEntrySizeForADeclarationWithoutAnIntegerOne() {
        for (declaration in listOf("{}", """{"entrySize":"512"}""", """{"entrySize":true}""", """{"entrySize":1.5}""")) {
            val private = schema("""{"private":$declaration}""")
            assertTrue(declaration, isPrivateDocumentType(private))
            assertNull(declaration, privateEntrySize(private))
        }
    }

    @Test
    fun shouldListPropertiesSortedWithStringsBareAndOtherValuesAsJson() {
        assertEquals(
            listOf("a" to "1", "b" to "text", "c" to "{\"x\":true}"),
            propertyRows("""{"c":{"x":true},"b":"text","a":1}"""),
        )
        assertEquals(listOf("properties" to "[1]"), propertyRows("[1]"))
        assertEquals(listOf("properties" to "not json"), propertyRows("not json"))
    }
}
