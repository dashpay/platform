package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonObject
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.QueriesNative
import org.dashfoundation.dashsdk.queries.Contracts
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Round trips of the property type shorthand expansion (protocol version 14)
 * through `rs-unified-sdk-jni` and `rs-sdk-ffi` on a device: the contract
 * JSON in, the long form out, and the error code the Kotlin side maps. Rust's
 * own tests cover where a shorthand is read; these pin the marshalling and
 * that every place the example app reads a schema from comes back in long
 * form. No SDK handle and no network access. Kotlin counterpart of the Swift
 * SDK's `DataContractParserPropertyTypeShorthandTests`.
 */
@RunWith(AndroidJUnit4::class)
class PropertyTypeShorthandsFfiTest {

    /** A contract JSON writing each shorthand where a schema can hold one. */
    private val contractJson = """
        {
          "${'$'}formatVersion": "1",
          "id": "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
          "ownerId": "4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF",
          "version": 1,
          "schemaDefs": {
            "owner": { "type": "identifier" },
            "digest": { "type": "bytes", "size": 16 }
          },
          "documentSchemas": {
            "payment": {
              "type": "object",
              "properties": {
                "recipient": { "type": "identifier", "position": 0 },
                "txHash": { "type": "bytes", "size": 20, "position": 1 },
                "memo": {
                  "type": "object",
                  "properties": {
                    "author": { "type": "identifier", "position": 0 },
                    "tag": { "type": "bytes", "size": 4, "position": 1 }
                  },
                  "additionalProperties": false,
                  "position": 2
                },
                "cosigners": {
                  "type": "array",
                  "items": { "type": "identifier" },
                  "maxItems": 3,
                  "position": 3
                },
                "label": { "type": "string", "maxLength": 20, "position": 4 }
              },
              "additionalProperties": false
            }
          }
        }
    """.trimIndent()

    @Before
    fun loadLibrary() {
        NativeLoader.ensureLoaded()
    }

    @Test
    fun shouldWriteEveryShorthandInFull() {
        val expanded = parse(Contracts.expandPropertyTypeShorthands(contractJson))
        val properties = expanded.obj("documentSchemas").obj("payment").obj("properties")

        assertIdentifier(properties.obj("recipient"))
        assertBytes(properties.obj("txHash"), 20)
        assertIdentifier(properties.obj("memo").obj("properties").obj("author"))
        assertBytes(properties.obj("memo").obj("properties").obj("tag"), 4)
        assertIdentifier(properties.obj("cosigners").obj("items"))
        assertIdentifier(expanded.obj("schemaDefs").obj("owner"))
        assertBytes(expanded.obj("schemaDefs").obj("digest"), 16)

        // The typed array keeps its own element count, the string its schema
        assertEquals("3", properties.obj("cosigners").text("maxItems"))
        assertEquals("string", properties.obj("label").text("type"))
        // Keys around the schemas come back as given
        assertEquals("4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF", expanded.text("ownerId"))
    }

    @Test
    fun shouldReturnAContractWithoutShorthandsUnchanged() {
        val longForm = """
            {"documentSchemas":{"note":{"type":"object","properties":{
            "id":{"type":"array","byteArray":true,"minItems":32,"maxItems":32,
            "contentMediaType":"application/x.dash.dpp.identifier","position":0}},
            "additionalProperties":false}}}
        """.trimIndent()
        assertEquals(parse(longForm), parse(Contracts.expandPropertyTypeShorthands(longForm)))
    }

    @Test
    fun shouldRefuseTextThatIsNotAJsonObject() {
        // DashSDKErrorCode 1 is InvalidParameter
        val native = assertThrows(DashSDKException::class.java) {
            QueriesNative.dataContractJsonExpandPropertyTypeShorthands("[1]")
        }
        assertEquals(native.message, 1, native.code)
        assertThrows(DashSdkError.InvalidParameter::class.java) {
            Contracts.expandPropertyTypeShorthands("not json")
        }
    }

    private fun assertIdentifier(schema: JsonObject) {
        assertBytes(schema, 32)
        assertEquals("application/x.dash.dpp.identifier", schema.text("contentMediaType"))
    }

    private fun assertBytes(schema: JsonObject, size: Int) {
        assertEquals("array", schema.text("type"))
        assertEquals("true", schema.text("byteArray"))
        assertEquals("$size", schema.text("minItems"))
        assertEquals("$size", schema.text("maxItems"))
        assertNull(schema["size"])
        if (size != 32) assertNull(schema["contentMediaType"])
    }

    private fun parse(json: String): JsonObject = Json.parseToJsonElement(json).jsonObject

    private fun JsonObject.obj(key: String): JsonObject = getValue(key).jsonObject

    private fun JsonObject.text(key: String): String? = (this[key] as? JsonPrimitive)?.content
}
