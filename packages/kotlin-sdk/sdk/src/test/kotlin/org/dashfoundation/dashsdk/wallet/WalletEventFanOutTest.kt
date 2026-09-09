package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.ExperimentalCoroutinesApi
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.toList
import kotlinx.coroutines.launch
import kotlinx.coroutines.test.UnconfinedTestDispatcher
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.ffi.NativeWalletEventBridge
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Event-bridge fan-out: a [NativeWalletEventBridge] subclass wired to a
 * [MutableSharedFlow] (the exact pattern [PlatformWalletManager] uses) must
 * map every native callback to the matching typed [WalletSyncEvent]. Runs
 * without a native handle — it drives the bridge callbacks directly.
 */
class WalletEventFanOutTest {

    /** Mirror of the manager's inline event bridge. */
    private class FanOutBridge(
        private val sink: MutableSharedFlow<WalletSyncEvent>,
    ) : NativeWalletEventBridge() {
        override fun onWalletEvent(eventDebug: String) {
            sink.tryEmit(WalletSyncEvent.Generic(eventDebug))
        }

        override fun onError(message: String) {
            sink.tryEmit(WalletSyncEvent.Error(message))
        }

        override fun onPlatformAddressSyncCompleted(
            walletId: ByteArray,
            success: Boolean,
            foundCount: Long,
            absentCount: Long,
            checkpointHeight: Long,
            newSyncHeight: Long,
            newSyncTimestamp: Long,
            lastKnownRecentBlock: Long,
            errorMessage: String?,
        ) {
            sink.tryEmit(
                WalletSyncEvent.PlatformAddressResult(
                    walletId, success, foundCount, absentCount, checkpointHeight,
                    newSyncHeight, newSyncTimestamp, lastKnownRecentBlock, errorMessage,
                ),
            )
        }

        override fun onPlatformAddressSyncPassCompleted(syncUnixSeconds: Long, walletCount: Int) {
            sink.tryEmit(WalletSyncEvent.PlatformAddressPassCompleted(syncUnixSeconds, walletCount))
        }

        override fun onDpnsMarketplaceSyncCompleted(
            walletId: ByteArray,
            success: Boolean,
            namesTracked: Int,
            namesAdded: Int,
            namesDeparted: Int,
            pricesChanged: Int,
            errorMessage: String?,
        ) {
            sink.tryEmit(
                WalletSyncEvent.DpnsMarketplaceResult(
                    walletId, success, namesTracked, namesAdded,
                    namesDeparted, pricesChanged, errorMessage,
                ),
            )
        }

        override fun onDpnsMarketplaceSyncPassCompleted(
            syncUnixSeconds: Long,
            walletCount: Int,
        ) {
            sink.tryEmit(WalletSyncEvent.DpnsMarketplacePassCompleted(syncUnixSeconds, walletCount))
        }

        override fun onShieldedSyncCompleted(
            walletId: ByteArray,
            success: Boolean,
            skipped: Boolean,
            cooldownSkip: Boolean,
            newNotes: Int,
            totalScanned: Long,
            newlySpent: Int,
            balance: Long,
            errorMessage: String?,
        ) {
            sink.tryEmit(
                WalletSyncEvent.ShieldedResult(
                    walletId, success, skipped, cooldownSkip, newNotes,
                    totalScanned, newlySpent, balance, errorMessage,
                ),
            )
        }

        override fun onShieldedSyncPassCompleted(syncUnixSeconds: Long, walletCount: Int) {
            sink.tryEmit(WalletSyncEvent.ShieldedPassCompleted(syncUnixSeconds, walletCount))
        }

        override fun onShieldedSyncProgress(cumulativeScanned: Long, blockHeight: Long) {
            sink.tryEmit(WalletSyncEvent.ShieldedProgress(cumulativeScanned, blockHeight))
        }

        override fun onShieldedTreeProgress(leavesCommitted: Long, totalTarget: Long) {
            sink.tryEmit(WalletSyncEvent.ShieldedTreeProgress(leavesCommitted, totalTarget))
        }
    }

    @OptIn(ExperimentalCoroutinesApi::class)
    @Test
    fun everyCallbackFansToItsTypedEvent() = runTest {
        val sink = MutableSharedFlow<WalletSyncEvent>(replay = 0, extraBufferCapacity = 64)
        val bridge = FanOutBridge(sink)
        val walletId = ByteArray(32) { 5 }

        val received = mutableListOf<WalletSyncEvent>()
        // Unconfined dispatcher: the collector runs eagerly to its first
        // suspension on launch, registering the SharedFlow subscription
        // before any emit (replay=0 drops pre-subscription emissions).
        val collector = launch(UnconfinedTestDispatcher(testScheduler)) { sink.toList(received) }

        // Drive one of every callback.
        bridge.onWalletEvent("evt")
        bridge.onError("err")
        bridge.onPlatformAddressSyncCompleted(walletId, true, 1, 2, 3, 4, 5, 6, null)
        bridge.onPlatformAddressSyncPassCompleted(1_000, 1)
        bridge.onDpnsMarketplaceSyncCompleted(walletId, true, 7, 8, 9, 10, null)
        bridge.onDpnsMarketplaceSyncPassCompleted(1_500, 1)
        bridge.onShieldedSyncCompleted(walletId, true, false, false, 7, 8, 9, 10, null)
        bridge.onShieldedSyncPassCompleted(2_000, 1)
        bridge.onShieldedSyncProgress(11, 12)
        bridge.onShieldedTreeProgress(13, 14)

        // Let the collector drain, then stop it.
        kotlinx.coroutines.yield()
        collector.cancel()

        assertEquals(10, received.size)
        assertTrue(received[0] is WalletSyncEvent.Generic)
        assertEquals("evt", (received[0] as WalletSyncEvent.Generic).debug)
        assertTrue(received[1] is WalletSyncEvent.Error)
        assertEquals("err", (received[1] as WalletSyncEvent.Error).message)

        val par = received[2] as WalletSyncEvent.PlatformAddressResult
        assertTrue(par.walletId.contentEquals(walletId))
        assertEquals(4L, par.newSyncHeight)
        assertEquals(5L, par.newSyncTimestamp)

        assertTrue(received[3] is WalletSyncEvent.PlatformAddressPassCompleted)

        val dpns = received[4] as WalletSyncEvent.DpnsMarketplaceResult
        assertEquals(7, dpns.namesTracked)
        assertEquals(9, dpns.namesDeparted)
        assertTrue(received[5] is WalletSyncEvent.DpnsMarketplacePassCompleted)

        val sr = received[6] as WalletSyncEvent.ShieldedResult
        assertEquals(7, sr.newNotes)
        assertEquals(8L, sr.totalScanned)
        assertEquals(10L, sr.balance)

        assertTrue(received[7] is WalletSyncEvent.ShieldedPassCompleted)
        val sp = received[8] as WalletSyncEvent.ShieldedProgress
        assertEquals(11L, sp.cumulativeScanned)
        val tp = received[9] as WalletSyncEvent.ShieldedTreeProgress
        assertEquals(13L, tp.leavesCommitted)
    }
}
