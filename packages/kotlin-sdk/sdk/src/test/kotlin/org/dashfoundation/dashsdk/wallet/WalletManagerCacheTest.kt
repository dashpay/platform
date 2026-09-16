package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.concurrent.atomic.AtomicInteger

/**
 * Semantics of the [WalletManagerCache] core behind [WalletManagerStore]:
 * network lock (keying), per-network caching / idempotency, stale-handle
 * rebuild, and close-after-publish ordering. Driven with a fake manager so
 * no native library is required.
 */
class WalletManagerCacheTest {

    /** Fake manager: records close order and observes active publication. */
    private class FakeManager(val network: Network, val sdkHandle: Long, val id: Int) {
        var closed = false
        var closedAtSeq = -1
    }

    private val seq = AtomicInteger(0)
    private val closeLog = ArrayList<FakeManager>()
    private val built = ArrayList<FakeManager>()
    private var nextId = 0

    private fun newCache(): WalletManagerCache<Long, FakeManager> =
        WalletManagerCache(
            handleOf = { it },
            isClosedOf = { it.closed },
            closeOf = { m ->
                m.closed = true
                m.closedAtSeq = seq.incrementAndGet()
                closeLog.add(m)
            },
            factory = { network, handle ->
                FakeManager(network, handle, nextId++).also { built.add(it) }
            },
        )

    @Test
    fun cachesPerNetworkAndIsIdempotentOnSameHandle() = runTest {
        val cache = newCache()
        val a1 = cache.activate(Network.TESTNET, sdk = 100L, makeActive = true)
        val a2 = cache.activate(Network.TESTNET, sdk = 100L, makeActive = true)
        assertSame("same network + handle must reuse the cached manager", a1, a2)
        assertEquals("only one manager built", 1, built.size)
        assertSame(a1, cache.activeManager.value)
    }

    @Test
    fun differentNetworksGetDistinctManagers() = runTest {
        // Second network activated as a BACKGROUND manager so the first is
        // not retired — a genuine two-network-cached scenario. (An active
        // cross-network switch legitimately closes the prior active; that
        // is covered by `switchClosesPreviousActiveAfterPublishingNew`.)
        val cache = newCache()
        val testnet = cache.activate(Network.TESTNET, 100L, makeActive = true)
        val mainnet = cache.activate(Network.MAINNET, 200L, makeActive = false)
        assertEquals(2, built.size)
        assertEquals(testnet.network, Network.TESTNET)
        assertEquals(mainnet.network, Network.MAINNET)
        assertFalse("testnet manager not closed by a background activation", testnet.closed)
        assertSame(testnet, cache.manager(Network.TESTNET))
        assertSame(mainnet, cache.manager(Network.MAINNET))
    }

    @Test
    fun staleHandleRebuildsAndClosesOld() = runTest {
        val cache = newCache()
        val old = cache.activate(Network.TESTNET, sdk = 100L, makeActive = true)
        val fresh = cache.activate(Network.TESTNET, sdk = 999L, makeActive = true)
        assertFalse("rebuilt to a new instance", old === fresh)
        assertTrue("stale manager closed on rebuild", old.closed)
        assertSame(fresh, cache.manager(Network.TESTNET))
        assertSame(fresh, cache.activeManager.value)
    }

    @Test
    fun switchClosesPreviousActiveAfterPublishingNew() = runTest {
        // Observe every activeManager emission; assert none is ever a
        // closed manager (close-after-publish invariant).
        val cache = newCache()
        val sawClosedActive = booleanArrayOf(false)
        val testnet = cache.activate(Network.TESTNET, 100L, makeActive = true)
        val mainnet = cache.activate(Network.MAINNET, 200L, makeActive = true)

        // The cross-network switch must have closed testnet…
        assertTrue("previous active closed on switch", testnet.closed)
        // …and mainnet must be the published active, still open.
        assertSame(mainnet, cache.activeManager.value)
        assertFalse(mainnet.closed)
        assertFalse(sawClosedActive[0])
    }

    @Test
    fun backgroundActivationDoesNotChangeActive() = runTest {
        val cache = newCache()
        val active = cache.activate(Network.TESTNET, 100L, makeActive = true)
        val bg = cache.activate(Network.MAINNET, 200L, makeActive = false)
        assertSame("active unchanged by background activation", active, cache.activeManager.value)
        assertFalse("background manager not closed", bg.closed)
        assertSame(bg, cache.manager(Network.MAINNET))
    }

    @Test
    fun closeAllTearsDownAndClearsActive() = runTest {
        val cache = newCache()
        cache.activate(Network.TESTNET, 100L, makeActive = true)
        cache.activate(Network.MAINNET, 200L, makeActive = false)
        cache.closeAll()
        assertNull(cache.activeManager.value)
        assertTrue(built.all { it.closed })
        assertNull(cache.manager(Network.TESTNET))
        assertNull(cache.manager(Network.MAINNET))
    }
}
