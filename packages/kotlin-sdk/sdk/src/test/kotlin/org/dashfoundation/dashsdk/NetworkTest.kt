package org.dashfoundation.dashsdk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Test

class NetworkTest {

    @Test
    fun ffiOrdinalsMatchDashNetworkFfiEnum() {
        // Must stay in sync with dash-network/src/ffi.rs.
        assertEquals(0, Network.MAINNET.ffiValue)
        assertEquals(1, Network.TESTNET.ffiValue)
        assertEquals(2, Network.DEVNET.ffiValue)
        assertEquals(3, Network.REGTEST.ffiValue)
    }

    @Test
    fun roundTripsThroughFfiValue() {
        Network.entries.forEach { network ->
            assertEquals(network, Network.fromFfiValue(network.ffiValue))
        }
    }

    @Test
    fun rejectsUnknownFfiValue() {
        assertThrows(IllegalArgumentException::class.java) { Network.fromFfiValue(42) }
    }

    @Test
    fun networkNamesMatchIosConventions() {
        assertEquals("mainnet", Network.MAINNET.networkName)
        assertEquals("Local", Network.REGTEST.displayName)
        assertEquals(Network.TESTNET, Network.fromNetworkName("TESTNET"))
        assertNull(Network.fromNetworkName("nonsense"))
    }
}
