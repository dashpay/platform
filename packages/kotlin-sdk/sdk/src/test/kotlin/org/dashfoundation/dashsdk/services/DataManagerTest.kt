package org.dashfoundation.dashsdk.services

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.joinAll
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeoutOrNull
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.PlaintextIdentityKeysKeystore
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
    private lateinit var manager: DataManager

    private val identityId = ByteArray(32) { 5 }
    private val votingIdentifier = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { 7 })
    private val walletKeyHex = "02" + "ab".repeat(32)

    @Before
    fun setUp() = runBlocking {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), PlaintextIdentityKeysKeystore())
        storage.deleteAll()
        manager = DataManager(db, storage)
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
     * B1: a clear started while an import has stored its key but not yet recorded it waits
     * for the import, then deletes both the key and the row.
     */
    @Test
    fun shouldNotMissAKeyAnImportStoresDuringAClear() = runBlocking<Unit> {
        val importedId = ByteArray(32) { 6 }
        val importedIdentifier = MasternodeVotingKeyStore.identifierFor(importedId, ByteArray(20) { 8 })
        db.identityDao().upsert(IdentityEntity(identityId = importedId, identityType = "Masternode", networkRaw = 1))
        val storing = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        val import = launch(Dispatchers.IO) {
            MasternodeVotingKeyStore.withVotingKeyLock {
                storage.storePrivateKey(importedIdentifier, ByteArray(32) { 1 })
                storing.complete(Unit)
                release.await()
                db.identityDao().replaceVotingPrivateKeyIdentifier(importedId, null, importedIdentifier, 0)
            }
        }
        storing.await()

        val clear = launch(Dispatchers.IO) { manager.clear(DataManager.Category.IDENTITIES) }
        // The clear waits on the voting key lock while the import holds it.
        assertNull(withTimeoutOrNull(300) { clear.join() })
        assertTrue(storage.hasPrivateKey(votingIdentifier))

        release.complete(Unit)
        joinAll(import, clear)
        assertNull(db.identityDao().getByIdentityId(importedId))
        assertFalse(storage.hasPrivateKey(importedIdentifier))
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertFalse(storage.hasPrivateKey(votingIdentifier))
    }

    @Test
    fun shouldKeepVotingKeysWhenClearingAnotherCategory() = runBlocking {
        manager.clear(DataManager.Category.TOKENS)

        assertTrue(storage.hasPrivateKey(votingIdentifier))
        assertEquals(votingIdentifier, db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
    }
}
