package org.dashfoundation.dashsdk.security

import androidx.test.core.app.ApplicationProvider
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.withTransaction
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.coroutines.withTimeoutOrNull
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * The parts of [MasternodeVotingKeyStore] that need no native library: the identifier format
 * and reading it back, the lookups of a key that was never stored, and deleting every voting
 * entry. Forming the address and storing a key go through `libdash_sdk_jni` and are covered
 * by `MasternodeVotingKeyFfiTest` and `MasternodeVotingKeySigningTest` (androidTest).
 */
@RunWith(RobolectricTestRunner::class)
class MasternodeVotingKeyStoreTest {

    private lateinit var storage: WalletStorage
    private lateinit var store: MasternodeVotingKeyStore

    /** hash160 of the compressed public key of the secret 1. */
    private val addressHex = "751e76e8199196d454941c45d1b3a323f1433bd6"
    private val address = hex(addressHex)
    private val identityId = ByteArray(32) { (it + 1).toByte() }
    private val identityIdHex = identityId.joinToString("") { "%02x".format(it) }
    private val identifier = "mnvoting.$identityIdHex.$addressHex"

    private lateinit var db: DashDatabase

    /** The network every fixture row is on (`Network.TESTNET.ffiValue`). */
    private val testnet = 1
    private lateinit var keystore: PlaintextIdentityKeysKeystore

    /** A store whose voting address of a key is its first 20 bytes (no native library). */
    private lateinit var assigning: MasternodeVotingKeyStore

