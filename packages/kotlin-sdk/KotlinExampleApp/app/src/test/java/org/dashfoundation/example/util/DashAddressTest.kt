package org.dashfoundation.example.util

import org.dashfoundation.dashsdk.Network
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class DashAddressTest {

    private val raw43 = ByteArray(43) { (it + 7).toByte() }

    @Test
    fun encodeOrchardRoundTripsThroughParse() {
        val encoded = DashAddress.encodeOrchard(raw43, Network.TESTNET)
        assertNotNull(encoded)
        assertTrue(
            "testnet Orchard addresses carry the tdash HRP",
            encoded!!.startsWith("tdash1"),
        )
        val parsed = DashAddress.parse(encoded, Network.TESTNET)
        assertTrue(parsed is DashAddressType.Orchard)
        assertArrayEquals(raw43, (parsed as DashAddressType.Orchard).raw43)
    }

    @Test
    fun encodeOrchardUsesMainnetHrpOnMainnet() {
        val encoded = DashAddress.encodeOrchard(raw43, Network.MAINNET)
        assertNotNull(encoded)
        assertTrue(encoded!!.startsWith("dash1"))
    }

    @Test
    fun encodeOrchardRejectsWrongLength() {
        assertNull(DashAddress.encodeOrchard(ByteArray(42), Network.TESTNET))
        assertNull(DashAddress.encodeOrchard(ByteArray(44), Network.TESTNET))
    }

    @Test
    fun wrongNetworkHrpIsUnknown() {
        val mainnetEncoded = DashAddress.encodeOrchard(raw43, Network.MAINNET)!!
        assertEquals(
            DashAddressType.Unknown,
            DashAddress.parse(mainnetEncoded, Network.TESTNET),
        )
    }

    @Test
    fun platformBech32mPayloadDetected() {
        // Platform bech32m payload: user-facing P2PKH type byte (0xb0) +
        // 20-byte hash (← rs-dpp platform_address.rs wire bytes).
        val payload = byteArrayOf(0xb0.toByte()) + ByteArray(20) { it.toByte() }
        val encoded = Bech32m.encode("tdash", payload)!!
        val parsed = DashAddress.parse(encoded, Network.TESTNET)
        assertTrue(parsed is DashAddressType.Platform)
        assertArrayEquals(payload, (parsed as DashAddressType.Platform).payload21)
    }

    @Test
    fun platformStorageTypeByteIsRejected() {
        // 0x00/0x01 are the STORAGE bytes (GroveDB keys) and must never
        // appear in a tdash1…/dash1… string (← DashAddress.swift:30).
        val payload = byteArrayOf(0x00) + ByteArray(20) { it.toByte() }
        val encoded = Bech32m.encode("tdash", payload)!!
        assertEquals(DashAddressType.Unknown, DashAddress.parse(encoded, Network.TESTNET))
    }

    @Test
    fun coreBase58AddressDetected() {
        // 25 decoded bytes = version + hash160 + checksum, the same light
        // check the send screen shipped with (checksum unverified — the
        // authoritative validation is Rust-side at send time).
        val core = Base58.encode(ByteArray(25) { (it + 1).toByte() })
        val parsed = DashAddress.parse(core, Network.TESTNET)
        assertTrue(parsed is DashAddressType.Core)
        assertEquals(core, (parsed as DashAddressType.Core).address)
    }

    @Test
    fun garbageIsUnknown() {
        assertEquals(DashAddressType.Unknown, DashAddress.parse("", Network.TESTNET))
        assertEquals(DashAddressType.Unknown, DashAddress.parse("   ", Network.TESTNET))
        assertEquals(
            DashAddressType.Unknown,
            DashAddress.parse("not-an-address", Network.TESTNET),
        )
    }
}
