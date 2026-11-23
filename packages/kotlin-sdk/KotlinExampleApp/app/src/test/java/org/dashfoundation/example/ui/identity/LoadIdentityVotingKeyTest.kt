package org.dashfoundation.example.ui.identity

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.VotingKeyAssignment
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.RuntimeEnvironment

/**
 * The voting key decisions and the Room writes of [LoadIdentityScreen] (port of
 * `LoadIdentityView.swift`), against an in-memory database and recorded key entries.
 */
@RunWith(RobolectricTestRunner::class)
class LoadIdentityVotingKeyTest {

    private val testnet = 1
    private val mainnet = 0
    private val identityId = ByteArray(32) { 4 }
    private val keyA = ByteArray(32) { 0x0a }
    private val keyB = ByteArray(32) { 0x0b }

    private lateinit var db: DashDatabase
    private lateinit var dao: IdentityDao
    private lateinit var entries: RecordedEntries

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(RuntimeEnvironment.getApplication())
        dao = db.identityDao()
        entries = RecordedEntries()
    }

    @After
    fun tearDown() {
        db.close()
    }

    // ── The store/keep/confirm rule ─────────────────────────────────────

    private val stored = "ab".repeat(20)
    private val entered = "cd".repeat(20)

    @Test
    fun shouldStoreTheEnteredKeyWhenTheRowNamesNone() {
        assertEquals(VotingKeyOutcome.StoreNew, decideVotingKeyOutcome(null, entered))
    }

    @Test
    fun shouldTreatTheKeyTheRowAlreadyNamesAsStored() {
        assertEquals(VotingKeyOutcome.AlreadyStored, decideVotingKeyOutcome(stored, stored))
        assertEquals(VotingKeyOutcome.AlreadyStored, decideVotingKeyOutcome(stored.uppercase(), stored))
    }

    @Test
    fun shouldKeepADifferentStoredKeyAndNameItInTheConflict() {
        assertEquals(VotingKeyOutcome.Conflicting(stored), decideVotingKeyOutcome(stored, entered))
    }

    @Test
    fun shouldQuoteRustsReasonWithoutDoublingItsPeriod() {
        assertEquals(
            "The voting private key was refused: not a WIF key.",
            votingKeyRefusedMessage("not a WIF key."),
        )
        assertEquals("The voting private key was refused.", votingKeyRefusedMessage(null))
        assertEquals("The voting private key was refused.", votingKeyRefusedMessage("  "))
    }

    @Test
    fun shouldOfferMasternodeKeysOnlyForMasternodeAndEvonode() {
        assertFalse(LoadIdentityType.USER.isMasternode)
        assertTrue(LoadIdentityType.MASTERNODE.isMasternode)
        assertTrue(LoadIdentityType.EVONODE.isMasternode)
        assertEquals(listOf("User", "Masternode", "Evonode"), LoadIdentityType.entries.map { it.rawValue })
    }

    // ── The type picker ──────────────────────────────────────────────────

    @Test
    fun shouldInferTheStoredTypeOnThisNetworkAndUserOtherwise() {
        val evonode = row(type = "Evonode")
        assertEquals(LoadIdentityType.EVONODE, inferredLoadIdentityType(evonode, testnet))
        assertEquals(LoadIdentityType.USER, inferredLoadIdentityType(evonode, mainnet))
        assertEquals(LoadIdentityType.USER, inferredLoadIdentityType(null, testnet))
        assertEquals(LoadIdentityType.USER, inferredLoadIdentityType(row(type = "Something else"), testnet))
    }

    @Test
    fun shouldWriteTheStoredTypeOrUserUnlessTheUserPickedOne() {
        assertEquals("Evonode", identityTypeToSave("Evonode", LoadIdentityType.USER, typePicked = false))
        assertEquals("User", identityTypeToSave("Evonode", LoadIdentityType.USER, typePicked = true))
        assertEquals("User", identityTypeToSave(null, LoadIdentityType.EVONODE, typePicked = false))
        assertEquals("Evonode", identityTypeToSave(null, LoadIdentityType.EVONODE, typePicked = true))
    }

    /** S7: a known evonode's id, then an unseen id in the same form, saves the new one as User. */
    @Test
    fun shouldNotCarryAKnownIdentitysTypeIntoANewRow() = runBlocking {
        dao.upsert(row(type = "Evonode"))
        // The picker followed the known id...
        val inferredForKnown = inferredLoadIdentityType(dao.getByIdentityId(identityId), testnet)
        assertEquals(LoadIdentityType.EVONODE, inferredForKnown)
        val unseenId = ByteArray(32) { 6 }

        // ...and a save of an unseen id with that stale, unpicked type still writes User.
        val stale = saveLoadedIdentity(
            dao, entries, unseenId, testnet, 1L, null,
            pickedType = inferredForKnown, typePicked = false, votingKey = null,
        )
        assertEquals(LoadSaveResult.Saved, stale)
        assertEquals("User", dao.getByIdentityId(unseenId)!!.identityType)
        // The picker itself returns to User for the unseen id.
        assertEquals(LoadIdentityType.USER, inferredLoadIdentityType(dao.getByIdentityId(ByteArray(32) { 7 }), testnet))
    }

    // ── Saving a load ────────────────────────────────────────────────────

    /** F6: the default form (type User, no key) with only an alias keeps a voter a voter. */
    @Test
    fun shouldKeepAnEvonodesTypeAndVotingKeyOnAnAliasOnlyReload() = runBlocking {
        val identifier = identifierOf(keyA)
        dao.upsert(row(type = "Evonode", votingIdentifier = identifier, dpnsName = "node"))

        val result = save(alias = "renamed", pickedType = LoadIdentityType.USER, typePicked = false)

        assertEquals(LoadSaveResult.Saved, result)
        val saved = dao.getByIdentityId(identityId)!!
        assertEquals("Evonode", saved.identityType)
        assertEquals(identifier, saved.votingPrivateKeyIdentifier)
        assertEquals("renamed", saved.alias)
        assertEquals("node", saved.dpnsName)
        assertEquals(emptyList<String>(), entries.storedIdentifiers)
    }

    @Test
    fun shouldInsertANewIdentityWithThePickedType() = runBlocking {
        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true)

        assertEquals(LoadSaveResult.Saved, result)
        val saved = dao.getByIdentityId(identityId)!!
        assertEquals("Masternode", saved.identityType)
        assertEquals(testnet, saved.networkRaw)
        assertTrue(saved.isLocal)
        assertEquals(42L, saved.balance)
    }

    /** Only the loaded fields change: columns another writer set are kept. */
    @Test
    fun shouldLeaveFieldsTheLoadDoesNotOwnOnTheCurrentRow() = runBlocking {
        dao.upsert(
            row(type = "Masternode", dpnsName = "kept").copy(
                alias = "old",
                ownerPrivateKeyIdentifier = "owner",
                mainDpnsName = "main",
            ),
        )

        save(alias = null, pickedType = LoadIdentityType.MASTERNODE, typePicked = true)

        val saved = dao.getByIdentityId(identityId)!!
        assertEquals("old", saved.alias)
        assertEquals("owner", saved.ownerPrivateKeyIdentifier)
        assertEquals("kept", saved.dpnsName)
        assertEquals("main", saved.mainDpnsName)
        assertEquals(42L, saved.balance)
    }

    /** B4: an unlinked row takes the fetched balance. */
    @Test
    fun shouldTakeTheFetchedBalanceOnARowNoWalletOwns() = runBlocking {
        dao.upsert(row(type = "Masternode").copy(balance = 7L))

        save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true)

        assertEquals(42L, dao.getByIdentityId(identityId)!!.balance)
    }

    /** B4: a wallet's identity keeps the balance wallet sync wrote, newer than the fetch. */
    @Test
    fun shouldKeepTheSyncedBalanceOfAWalletsIdentity() = runBlocking {
        val walletId = ByteArray(32) { 3 }
        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        dao.upsert(row(type = "User").copy(balance = 1_000L, walletId = walletId))

        val result = save(alias = "mine", pickedType = LoadIdentityType.USER, typePicked = false)

        assertEquals(LoadSaveResult.Saved, result)
        val saved = dao.getByIdentityId(identityId)!!
        assertEquals(1_000L, saved.balance)
        assertEquals("mine", saved.alias)
    }

    @Test
    fun shouldRefuseAnIdentityStoredForAnotherNetworkAndWriteNothing() = runBlocking {
        val identifier = identifierOf(keyA)
        dao.upsert(row(type = "Masternode", votingIdentifier = identifier, network = mainnet))

        val result = save(
            alias = "x",
            pickedType = LoadIdentityType.USER,
            typePicked = true,
            votingKey = keyB,
        )

        assertEquals(
            LoadSaveResult.Failed(
                "This identity is already stored for Mainnet. Switch to that network to reload it.",
            ),
            result,
        )
        val unchanged = dao.getByIdentityId(identityId)!!
        assertEquals(mainnet, unchanged.networkRaw)
        assertEquals("Masternode", unchanged.identityType)
        assertNull(unchanged.alias)
        assertEquals(identifier, unchanged.votingPrivateKeyIdentifier)
        assertEquals(emptyList<String>(), entries.storedIdentifiers)
    }

    @Test
    fun shouldStoreANewKeyAndPointTheRowAtIt() = runBlocking {
        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, votingKey = keyA)

        assertEquals(LoadSaveResult.Saved, result)
        assertEquals(listOf(identifierOf(keyA)), entries.storedIdentifiers)
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
    }

    /** F5: the key the row names is written again from the entered bytes. */
    @Test
    fun shouldRewriteTheEntryWhenTheSameKeyIsEnteredAgain() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))

        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, votingKey = keyA)

        assertEquals(LoadSaveResult.Saved, result)
        assertEquals(listOf(identifierOf(keyA)), entries.storedIdentifiers)
    }

    @Test
    fun shouldShowAStorageFailureInsteadOfReportingSuccess() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.failure = IllegalStateException("keystore unavailable")

        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, votingKey = keyA)

        assertEquals(LoadSaveResult.Failed(votingKeyNotStoredMessage("keystore unavailable")), result)
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
    }

    @Test
    fun shouldPropagateCancellationOfTheKeyWrite() {
        entries.failure = CancellationException("cancelled")
        assertThrows(CancellationException::class.java) {
            runBlocking {
                save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, votingKey = keyA)
            }
        }
    }

    /** F4: a key association written while the new key was stored is kept. */
    @Test
    fun shouldKeepAnAssociationWrittenWhileStoringANewKey() = runBlocking {
        val meanwhile = identifierOf(keyB)
        entries.onStore = {
            dao.replaceVotingPrivateKeyIdentifier(identityId, null, meanwhile, 0)
        }

        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, votingKey = keyA)

        assertEquals(LoadSaveResult.Failed(IDENTITY_CHANGED_MESSAGE), result)
        assertEquals(meanwhile, dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(listOf(identifierOf(keyA)), entries.deletedIdentifiers)
    }

    @Test
    fun shouldAskBeforeReplacingADifferentStoredKeyAndChangeNothingYet() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))

        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, votingKey = keyB)

        assertEquals(LoadSaveResult.ConfirmReplacement(identifierOf(keyA)), result)
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(emptyList<String>(), entries.storedIdentifiers)
    }

    @Test
    fun shouldReplaceTheKeyTheQuestionShowedAndDeleteItsEntry() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += identifierOf(keyA)

        val result = replaceVotingKey(dao, entries, identityId, identifierOf(keyA), keyB)

        assertEquals(LoadSaveResult.Saved, result)
        assertEquals(identifierOf(keyB), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(setOf(identifierOf(keyB)), entries.live)
    }

    /** S6: the replacement stands, the leftover is reported, and the next load sweeps it. */
    @Test
    fun shouldReportAReplacedKeyThatCouldNotBeRemovedAndSweepItOnTheNextLoad() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += identifierOf(keyA)
        entries.sweepFailure = IllegalStateException("secure storage unavailable")

        val result = replaceVotingKey(dao, entries, identityId, identifierOf(keyA), keyB)

        assertEquals(LoadSaveResult.SavedWithWarning(PREVIOUS_VOTING_KEY_NOT_REMOVED_MESSAGE), result)
        assertEquals(identifierOf(keyB), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(setOf(identifierOf(keyA), identifierOf(keyB)), entries.live)

        entries.sweepFailure = null
        assertEquals(
            LoadSaveResult.Saved,
            save(alias = "again", pickedType = LoadIdentityType.USER, typePicked = false),
        )
        assertEquals(setOf(identifierOf(keyB)), entries.live)
    }

    /** F4: the row changed after the question was asked, so nothing is replaced. */
    @Test
    fun shouldKeepTheCurrentKeyWhenTheRowChangedAfterTheQuestion() = runBlocking {
        val shown = identifierOf(keyA)
        val current = identifierOf(ByteArray(32) { 0x0c })
        dao.upsert(row(type = "Masternode", votingIdentifier = current))

        val result = replaceVotingKey(dao, entries, identityId, shown, keyB)

        assertEquals(LoadSaveResult.Failed(VOTING_KEY_CHANGED_MESSAGE), result)
        assertEquals(current, dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(listOf(identifierOf(keyB)), entries.deletedIdentifiers)
    }

    /** S2: every reload warns while an earlier entry cannot be removed, then sweeps it. */
    @Test
    fun shouldWarnOnEachReloadUntilAStaleKeyIsRemoved() = runBlocking<Unit> {
        val stale = identifierOf(ByteArray(32) { 0x0d })
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += setOf(identifierOf(keyA), stale)
        entries.sweepFailure = IllegalStateException("secure storage unavailable")

        repeat(2) {
            assertEquals(
                LoadSaveResult.SavedWithWarning(STALE_VOTING_KEY_NOT_REMOVED_MESSAGE),
                save(alias = "again", pickedType = LoadIdentityType.USER, typePicked = false),
            )
            assertEquals(setOf(identifierOf(keyA), stale), entries.live)
        }

        entries.sweepFailure = null
        assertEquals(
            LoadSaveResult.Saved,
            save(alias = "again", pickedType = LoadIdentityType.USER, typePicked = false),
        )
        assertEquals(setOf(identifierOf(keyA)), entries.live)
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
    }

    /**
     * B1: a reload started while a confirmed replacement has stored its new key but not yet
     * recorded it waits for the replacement, so its sweep cannot delete the new key.
     */
    @Test
    fun shouldRunAReloadAfterAReplacementThatIsStoringItsKey() = runBlocking<Unit> {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += identifierOf(keyA)
        val storing = CompletableDeferred<Unit>()
        val release = CompletableDeferred<Unit>()
        entries.onStore = {
            storing.complete(Unit)
            release.await()
        }

        try {
            val replacement = async(Dispatchers.IO) {
                replaceVotingKey(dao, entries, identityId, identifierOf(keyA), keyB)
            }
            storing.await()
            val reloadWaiting = CompletableDeferred<Unit>()
            entries.onLockRequested = { reloadWaiting.complete(Unit) }
            val reload = async(Dispatchers.IO) {
                save(alias = "again", pickedType = LoadIdentityType.USER, typePicked = false)
            }
            // The reload asks for the voting key lock the replacement holds, and has swept nothing.
            withTimeout(10_000) { reloadWaiting.await() }
            assertTrue(identifierOf(keyB) in entries.live)

            release.complete(Unit)
            assertEquals(LoadSaveResult.Saved, replacement.await())
            assertEquals(LoadSaveResult.Saved, reload.await())
            assertEquals(identifierOf(keyB), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
            assertEquals(setOf(identifierOf(keyB)), entries.live)
        } finally {
            // Never leave the paused operation waiting, even when an assertion failed.
            release.complete(Unit)
        }
    }

    /** S3: a cleanup lookup that throws leaves the entries alone and only warns. */
    @Test
    fun shouldWarnAndDeleteNothingWhenTheCleanupLookupFails() = runBlocking<Unit> {
        val stale = identifierOf(ByteArray(32) { 0x0d })
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += setOf(identifierOf(keyA), stale)
        // The save's own read is the first lookup; the cleanup's is the second.
        val failingDao = ThrowingLookupDao(dao, failOnCall = 2)

        val result = saveLoadedIdentity(
            dao = failingDao,
            entries = entries,
            identityId = identityId,
            networkRaw = testnet,
            balance = 42L,
            alias = "again",
            pickedType = LoadIdentityType.USER,
            typePicked = false,
            votingKey = null,
        )

        assertEquals(LoadSaveResult.SavedWithWarning(STALE_VOTING_KEY_NOT_REMOVED_MESSAGE), result)
        assertEquals(setOf(identifierOf(keyA), stale), entries.live)
        assertEquals("again", dao.getByIdentityId(identityId)!!.alias)
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    /** An [IdentityDao] whose [failOnCall]-th `getByIdentityId` throws. */
    private class ThrowingLookupDao(
        private val real: IdentityDao,
        private val failOnCall: Int,
    ) : IdentityDao by real {
        private var calls = 0

        override suspend fun getByIdentityId(identityId: ByteArray): IdentityEntity? {
            calls += 1
            if (calls == failOnCall) throw IllegalStateException("database unavailable")
            return real.getByIdentityId(identityId)
        }
    }

    private suspend fun save(
        alias: String? = null,
        pickedType: LoadIdentityType,
        typePicked: Boolean,
        votingKey: ByteArray? = null,
    ): LoadSaveResult = saveLoadedIdentity(
        dao = dao,
        entries = entries,
        identityId = identityId,
        networkRaw = testnet,
        balance = 42L,
        alias = alias,
        pickedType = pickedType,
        typePicked = typePicked,
        votingKey = votingKey,
    )

    private fun row(
        type: String,
        votingIdentifier: String? = null,
        network: Int = testnet,
        dpnsName: String? = null,
    ) = IdentityEntity(
        identityId = identityId,
        identityType = type,
        votingPrivateKeyIdentifier = votingIdentifier,
        networkRaw = network,
        dpnsName = dpnsName,
    )

    /** A stand-in voting address: the key's first 20 bytes. */
    private fun identifierOf(key: ByteArray): String =
        MasternodeVotingKeyStore.identifierFor(identityId, key.copyOf(20))

    /**
     * Records the entries written and deleted and keeps the set of [live] ones; no native
     * library, no Keystore.
     */
    private inner class RecordedEntries : VotingKeyEntries {
        val storedIdentifiers = mutableListOf<String>()
        val deletedIdentifiers = mutableListOf<String>()
        val live = mutableSetOf<String>()
        var failure: Exception? = null
        var sweepFailure: Exception? = null
        var onStore: (suspend () -> Unit)? = null

        /** Called just before asking for the voting key lock. */
        var onLockRequested: (() -> Unit)? = null

        override fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String =
            MasternodeVotingKeyStore.identifierFor(identityId, votingKey.copyOf(20))

        /**
         * What `MasternodeVotingKeyStore.assignToIdentity` does, over [live] instead of
         * secure storage, under the real voting key lock. The SDK's own tests cover the real
         * operation.
         */
        override suspend fun assign(
            dao: IdentityDao,
            identityId: ByteArray,
            votingKey: ByteArray,
            expectedIdentifier: String?,
            nowMillis: Long,
        ): VotingKeyAssignment = withLock {
            failure?.let { throw it }
            val identifier = identifierFor(identityId, votingKey)
            storedIdentifiers += identifier
            live += identifier
            onStore?.invoke()
            if (dao.replaceVotingPrivateKeyIdentifier(identityId, expectedIdentifier, identifier, nowMillis) == 0) {
                val current = dao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier
                if (current != identifier) {
                    deletedIdentifiers += identifier
                    live -= identifier
                }
                VotingKeyAssignment.Changed(current)
            } else {
                val swept = try {
                    deleteOtherEntriesOf(identityId, identifier)
                    true
                } catch (e: CancellationException) {
                    throw e
                } catch (_: Exception) {
                    false
                }
                VotingKeyAssignment.Assigned(identifier, otherEntriesRemoved = swept)
            }
        }

        override suspend fun <T> withLock(block: suspend () -> T): T {
            onLockRequested?.invoke()
            return MasternodeVotingKeyStore.withVotingKeyLock(block)
        }

        override suspend fun deleteOtherEntriesOf(identityId: ByteArray, keep: String?) {
            sweepFailure?.let { throw it }
            val prefix = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20))
                .substringBeforeLast('.')
            live.removeAll { it.startsWith("$prefix.") && it != keep }
        }
    }
}
