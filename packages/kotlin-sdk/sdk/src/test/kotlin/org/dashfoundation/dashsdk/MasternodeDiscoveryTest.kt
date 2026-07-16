package org.dashfoundation.dashsdk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

/**
 * Mirrors the decoding tolerance of `SDK.discoverActiveMasternodes` in
 * `SwiftDashSDK/SDK.swift`: ENABLED + versionCheck=success filter, default
 * DAPI port 443, lenient optional fields.
 */
class MasternodeDiscoveryTest {

    @Test
    fun filtersToEnabledAndVersionCheckedNodes() {
        val body = """
            {"success": true, "data": [
              {"address": "34.1.2.3:19999", "status": "ENABLED", "platformHTTPPort": 1443, "versionCheck": "success"},
              {"address": "34.1.2.4:19999", "status": "ENABLED", "platformHTTPPort": 1443, "versionCheck": "failed"},
              {"address": "34.1.2.5:19999", "status": "POSE_BANNED", "platformHTTPPort": 1443, "versionCheck": "success"}
            ]}
        """.trimIndent()
        val active = Sdk.parseActiveMasternodes(body)!!
        assertEquals(1, active.size)
        assertEquals("34.1.2.3:19999", active[0].spvPeer)
        assertEquals("https://34.1.2.3:1443", active[0].dapiUrl)
    }

    @Test
    fun missingPlatformPortFallsBackTo443() {
        val body = """
            {"success": true, "data": [
              {"address": "1.2.3.4:9999", "status": "ENABLED", "versionCheck": "success"}
            ]}
        """.trimIndent()
        assertEquals("https://1.2.3.4:443", Sdk.parseActiveMasternodes(body)!![0].dapiUrl)
        assertEquals("https://1.2.3.4:1443", Sdk.parseActiveMasternodes(body, 1443)!![0].dapiUrl)
    }

    @Test
    fun bracketedIpv6AddressPreservesTheWholeHost() {
        val body = """
            {"success": true, "data": [
              {"address": "[2001:db8::1]:19999", "status": "ENABLED", "platformHTTPPort": 1443, "versionCheck": "success"}
            ]}
        """.trimIndent()

        assertEquals(
            "https://[2001:db8::1]:1443",
            Sdk.parseActiveMasternodes(body)!![0].dapiUrl,
        )
    }

    @Test
    fun missingVersionCheckIsExcluded() {
        val body = """
            {"success": true, "data": [
              {"address": "1.2.3.4:9999", "status": "ENABLED"}
            ]}
        """.trimIndent()
        assertNull(Sdk.parseActiveMasternodes(body))
    }

    @Test
    fun toleratesUnknownFieldsAndFailsClosedOnBadShape() {
        val ok = """
            {"success": true, "extra": 1, "data": [
              {"address": "1.2.3.4:9999", "status": "ENABLED", "versionCheck": "success", "unknown": {"x": 1}}
            ]}
        """.trimIndent()
        assertEquals(1, Sdk.parseActiveMasternodes(ok)!!.size)

        assertNull(Sdk.parseActiveMasternodes("""{"success": false, "data": []}"""))
        assertNull(Sdk.parseActiveMasternodes("not json"))
        assertNull(Sdk.parseActiveMasternodes("""{"success": true, "data": []}"""))
    }
}