    @Before
    fun setUp() = runBlocking {
        keystore = PlaintextIdentityKeysKeystore()
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), keystore)
        storage.deleteAll()
        store = MasternodeVotingKeyStore(storage)
        assigning = MasternodeVotingKeyStore.forTesting(storage) { key -> key.copyOf(20) }
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        // The pending set is process-wide; every test starts as a fresh process would.
        MasternodeVotingKeyStore.resetPendingForTesting()
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun keyWithAddress(byte: Int) = ByteArray(32) { byte.toByte() }

    private fun identifierOfKey(byte: Int) =
        MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { byte.toByte() })

    private suspend fun insertRow(votingIdentifier: String?) {
        db.identityDao().upsert(
            IdentityEntity(
                identityId = identityId,
                identityType = "Masternode",
                votingPrivateKeyIdentifier = votingIdentifier,
                networkRaw = 1,
            ),
        )
    }

    /** Assignment records the new key and deletes nothing; the cleanup then leaves only it. */
    @Test
    fun shouldRecordTheNewKeyAndLeaveOldEntriesToTheCleanup() = runBlocking<Unit> {
        insertRow(votingIdentifier = identifierOfKey(1))
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        storage.storePrivateKey(identifierOfKey(3), keyWithAddress(3))

        val assignment = assigning.assignToIdentity(
            db, identityId, testnet, keyWithAddress(2), expectedIdentifier = identifierOfKey(1),
        )

        assertEquals(VotingKeyAssignment.Assigned(identifierOfKey(2)), assignment)
        assertEquals(identifierOfKey(2), db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertTrue(storage.hasPrivateKey(identifierOfKey(2)))
        assertTrue(storage.hasPrivateKey(identifierOfKey(3)))

        assertTrue(assigning.removeStaleEntries(db, identityId))
        assertTrue(storage.hasPrivateKey(identifierOfKey(2)))
        assertFalse(storage.hasPrivateKey(identifierOfKey(1)))
        assertFalse(storage.hasPrivateKey(identifierOfKey(3)))
        assertTrue(db.identityDao().existsByVotingPrivateKeyIdentifier(identifierOfKey(2)))
        assertFalse(db.identityDao().existsByVotingPrivateKeyIdentifier(identifierOfKey(1)))
    }

    /** S2: a row that names another key is seen before anything is written. */
    @Test
    fun shouldWriteNothingWhenTheRowNamesAnother() = runBlocking<Unit> {
        insertRow(votingIdentifier = identifierOfKey(1))
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        val writesBefore = keystore.writes

        val assignment = assigning.assignToIdentity(
            db, identityId, testnet, keyWithAddress(2), expectedIdentifier = null,
        )

        assertEquals(VotingKeyAssignment.Changed(identifierOfKey(1)), assignment)
        assertEquals(identifierOfKey(1), db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertFalse(storage.hasPrivateKey(identifierOfKey(2)))
        assertEquals(writesBefore, keystore.writes)
    }

    /** S2: a row that is gone (a clear ran first) gets no key written back. */
    @Test
    fun shouldWriteNothingWhenTheRowIsGone() = runBlocking<Unit> {
        val assignment = assigning.assignToIdentity(
            db, identityId, testnet, keyWithAddress(2), expectedIdentifier = null,
        )

        assertEquals(VotingKeyAssignment.Changed(null), assignment)
        assertFalse(storage.hasPrivateKey(identifierOfKey(2)))
        assertEquals(0, keystore.writes)
    }

    /** A row changed between the check and the record still keeps its key; the new one goes. */
    @Test
    fun shouldDeleteTheNewKeyWhenTheRowChangesAfterTheWrite() = runBlocking<Unit> {
        insertRow(votingIdentifier = null)
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        // Records key 1 on the row while key 2 is being written, outside the lock.
        val racing = MasternodeVotingKeyStore.forTesting(storage) { key ->
            runBlocking {
                db.identityDao().replaceVotingPrivateKeyIdentifier(identityId, testnet, null, identifierOfKey(1), 0)
            }
            key.copyOf(20)
        }

        val assignment = racing.assignToIdentity(db, identityId, testnet, keyWithAddress(2), expectedIdentifier = null)

        assertEquals(VotingKeyAssignment.Changed(identifierOfKey(1)), assignment)
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertFalse(storage.hasPrivateKey(identifierOfKey(2)))
    }

    /**
     * The rollback the thread check cannot see: a caller hops to an IO worker inside
     * `withTransaction`, so `inTransaction()` is false there while Room still routes the row
     * update into the outer transaction. The rollback restores the row's reference to key 1,
     * and key 1 is still stored because the assignment deletes nothing. Key 2 stays pending,
     * so this process's cleanups keep it; the next process (simulated by resetting the
     * pending set) removes it.
     */
    @Test
    fun shouldKeepTheOldKeyWhenAnEnclosingTransactionRollsBack() = runBlocking<Unit> {
        insertRow(votingIdentifier = identifierOfKey(1))
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        var assignedInside: VotingKeyAssignment? = null

        val rolledBack = runCatching {
            // Bounded, so a transaction that cannot be re-entered fails instead of hanging.
            withTimeout(20_000) {
                db.withTransaction {
                    assignedInside = withContext(Dispatchers.IO) {
                        assigning.assignToIdentity(db, identityId, testnet, keyWithAddress(2), identifierOfKey(1))
                    }
                    throw IllegalStateException("roll back")
                }
            }
        }.exceptionOrNull()

        assertEquals("roll back", rolledBack?.message)
        // The IO hop passed the thread-bound guard and the update joined the transaction.
        assertEquals(VotingKeyAssignment.Assigned(identifierOfKey(2)), assignedInside)
        assertEquals(identifierOfKey(1), db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertTrue(storage.hasPrivateKey(identifierOfKey(2)))

        // Still pending in this process: kept, and the cleanup reports it incomplete.
        assertFalse(assigning.removeStaleEntries(db, identityId))
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertTrue(storage.hasPrivateKey(identifierOfKey(2)))

        // A new process forgets it; its first cleanup removes the leftover.
        MasternodeVotingKeyStore.resetPendingForTesting()
        assertTrue(assigning.removeStaleEntries(db, identityId))
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertFalse(storage.hasPrivateKey(identifierOfKey(2)))
    }

    /**
     * The commit case: an assignment inside a caller's transaction that has not committed
     * yet, and a cleanup from an independent coroutine meanwhile. On a WAL database the
     * cleanup reads the committed row (key 1) while the transaction is open; the new key 2
     * is pending, so it survives, and the commit's row names a key that is still stored. The
     * next cleanup sees the row name key 2, unpends it and removes key 1.
     */
    @Test
    fun shouldNotSweepAnAssignmentWhoseTransactionHasNotCommitted() = runBlocking<Unit> {
        val context = ApplicationProvider.getApplicationContext<android.content.Context>()
        val name = "voting-key-commit-${System.nanoTime()}.db"
        val walDb = Room.databaseBuilder(context, DashDatabase::class.java, name)
            .setJournalMode(RoomDatabase.JournalMode.WRITE_AHEAD_LOGGING)
            .build()
        try {
            walDb.identityDao().upsert(
                IdentityEntity(
                    identityId = identityId,
                    identityType = "Masternode",
                    votingPrivateKeyIdentifier = identifierOfKey(1),
                    networkRaw = testnet,
                ),
            )
            storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
            val assigned = CompletableDeferred<Unit>()
            val cleanedUp = CompletableDeferred<Boolean>()
            var cleanupFinishedInsideTransaction = false

            val writer = async(Dispatchers.Default) {
                walDb.withTransaction {
                    withContext(Dispatchers.IO) {
                        assigning.assignToIdentity(walDb, identityId, testnet, keyWithAddress(2), identifierOfKey(1))
                    }
                    assigned.complete(Unit)
                    // Hold the transaction open while the independent cleanup runs.
                    cleanupFinishedInsideTransaction =
                        withTimeoutOrNull(5_000) { cleanedUp.await() } != null
                }
            }
            withTimeout(10_000) { assigned.await() }
            val cleanup = async(Dispatchers.IO) {
                assigning.removeStaleEntries(walDb, identityId).also { cleanedUp.complete(it) }
            }
            withTimeout(20_000) { writer.await() }
            // B had to stay while pending: the cleanup reports it incomplete.
            assertFalse(withTimeout(10_000) { cleanup.await() })
            // The cleanup ran against the committed row, before the commit.
            assertTrue("the cleanup did not run inside the open transaction", cleanupFinishedInsideTransaction)

            assertEquals(identifierOfKey(2), walDb.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
            assertTrue("the committed row names a key the cleanup deleted", storage.hasPrivateKey(identifierOfKey(2)))
            assertTrue(storage.hasPrivateKey(identifierOfKey(1)))

            // After the commit: the row names key 2, so it is unpended and key 1 goes.
            assertTrue(assigning.removeStaleEntries(walDb, identityId))
            assertTrue(storage.hasPrivateKey(identifierOfKey(2)))
            assertFalse(storage.hasPrivateKey(identifierOfKey(1)))
        } finally {
            walDb.close()
            context.deleteDatabase(name)
        }
    }

    /**
     * S2: A then B assigned and committed before any sweep. B's assignment saw A on the row,
     * so A is no longer pending: the first sweep removes it and reports a complete cleanup.
     */
    @Test
    fun shouldSweepAKeyReplacedBeforeItsFirstCleanup() = runBlocking<Unit> {
        insertRow(votingIdentifier = null)
        assertEquals(
            VotingKeyAssignment.Assigned(identifierOfKey(1)),
            assigning.assignToIdentity(db, identityId, testnet, keyWithAddress(1), null),
        )
        assertEquals(
            VotingKeyAssignment.Assigned(identifierOfKey(2)),
            assigning.assignToIdentity(db, identityId, testnet, keyWithAddress(2), identifierOfKey(1)),
        )

        assertTrue(assigning.removeStaleEntries(db, identityId))
        assertFalse(storage.hasPrivateKey(identifierOfKey(1)))
        assertTrue(storage.hasPrivateKey(identifierOfKey(2)))
    }

    /**
     * S1: a same-key re-entry whose row moves network between the check and the update. The
     * update misses, but the row still names the entry the re-entry rewrote, so it stays.
     */
    @Test
    fun shouldKeepARewrittenEntryTheRowStillNamesAfterANetworkChange() = runBlocking<Unit> {
        insertRow(votingIdentifier = identifierOfKey(1))
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        // Moves the row to another network while key 1 is being rewritten, keeping its key.
        val moving = MasternodeVotingKeyStore.forTesting(storage) { key ->
            runBlocking {
                val row = db.identityDao().getByIdentityId(identityId)!!
                db.identityDao().upsert(row.copy(networkRaw = 0))
            }
            key.copyOf(20)
        }

        val assignment = moving.assignToIdentity(
            db, identityId, testnet, keyWithAddress(1), expectedIdentifier = identifierOfKey(1),
        )

        assertEquals(VotingKeyAssignment.Changed(null), assignment)
        assertEquals(identifierOfKey(1), db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertTrue("the entry the row still names was deleted", storage.hasPrivateKey(identifierOfKey(1)))
    }

    /** A row recreated on another network is not this load's: nothing is written. */
    @Test
    fun shouldWriteNothingForARowOnAnotherNetwork() = runBlocking<Unit> {
        insertRow(votingIdentifier = null)
        val writesBefore = keystore.writes

        val assignment = assigning.assignToIdentity(
            db, identityId, networkRaw = 0, privateKey = keyWithAddress(2), expectedIdentifier = null,
        )

        assertEquals(VotingKeyAssignment.Changed(null), assignment)
        assertEquals(null, db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertFalse(storage.hasPrivateKey(identifierOfKey(2)))
        assertEquals(writesBefore, keystore.writes)
        // The row write itself is scoped to the network too.
        assertEquals(0, db.identityDao().replaceVotingPrivateKeyIdentifier(identityId, 0, null, identifierOfKey(2), 0))
    }

    /** With the row gone, the cleanup removes every entry of the identity, and only those. */
    @Test
    fun shouldRemoveAllOfAGoneIdentitysEntries() = runBlocking<Unit> {
        val otherIdentity = MasternodeVotingKeyStore.identifierFor(ByteArray(32) { 9 }, address)
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        storage.storePrivateKey(otherIdentity, keyWithAddress(1))

        assertTrue(assigning.removeStaleEntries(db, identityId))

        assertFalse(storage.hasPrivateKey(identifierOfKey(1)))
        assertTrue(storage.hasPrivateKey(otherIdentity))
    }

    /** B1: inside a Room transaction it refuses before writing, and a rollback finds A intact. */
    @Test
    fun shouldRefuseToRunInsideARoomTransaction() = runBlocking<Unit> {
        insertRow(votingIdentifier = identifierOfKey(1))
        storage.storePrivateKey(identifierOfKey(1), keyWithAddress(1))
        val writesBefore = keystore.writes

        var refused: Throwable? = null
        db.withTransaction {
            refused = try {
                assigning.assignToIdentity(db, identityId, testnet, keyWithAddress(2), identifierOfKey(1))
                null
            } catch (e: IllegalStateException) {
                e
            }
        }

        assertTrue(refused.toString(), refused is IllegalStateException)
        assertEquals(writesBefore, keystore.writes)
        assertEquals(identifierOfKey(1), db.identityDao().getByIdentityId(identityId)?.votingPrivateKeyIdentifier)
        assertTrue(storage.hasPrivateKey(identifierOfKey(1)))
        assertFalse(storage.hasPrivateKey(identifierOfKey(2)))
    }

    @Test
    fun shouldNameAnEntryByItsIdentityAndVotingAddress() {
        assertEquals(identifier, MasternodeVotingKeyStore.identifierFor(identityId, address))
    }

    @Test
    fun shouldReadAnIdentifierBackAsItsIdentityAndVotingAddress() {
        assertArrayEquals(address, store.votingAddressOf(identifier))
        assertArrayEquals(identityId, store.identityIdOf(identifier))
        assertArrayEquals(address, store.votingAddressOf(identifier.uppercase()))
    }

    @Test
    fun shouldRefuseAnythingThatIsNotAVotingKeyIdentifier() {
        val refused = listOf(
            "",
            // The address alone, the format of an ordinary identity key entry.
            addressHex,
            "mnvoting.$identityIdHex",
            "mnvoting.$identityIdHex.${addressHex.dropLast(2)}",
            "mnvoting.${identityIdHex.drop(2)}.$addressHex",
            "mnvoting.$identityIdHex.$addressHex.00",
            "other.$identityIdHex.$addressHex",
            "mnvoting.$identityIdHex.zz${addressHex.drop(2)}",
        )
        for (text in refused) {
            assertThrows(text, IllegalArgumentException::class.java) { store.votingAddressOf(text) }
            assertThrows(text, IllegalArgumentException::class.java) { store.identityIdOf(text) }
        }
    }

    @Test
    fun shouldRefuseIdsAndAddressesOfTheWrongWidth() {
        assertThrows(IllegalArgumentException::class.java) {
            MasternodeVotingKeyStore.identifierFor(ByteArray(31), address)
        }
        assertThrows(IllegalArgumentException::class.java) {
            MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(21))
        }
    }

    /** A bare address hex is refused, so it can reach neither a wallet entry nor a voting one. */
    @Test
    fun shouldRefuseABareAddressInLookupsAndDeletes() = runBlocking {
        storage.storePrivateKey(identifier, ByteArray(32) { 1 })
        storage.storePrivateKey(addressHex, ByteArray(32) { 2 }, ownerWalletId = ByteArray(32) { 3 })

        assertThrows(IllegalArgumentException::class.java) { runBlocking { store.delete(addressHex) } }
        assertThrows(IllegalArgumentException::class.java) { runBlocking { store.isStored(addressHex) } }

        assertTrue(storage.hasPrivateKey(identifier))
        assertTrue(storage.hasPrivateKey(addressHex))
    }

    /** The sweep deletes this identity's other voting entries only. */
    @Test
    fun shouldDeleteTheIdentitysOtherVotingEntriesAndKeepTheCurrentOne() = runBlocking {
        val stale = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { 8 })
        val otherIdentity = MasternodeVotingKeyStore.identifierFor(ByteArray(32) { 9 }, address)
        for (name in listOf(identifier, stale, otherIdentity)) {
            storage.storePrivateKey(name, ByteArray(32) { 1 })
        }
        storage.storePrivateKey(addressHex, ByteArray(32) { 2 }, ownerWalletId = ByteArray(32) { 3 })

        store.deleteOtherEntriesOf(identityId, keep = identifier)

        assertTrue(storage.hasPrivateKey(identifier))
        assertFalse(storage.hasPrivateKey(stale))
        assertTrue(storage.hasPrivateKey(otherIdentity))
        assertTrue(storage.hasPrivateKey(addressHex))

        store.deleteOtherEntriesOf(identityId, keep = null)
        assertFalse(storage.hasPrivateKey(identifier))
        assertTrue(storage.hasPrivateKey(otherIdentity))
    }

    @Test
    fun shouldRefuseToKeepAnotherIdentitysEntryInTheSweep() {
        val otherIdentity = MasternodeVotingKeyStore.identifierFor(ByteArray(32) { 9 }, address)
        assertThrows(IllegalArgumentException::class.java) {
            runBlocking { store.deleteOtherEntriesOf(identityId, keep = otherIdentity) }
        }
    }

    @Test
    fun shouldReportAKeyNeverStoredAsAbsentAndDeleteItQuietly() = runBlocking {
        assertFalse(store.isStored(identifier))
        store.delete(identifier)
        assertFalse(store.isStored(identifier))
    }

    @Test
    fun shouldRefuseAVotingKeyThatIsNot32BytesBeforeTheNativeCall() {
        assertThrows(IllegalArgumentException::class.java) { store.votingAddress(ByteArray(31)) }
    }

    /** deleteAll removes every voting entry and nothing else, such as a wallet's key. */
    @Test
    fun shouldDeleteEveryVotingEntryAndKeepIdentityKeys() = runBlocking {
        val other = MasternodeVotingKeyStore.identifierFor(ByteArray(32) { 9 }, address)
        storage.storePrivateKey(identifier, ByteArray(32) { 1 })
        storage.storePrivateKey(other, ByteArray(32) { 1 })
        // A wallet identity's ECDSA_HASH160 key is stored under the same address hex.
        storage.storePrivateKey(addressHex, ByteArray(32) { 2 }, ownerWalletId = ByteArray(32) { 3 })

        store.deleteAll()

        assertFalse(storage.hasPrivateKey(identifier))
        assertFalse(storage.hasPrivateKey(other))
        assertTrue(storage.hasPrivateKey(addressHex))
    }

    private fun hex(value: String): ByteArray =
        ByteArray(value.length / 2) { value.substring(it * 2, it * 2 + 2).toInt(16).toByte() }
}
