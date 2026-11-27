package org.dashfoundation.dashsdk.services

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlinx.coroutines.async
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.PlaintextIdentityKeysKeystore
import org.dashfoundation.dashsdk.security.VotingKeyAssignment
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/** Clearing identities removes the masternode voting keys imported for them. */
@RunWith(RobolectricTestRunner::class)
class DataManagerTest {

    private lateinit var db: DashDatabase
    private lateinit var storage: WalletStorage
    private lateinit var keystore: PlaintextIdentityKeysKeystore
    private lateinit var manager: DataManager

    private val identityId = ByteArray(32) { 5 }
    private val votingIdentifier = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { 7 })
    private val walletKeyHex = "02" + "ab".repeat(32)

    @Before
    fun setUp() = runBlocking {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        keystore = PlaintextIdentityKeysKeystore()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), keystore)
        storage.deleteAll()
        manager = DataManager(db, storage)
        // The pending set is process-wide; every test starts as a fresh process would.
        MasternodeVotingKeyStore.resetPendingForTesting()
        db.identityDao().upsert(
            IdentityEntity(
                identityId = identityId,
                identityType = "Masternode",
                votingPrivateKeyIdentifier = votingIdentifier,
                networkRaw = 1,
            ),
        )
        storage.storePrivateKey(votingIdentifier, ByteArray(32) { 1 })
        storage.storePrivateKey(walletKeyHex, ByteArray(32) { 2 }, ownerWalletId = ByteArray(32) { 3 })
    }

    @After
    fun tearDown() {
        MasternodeVotingKeyStore.onLockContendedForTesting = null
        db.close()
    }

    @Test
    fun shouldDeleteImportedVotingKeysWhenClearingIdentities() = runBlocking {
        manager.clear(DataManager.Category.IDENTITIES)

        assertNull(db.identityDao().getByIdentityId(identityId))
        assertFalse(storage.hasPrivateKey(votingIdentifier))
        assertTrue(storage.hasPrivateKey(walletKeyHex))
    }

    @Test
    fun shouldDeleteImportedVotingKeysWhenClearingEverything() = runBlocking {
        manager.clearAll()

        assertNull(db.identityDao().getByIdentityId(identityId))
        assertFalse(storage.hasPrivateKey(votingIdentifier))
        assertTrue(storage.hasPrivateKey(walletKeyHex))
    }

    /** The keys go first: a clear interrupted after them leaves rows to clear again. */
    @Test
    fun shouldDeleteTheVotingKeysBeforeTheRows() = runBlocking {
        var rowsWhenKeysWereDeleted = -1
        val ordered = DataManager(db) {
            rowsWhenKeysWereDeleted = if (db.identityDao().getByIdentityId(identityId) != null) 1 else 0
        }

        ordered.clear(DataManager.Category.IDENTITIES)

        assertEquals(1, rowsWhenKeysWereDeleted)
        assertNull(db.identityDao().getByIdentityId(identityId))
    }

    @Test
    fun shouldLeaveRowsAndKeysToRetryWhenTheKeyStepFails() = runBlocking {
        val failing = DataManager(db) { throw IllegalStateException("secure storage unavailable") }

        val error = runCatching { failing.clear(DataManager.Category.IDENTITIES) }.exceptionOrNull()

        assertTrue(error is IllegalStateException)
        assertEquals(votingIdentifier, db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertTrue(storage.hasPrivateKey(votingIdentifier))

        // The retry finishes the job.
        manager.clear(DataManager.Category.IDENTITIES)
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertFalse(storage.hasPrivateKey(votingIdentifier))
    }

    /**
     * An import ([MasternodeVotingKeyStore.assignToIdentity], the public operation) that
     * holds the voting key lock when a clear starts: the clear waits on the lock, then
     * deletes both the key the import recorded and its row.
     */
    @Test
    fun shouldClearAKeyAnImportRecordsWhileTheClearWaits() = runBlocking<Unit> {
        val importedId = ByteArray(32) { 6 }
        db.identityDao().upsert(IdentityEntity(identityId = importedId, identityType = "Masternode", networkRaw = 1))
        val importing = CountDownLatch(1)
        val release = CountDownLatch(1)
        // Pauses the import inside its critical section, where it forms the address.
        val keys = MasternodeVotingKeyStore.forTesting(storage) { key ->
            importing.countDown()
            // Bounded, so a broken test cannot park this thread for ever.
            release.await(10, TimeUnit.SECONDS)
            key.copyOf(20)
        }
        val clearWaiting = CompletableDeferred<Unit>()

        try {
            val import = async(Dispatchers.IO) {
                keys.assignToIdentity(db, importedId, 1, ByteArray(32) { 1 }, null)
            }
            assertTrue("the import never started", importing.await(10, TimeUnit.SECONDS))
            MasternodeVotingKeyStore.onLockContendedForTesting = { clearWaiting.complete(Unit) }
            val clear = launch(Dispatchers.IO) { manager.clear(DataManager.Category.IDENTITIES) }
            // Bounded, so a lock that never contends fails the test instead of hanging it.
            withTimeout(10_000) { clearWaiting.await() }
            // The clear reached the lock and is waiting: nothing is cleared yet.
            assertTrue(storage.hasPrivateKey(votingIdentifier))
            assertEquals(votingIdentifier, db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)

            release.countDown()
            val assignment = withTimeout(10_000) { import.await() } as VotingKeyAssignment.Assigned
            withTimeout(10_000) { clear.join() }
            assertNull(db.identityDao().getByIdentityId(importedId))
            assertFalse(storage.hasPrivateKey(assignment.identifier))
            assertNull(db.identityDao().getByIdentityId(identityId))
            assertFalse(storage.hasPrivateKey(votingIdentifier))
        } finally {
            // Never leave the paused operation waiting, even when an assertion failed.
            release.countDown()
        }
    }

    /**
     * A clear that holds the voting key lock when an import starts: the import waits, then
     * finds its row gone and writes nothing to secure storage at all.
     */
    @Test
    fun shouldLeaveNoKeyWhenAnImportWaitsForAClear() = runBlocking<Unit> {
        val importedId = ByteArray(32) { 6 }
        db.identityDao().upsert(IdentityEntity(identityId = importedId, identityType = "Masternode", networkRaw = 1))
        val clearing = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val pausedClear = DataManager(db) {
            clearing.complete(Unit)
            withTimeout(10_000) { release.await() }
            MasternodeVotingKeyStore(storage).deleteAll()
        }
        val keys = MasternodeVotingKeyStore.forTesting(storage) { key -> key.copyOf(20) }
        val importWaiting = CompletableDeferred<Unit>()
        val writesBefore = keystore.writes

        try {
            val clear = launch(Dispatchers.IO) { pausedClear.clear(DataManager.Category.IDENTITIES) }
            withTimeout(10_000) { clearing.await() }
            MasternodeVotingKeyStore.onLockContendedForTesting = { importWaiting.complete(Unit) }
            val import = async(Dispatchers.IO) {
                keys.assignToIdentity(db, importedId, 1, ByteArray(32) { 1 }, null)
            }
            // Bounded, so a lock that never contends fails the test instead of hanging it.
            withTimeout(10_000) { importWaiting.await() }
            val importedIdentifier = MasternodeVotingKeyStore.identifierFor(importedId, ByteArray(20) { 1 })
            // The import reached the lock and is waiting: it has written nothing.
            assertFalse(storage.hasPrivateKey(importedIdentifier))

            release.complete(Unit)
            withTimeout(10_000) { clear.join() }
            assertEquals(VotingKeyAssignment.Changed(null), withTimeout(10_000) { import.await() })
            assertNull(db.identityDao().getByIdentityId(importedId))
            assertFalse(storage.hasPrivateKey(importedIdentifier))
            assertFalse(storage.hasPrivateKey(votingIdentifier))
            assertEquals("the import wrote to secure storage", writesBefore, keystore.writes)
        } finally {
            // Never leave the paused operation waiting, even when an assertion failed.
            release.complete(Unit)
        }
    }

    @Test
    fun shouldKeepVotingKeysWhenClearingAnotherCategory() = runBlocking {
        manager.clear(DataManager.Category.TOKENS)

        assertTrue(storage.hasPrivateKey(votingIdentifier))
        assertEquals(votingIdentifier, db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
    }
}
