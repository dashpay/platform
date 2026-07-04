package org.dashfoundation.dashsdk.services

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity
import org.dashfoundation.dashsdk.persistence.syncStateScopeId
import org.dashfoundation.dashsdk.wallet.WalletSyncEvent
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * Sync-state reduction, watermark reads, and balance Flows for
 * [PlatformBalanceSyncService]. Reduction is driven directly (no native
 * handle needed); watermark + balance come from an in-memory [DashDatabase].
 */
@RunWith(RobolectricTestRunner::class)
class PlatformBalanceSyncServiceTest {

    private lateinit var db: DashDatabase
    private lateinit var service: PlatformBalanceSyncService

    private val walletId = ByteArray(32) { 3 }

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        service = PlatformBalanceSyncService(db)
    }

    @After
    fun tearDown() {
        service.close()
        db.close()
    }

    private fun address(hash: Byte, balance: Long) = PlatformAddressEntity(
        address = "addr-$hash",
        addressType = 0,
        addressHash = ByteArray(20) { hash },
        accountIndex = 0,
        addressIndex = hash.toInt(),
        derivationPath = "m/9'/5'/17'/0'/0'/$hash",
        balance = balance,
        walletId = walletId,
    )

    @Test
    fun startsIdle() {
        assertTrue(service.state.value is PlatformBalanceSyncService.PlatformSyncState.Idle)
    }

    @Test
    fun successfulResultReducesToSynced() {
        service.reduce(
            WalletSyncEvent.PlatformAddressResult(
                walletId = walletId,
                success = true,
                foundCount = 5,
                absentCount = 1,
                checkpointHeight = 100,
                newSyncHeight = 1_234,
                newSyncTimestamp = 1_700_000_000,
                lastKnownRecentBlock = 90,
                errorMessage = null,
            ),
        )
        val state = service.state.value
        assertTrue(state is PlatformBalanceSyncService.PlatformSyncState.Synced)
        state as PlatformBalanceSyncService.PlatformSyncState.Synced
        assertEquals(1_234L, state.syncHeight)
        assertEquals(1_700_000_000L, state.syncTimestamp)
    }

    @Test
    fun failedResultReducesToError() {
        service.reduce(
            WalletSyncEvent.PlatformAddressResult(
                walletId = walletId,
                success = false,
                foundCount = 0,
                absentCount = 0,
                checkpointHeight = 0,
                newSyncHeight = 0,
                newSyncTimestamp = 0,
                lastKnownRecentBlock = 0,
                errorMessage = "boom",
            ),
        )
        val state = service.state.value
        assertTrue(state is PlatformBalanceSyncService.PlatformSyncState.Error)
        assertEquals("boom", (state as PlatformBalanceSyncService.PlatformSyncState.Error).message)
    }

    @Test
    fun errorEventReducesToError() {
        service.reduce(WalletSyncEvent.Error("fatal"))
        val state = service.state.value
        assertTrue(state is PlatformBalanceSyncService.PlatformSyncState.Error)
        assertEquals("fatal", (state as PlatformBalanceSyncService.PlatformSyncState.Error).message)
    }

    @Test
    fun eventSequenceProducesFinalSyncedState() {
        // A failure then a later success settles Synced (latest wins).
        service.reduce(
            WalletSyncEvent.PlatformAddressResult(
                walletId, false, 0, 0, 0, 0, 0, 0, "transient",
            ),
        )
        assertTrue(service.state.value is PlatformBalanceSyncService.PlatformSyncState.Error)
        service.reduce(
            WalletSyncEvent.PlatformAddressResult(
                walletId, true, 3, 0, 50, 500, 1_699_000_000, 40, null,
            ),
        )
        val state = service.state.value
        assertTrue(state is PlatformBalanceSyncService.PlatformSyncState.Synced)
        assertEquals(500L, (state as PlatformBalanceSyncService.PlatformSyncState.Synced).syncHeight)
    }

    @Test
    fun watermarkRowIsReadOnConfigureSnapshot() = runTest {
        // Persist a watermark under the network-scoped pseudo id (testnet=1).
        val scopeId = syncStateScopeId(1)
        db.platformAddressDao().upsertSyncState(
            PlatformAddressesSyncStateEntity(
                walletId = scopeId,
                networkRaw = 1,
                syncHeight = 9_999,
                syncTimestamp = 1_650_000_000,
                lastKnownRecentBlock = 9_000,
            ),
        )
        val row = db.platformAddressDao().getSyncState(scopeId)
        assertEquals(9_999L, row!!.syncHeight)
        assertEquals(1_650_000_000L, row.syncTimestamp)
    }

    @Test
    fun totalBalanceAndActiveCountReflectRoom() = runTest {
        assertEquals(0L, service.totalPlatformBalance.first())
        assertEquals(0, service.activeAddressCount.first())

        db.platformAddressDao().upsert(address(1, 1_000))
        db.platformAddressDao().upsert(address(2, 2_500))
        db.platformAddressDao().upsert(address(3, 0)) // zero balance — excluded

        assertEquals(3_500L, service.totalPlatformBalance.first())
        assertEquals(2, service.activeAddressCount.first())
    }
}
