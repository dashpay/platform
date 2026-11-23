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
import org.dashfoundation.dashsdk.voting.MasternodeRecordedVotingKey
import org.dashfoundation.dashsdk.voting.MasternodeRecordedVotingKeyUse
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyAddress
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheck
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyCheckOutcome
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRecords
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeyRules
import org.dashfoundation.dashsdk.voting.MasternodeVotingKeySlot
import org.dashfoundation.dashsdk.voting.isVotingOnlyIdentity
import org.dashfoundation.dashsdk.voting.MasternodeVotingLoadedIdentity
import org.dashfoundation.dashsdk.voting.recordedVotingKey
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
    private lateinit var rules: AnsweringRules

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(RuntimeEnvironment.getApplication())
        dao = db.identityDao()
        entries = RecordedEntries()
        rules = AnsweringRules()
    }

    @After
    fun tearDown() {
        MasternodeVotingKeyStore.onLockContendedForTesting = null
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
            dao.replaceVotingPrivateKeyIdentifier(identityId, testnet, null, meanwhile, 0)
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

        val result = replaceVotingKey(dao, entries, identityId, testnet, identifierOf(keyA), keyB)

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

        val result = replaceVotingKey(dao, entries, identityId, testnet, identifierOf(keyA), keyB)

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

        val result = replaceVotingKey(dao, entries, identityId, testnet, shown, keyB)

        assertEquals(LoadSaveResult.Failed(VOTING_KEY_CHANGED_MESSAGE), result)
        assertEquals(current, dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        // The change is seen before the new key is written.
        assertEquals(emptyList<String>(), entries.storedIdentifiers)
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
            withTimeout(10_000) { release.await() }
        }

        try {
            val replacement = async(Dispatchers.IO) {
                replaceVotingKey(dao, entries, identityId, testnet, identifierOf(keyA), keyB)
            }
            withTimeout(10_000) { storing.await() }
            val reloadWaiting = CompletableDeferred<Unit>()
            // The SDK's own signal that a caller is waiting on the real voting key mutex.
            MasternodeVotingKeyStore.onLockContendedForTesting = { reloadWaiting.complete(Unit) }
            val reload = async(Dispatchers.IO) {
                save(alias = "again", pickedType = LoadIdentityType.USER, typePicked = false)
            }
            // The reload waits on the lock the replacement holds, and has swept nothing.
            withTimeout(10_000) { reloadWaiting.await() }
            assertTrue(identifierOf(keyB) in entries.live)

            release.complete(Unit)
            assertEquals(LoadSaveResult.Saved, withTimeout(10_000) { replacement.await() })
            assertEquals(LoadSaveResult.Saved, withTimeout(10_000) { reload.await() })
            assertEquals(identifierOf(keyB), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
            assertEquals(setOf(identifierOf(keyB)), entries.live)
        } finally {
            // Never leave the paused operation waiting, even when an assertion failed.
            release.complete(Unit)
        }
    }

    /** A cleanup that throws instead of reporting failure still only warns. */
    @Test
    fun shouldWarnAndKeepTheSaveWhenTheCleanupThrows() = runBlocking<Unit> {
        val stale = identifierOf(ByteArray(32) { 0x0d })
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += setOf(identifierOf(keyA), stale)
        entries.cleanupThrows = IllegalStateException("database unavailable")

        val result = save(alias = "again", pickedType = LoadIdentityType.USER, typePicked = false)

        assertEquals(LoadSaveResult.SavedWithWarning(STALE_VOTING_KEY_NOT_REMOVED_MESSAGE), result)
        assertEquals(setOf(identifierOf(keyA), stale), entries.live)
        assertEquals("again", dao.getByIdentityId(identityId)!!.alias)
    }

    /** Item 1: the draft belongs to the decoded id and network, not to the text. */
    @Test
    fun shouldClearTheVotingKeyDraftOnlyWhenTheIdentityOrNetworkChanges() {
        val hex = identityId.joinToString("") { "%02x".format(it) }
        val base58 = org.dashfoundation.example.util.Base58.encode(identityId)
        val scope = votingKeyDraftScope(hex, testnet)
        // The same id typed as hex or Base58 is one scope.
        assertEquals(scope, votingKeyDraftScope(base58, testnet))
        assertFalse(shouldClearVotingKeyDraft(scope, votingKeyDraftScope(base58, testnet)))
        // Another identity, or the same one on another network, clears the draft.
        val other = votingKeyDraftScope("06".repeat(32), testnet)
        assertTrue(shouldClearVotingKeyDraft(scope, other))
        assertTrue(shouldClearVotingKeyDraft(scope, votingKeyDraftScope(hex, mainnet)))
        // A half-typed id clears nothing, and a draft typed before any id is kept.
        assertNull(votingKeyDraftScope(hex.dropLast(2), testnet))
        assertFalse(shouldClearVotingKeyDraft(scope, null))
        assertFalse(shouldClearVotingKeyDraft(null, scope))
    }

    /** Item 2: keeping the stored key runs the cleanup a previous warning promised. */
    @Test
    fun shouldCleanUpWhenTheStoredKeyIsKept() = runBlocking<Unit> {
        val stale = identifierOf(ByteArray(32) { 0x0d })
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        entries.live += setOf(identifierOf(keyA), stale)
        entries.sweepFailure = IllegalStateException("secure storage unavailable")

        assertEquals(
            LoadSaveResult.SavedWithWarning(STALE_VOTING_KEY_NOT_REMOVED_MESSAGE),
            keepStoredVotingKey(entries, identityId),
        )
        assertEquals(setOf(identifierOf(keyA), stale), entries.live)

        entries.sweepFailure = null
        assertEquals(LoadSaveResult.Saved, keepStoredVotingKey(entries, identityId))
        assertEquals(setOf(identifierOf(keyA)), entries.live)
    }

    /** Item 3: a row recreated on another network is not the one being loaded. */
    @Test
    fun shouldRefuseToReplaceAKeyOnAnotherNetworksRow() = runBlocking<Unit> {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA), network = mainnet))

        val result = replaceVotingKey(dao, entries, identityId, testnet, identifierOf(keyA), keyB)

        assertEquals(LoadSaveResult.Failed(VOTING_KEY_CHANGED_MESSAGE), result)
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(emptyList<String>(), entries.storedIdentifiers)
    }

    /** Item 10: an unpicked type keeps the stored one; a picked type is written. */
    @Test
    fun shouldKeepTheStoredTypeUnlessOneIsGiven() = runBlocking<Unit> {
        dao.upsert(row(type = "Evonode"))

        assertEquals(1, dao.updateLoadedFields(identityId, testnet, 1L, null, true, identityType = null, nowMillis = 0))
        assertEquals("Evonode", dao.getByIdentityId(identityId)!!.identityType)

        assertEquals(1, dao.updateLoadedFields(identityId, testnet, 1L, null, true, identityType = "Masternode", nowMillis = 0))
        assertEquals("Masternode", dao.getByIdentityId(identityId)!!.identityType)

        // Through the load: an unpicked type leaves the stored one.
        save(alias = "x", pickedType = LoadIdentityType.USER, typePicked = false)
        assertEquals("Masternode", dao.getByIdentityId(identityId)!!.identityType)
        save(alias = "x", pickedType = LoadIdentityType.EVONODE, typePicked = true)
        assertEquals("Evonode", dao.getByIdentityId(identityId)!!.identityType)
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    private suspend fun save(
        alias: String? = null,
        pickedType: LoadIdentityType,
        typePicked: Boolean,
        votingKey: ByteArray? = null,
        votingKeyCheck: MasternodeVotingKeyCheck? = null,
        masternodeProTxHash: ByteArray? = null,
        masternodeRole: MasternodeVotingLoadedIdentity.Role? = null,
        newRowType: LoadIdentityType? = null,
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
        votingKeyCheck = votingKeyCheck,
        masternodeProTxHash = masternodeProTxHash,
        masternodeRole = masternodeRole,
        newRowType = newRowType,
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
        /** When set, the cleanup reports failure (as the SDK does) and deletes nothing. */
        var sweepFailure: Exception? = null

        /** When set, the cleanup throws it instead. */
        var cleanupThrows: Exception? = null
        var onStore: (suspend () -> Unit)? = null

        override fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String =
            MasternodeVotingKeyStore.identifierFor(identityId, votingKey.copyOf(20))

        /**
         * What `MasternodeVotingKeyStore.assignToIdentity` does, over [live] instead of
         * secure storage, under the SDK's real voting key lock. The SDK's own tests cover
         * the real operation.
         */
        override suspend fun assign(
            identityId: ByteArray,
            networkRaw: Int,
            votingKey: ByteArray,
            expectedIdentifier: String?,
            nowMillis: Long,
        ): VotingKeyAssignment = MasternodeVotingKeyStore.withVotingKeyLock {
            failure?.let { throw it }
            val before = dao.getByIdentityId(identityId)?.takeIf { it.networkRaw == networkRaw }
            if (before == null || before.votingPrivateKeyIdentifier != expectedIdentifier) {
                return@withVotingKeyLock VotingKeyAssignment.Changed(before?.votingPrivateKeyIdentifier)
            }
            val identifier = identifierFor(identityId, votingKey)
            storedIdentifiers += identifier
            live += identifier
            onStore?.invoke()
            if (dao.replaceVotingPrivateKeyIdentifier(identityId, networkRaw, expectedIdentifier, identifier, nowMillis) == 0) {
                val current = dao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier
                if (current != identifier) {
                    deletedIdentifiers += identifier
                    live -= identifier
                }
                VotingKeyAssignment.Changed(current)
            } else {
                VotingKeyAssignment.Assigned(identifier)
            }
        }

        /**
         * What `MasternodeVotingKeyStore.removeStaleEntries` does, over [live], under the
         * SDK's real voting key lock.
         */
        override suspend fun removeStaleEntries(identityId: ByteArray): Boolean =
            MasternodeVotingKeyStore.withVotingKeyLock {
                cleanupThrows?.let { throw it }
                if (sweepFailure != null) return@withVotingKeyLock false
                val keep = dao.getByIdentityId(identityId)?.votingPrivateKeyIdentifier
                val prefix = MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20))
                    .substringBeforeLast('.')
                live.removeAll { it.startsWith("$prefix.") && it != keep }
                true
            }

        /** The SDK's real record write over the in-memory database, with [rules]' answer. */
        override suspend fun recordCheck(
            identityId: ByteArray,
            networkRaw: Int,
            identifier: String,
            check: MasternodeVotingKeyCheck,
        ): MasternodeVotingKeyCheckOutcome? {
            recordFailure?.let { throw it }
            beforeRecord?.invoke()
            return MasternodeVotingKeyRecords.record(db, identityId, networkRaw, identifier, check, rules)
        }

        /** When set, recording a check throws it. */
        var recordFailure: Exception? = null

        /** Runs just before a check is recorded (another write landing in between). */
        var beforeRecord: (suspend () -> Unit)? = null

        /** When set, recording the siblings throws it. */
        var siblingFailure: Exception? = null

        /** The SDK's real sibling write over the in-memory database, with [rules]' answer. */
        override suspend fun recordSiblings(
            networkRaw: Int,
            proTxHash: ByteArray,
            loadedIdentityId: ByteArray,
            loadedIdentifier: String,
            check: MasternodeVotingKeyCheck,
        ) {
            siblingFailure?.let { throw it }
            MasternodeVotingKeyRecords.recordSiblings(
                db, networkRaw, proTxHash, loadedIdentityId, loadedIdentifier, check, rules,
            )
        }
    }

    /**
     * Returns the outcome each test sets and records what it was asked. Stands in for Rust,
     * which the JVM cannot load; it never decides anything itself.
     */
    private class AnsweringRules : MasternodeVotingKeyRules {
        var answer = MasternodeVotingKeyCheckOutcome(storeKey = true, record = null)
        val asked = mutableListOf<MasternodeVotingKeySlot>()

        override fun outcome(
            check: MasternodeVotingKeyCheck,
            votingAddress: MasternodeVotingKeyAddress,
            slot: MasternodeVotingKeySlot,
        ): MasternodeVotingKeyCheckOutcome {
            asked += slot
            return answer
        }

        override fun use(
            recorded: MasternodeRecordedVotingKey?,
            storedVotingAddress: MasternodeVotingKeyAddress?,
        ): MasternodeRecordedVotingKeyUse = error("not used by Load Identity")

        /** What [siblingRecord] returns for each sibling; null leaves it unchanged. */
        var siblingAnswer: MasternodeRecordedVotingKey? = null

        override fun siblingRecord(
            loaded: MasternodeVotingKeyCheck,
            loadedAddress: MasternodeVotingKeyAddress,
            sibling: MasternodeRecordedVotingKey?,
        ): MasternodeRecordedVotingKey? = siblingAnswer
    }

    // ── Checking the entered key against Platform (#6086) ───────────────

    private fun addressOf(key: ByteArray) = MasternodeVotingKeyAddress.of(key.copyOf(20))!!

    private suspend fun preflight(
        check: MasternodeVotingKeyCheck,
        key: ByteArray = keyA,
        stillCurrent: () -> Boolean = { true },
    ) = checkEnteredVotingKey(
        dao, entries, rules, identityId, testnet, identifierOf(key), check, stillCurrent = stillCurrent,
    )

    /** A key Rust refuses stops the load: nothing is saved, and the reason is Rust's. */
    @Test
    fun shouldRefuseAKeyRustDoesNotStoreAndSaveNothing() = runBlocking {
        rules.answer = MasternodeVotingKeyCheckOutcome(storeKey = false, record = null)

        val result = preflight(MasternodeVotingKeyCheck.Invalid("No voting identity exists."))

        assertEquals(
            EnteredVotingKeyCheck.Refused(
                votingKeyNotOfMasternodeMessage(identityId.toHexString(), "No voting identity exists."),
            ),
            result,
        )
        assertEquals(listOf<MasternodeVotingKeySlot>(MasternodeVotingKeySlot.Empty), rules.asked)
        assertNull(dao.getByIdentityId(identityId))
    }

    /** Swift: a refusal of the key already stored is recorded on it, so it is no longer offered. */
    @Test
    fun shouldRecordARefusalOfTheStoredKeyOnItsRow() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        val refusal = MasternodeVotingKeyCheck.Invalid("rotated")
        rules.answer = MasternodeVotingKeyCheckOutcome(
            storeKey = false,
            record = MasternodeRecordedVotingKey(refusal, addressOf(keyA)),
        )

        val result = preflight(refusal)

        assertTrue(result is EnteredVotingKeyCheck.Refused)
        assertEquals(MasternodeVotingKeySlot.SameKey(null), rules.asked.first())
        assertEquals(
            MasternodeRecordedVotingKey(refusal, addressOf(keyA)),
            dao.getByIdentityId(identityId)!!.recordedVotingKey,
        )
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
    }

    /** #6099: a load refused for a voter identity shows Rust's words and records on that row. */
    @Test
    fun shouldShowRustsRefusalAndRecordItOnTheStoredVoterRow() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        val refusal = MasternodeVotingKeyCheck.Invalid("No owner identity, and the key was rotated.")
        rules.answer = MasternodeVotingKeyCheckOutcome(
            storeKey = false,
            record = MasternodeRecordedVotingKey(refusal, addressOf(keyA)),
        )

        val result = checkEnteredVotingKey(
            dao, entries, rules, identityId, testnet, identifierOf(keyA), refusal,
            refusalText = refusal.message,
        )

        assertEquals(EnteredVotingKeyCheck.Refused(refusal.message), result)
        assertEquals(
            MasternodeRecordedVotingKey(refusal, addressOf(keyA)),
            dao.getByIdentityId(identityId)!!.recordedVotingKey,
        )
    }

    @Test
    fun shouldSayWhenTheRefusalOfTheStoredKeyCouldNotBeRecorded() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        val refusal = MasternodeVotingKeyCheck.Invalid("rotated")
        rules.answer = MasternodeVotingKeyCheckOutcome(
            storeKey = false,
            record = MasternodeRecordedVotingKey(refusal, addressOf(keyA)),
        )
        entries.recordFailure = IllegalStateException("disk full")

        val result = preflight(refusal) as EnteredVotingKeyCheck.Refused

        assertTrue(result.message, result.message.endsWith(verdictNotSavedMessage("disk full")))
    }

    @Test
    fun shouldReportAnotherStoredKeyAsADifferentKey() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyB)))

        assertEquals(EnteredVotingKeyCheck.Proceed, preflight(MasternodeVotingKeyCheck.Valid))
        assertEquals(listOf<MasternodeVotingKeySlot>(MasternodeVotingKeySlot.DifferentKey), rules.asked)
    }

    /** Once the new key is in its empty slot, what Rust says to record is written on the row. */
    @Test
    fun shouldRecordTheCheckOnceTheNewKeyIsStored() = runBlocking {
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressOf(keyA))
        rules.answer = MasternodeVotingKeyCheckOutcome(storeKey = true, record = valid)

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
        )

        assertEquals(LoadSaveResult.Saved, result)
        // The new key is in its slot with nothing recorded for it yet.
        assertEquals(listOf<MasternodeVotingKeySlot>(MasternodeVotingKeySlot.SameKey(null)), rules.asked)
        assertEquals(valid, dao.getByIdentityId(identityId)!!.recordedVotingKey)
    }

    /** The same key loaded again is reported with the record it holds. */
    @Test
    fun shouldReportTheSameKeyWithItsRecordWhenItIsLoadedAgain() = runBlocking {
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressOf(keyA))
        dao.upsert(
            row(type = "Masternode", votingIdentifier = identifierOf(keyA)).copy(
                votingKeyCheckRaw = "valid",
                votingKeyCheckAddress = addressOf(keyA).bytes,
            ),
        )
        entries.live += identifierOf(keyA)
        val undetermined = MasternodeVotingKeyCheck.Undetermined("Platform did not answer.")
        // Rust keeps a proven record of the same key when the check could not run.
        rules.answer = MasternodeVotingKeyCheckOutcome(storeKey = true, record = null)

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = undetermined,
        )

        assertEquals(listOf<MasternodeVotingKeySlot>(MasternodeVotingKeySlot.SameKey(valid)), rules.asked)
        assertEquals(valid, dao.getByIdentityId(identityId)!!.recordedVotingKey)
        assertEquals(
            LoadSaveResult.SavedWithWarning(
                "The identity was loaded and its voting key was stored. " +
                    "Platform did not answer. The result of its earlier check is kept.",
            ),
            result,
        )
    }

    @Test
    fun shouldSayAnUncheckedNewKeyIsCheckedWhenPicked() = runBlocking {
        val undetermined = MasternodeVotingKeyCheck.Undetermined("Platform did not answer.")
        rules.answer = MasternodeVotingKeyCheckOutcome(
            storeKey = true,
            record = MasternodeRecordedVotingKey(undetermined, addressOf(keyA)),
        )

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = undetermined,
        )

        assertEquals(
            LoadSaveResult.SavedWithWarning(
                "The identity was loaded and its voting key was stored. Platform did not " +
                    "answer. It is checked again when you pick this identity to vote.",
            ),
            result,
        )
        assertEquals("undetermined", dao.getByIdentityId(identityId)!!.votingKeyCheckRaw)
    }

    /** A record that could not be written leaves the key stored and says it is checked later. */
    @Test
    fun shouldKeepTheStoredKeyWhenItsCheckCannotBeRecorded() = runBlocking {
        entries.recordFailure = IllegalStateException("disk full")

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
        )

        assertEquals(
            LoadSaveResult.SavedWithWarning(
                "The identity was loaded and its voting key was stored. " +
                    votingKeyCheckNotSavedMessage("disk full"),
            ),
            result,
        )
        assertEquals(identifierOf(keyA), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertNull(dao.getByIdentityId(identityId)!!.votingKeyCheckRaw)
    }

    /** A confirmed replacement records the entered key's check, the slot having held another key. */
    /**
     * #6145: a voting key stored through the replacement dialog marks the other stored keys of
     * the same masternode as a load does, and says so when they could not be marked.
     */
    @Test
    fun shouldMarkTheOtherKeysOfTheMasternodeAfterAReplacement() = runBlocking {
        val keyC = ByteArray(32) { 0x0c }
        val proTxHash = ByteArray(32) { (0x40 + it).toByte() }
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)).copy(masternodeProTxHash = proTxHash))
        entries.live += identifierOf(keyA)
        val siblingId = ByteArray(32) { 6 }
        dao.upsert(
            IdentityEntity(
                identityId = siblingId,
                identityType = "Masternode",
                votingPrivateKeyIdentifier = MasternodeVotingKeyStore.identifierFor(siblingId, keyC.copyOf(20)),
                networkRaw = testnet,
                votingKeyCheckRaw = "valid",
                votingKeyCheckAddress = addressOf(keyC).bytes,
                masternodeProTxHash = proTxHash,
            ),
        )
        val recheck = MasternodeRecordedVotingKey(
            MasternodeVotingKeyCheck.Undetermined("Another voting key was loaded"),
            addressOf(keyC),
        )
        rules.siblingAnswer = recheck
        rules.answer = MasternodeVotingKeyCheckOutcome(
            storeKey = true,
            record = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressOf(keyB)),
        )

        val result = replaceVotingKey(
            dao, entries, identityId, testnet, identifierOf(keyA), keyB,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
        )

        assertEquals(LoadSaveResult.Saved, result)
        assertEquals(recheck, dao.getByIdentityId(siblingId)!!.recordedVotingKey)

        // The rows cannot be marked: the replacement stands and says so.
        entries.siblingFailure = IllegalStateException("disk full")
        entries.live += identifierOf(keyB)
        val failed = replaceVotingKey(
            dao, entries, identityId, testnet, identifierOf(keyB), keyA,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
        )
        assertEquals(
            LoadSaveResult.SavedWithWarning("The voting key was replaced. " + siblingVotingKeysNotMarkedMessage("disk full")),
            failed,
        )
    }

    @Test
    fun shouldRecordTheCheckOfAReplacementKey() = runBlocking {
        dao.upsert(
            row(type = "Masternode", votingIdentifier = identifierOf(keyA)).copy(
                votingKeyCheckRaw = "invalid",
                votingKeyCheckMessage = "rotated",
                votingKeyCheckAddress = addressOf(keyA).bytes,
            ),
        )
        entries.live += identifierOf(keyA)
        val valid = MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Valid, addressOf(keyB))
        rules.answer = MasternodeVotingKeyCheckOutcome(storeKey = true, record = valid)

        val result = replaceVotingKey(
            dao, entries, identityId, testnet, identifierOf(keyA), keyB,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
        )

        assertEquals(LoadSaveResult.Saved, result)
        // Rust is given the new key with the record the row holds, the old key's, which Rust
        // does not count for the new one.
        assertEquals(
            listOf<MasternodeVotingKeySlot>(
                MasternodeVotingKeySlot.SameKey(
                    MasternodeRecordedVotingKey(MasternodeVotingKeyCheck.Invalid("rotated"), addressOf(keyA)),
                ),
            ),
            rules.asked,
        )
        assertEquals(valid, dao.getByIdentityId(identityId)!!.recordedVotingKey)
    }

    /** #6099: a masternode row records the wire-order pro_tx_hash its votes name. */
    @Test
    fun shouldRecordTheMasternodesProTxHashBesideTheRow() = runBlocking {
        val proTxHash = ByteArray(32) { 0x5a }

        val result = saveLoadedIdentity(
            dao = dao,
            entries = entries,
            identityId = identityId,
            networkRaw = testnet,
            balance = 1L,
            alias = null,
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = null,
            masternodeProTxHash = proTxHash,
        )

        assertEquals(LoadSaveResult.Saved, result)
        assertTrue(proTxHash.contentEquals(dao.getByIdentityId(identityId)!!.masternodeProTxHash))
    }

    /**
     * #6099 review: the pro_tx_hash goes in with the row's own fields, so an interrupted
     * first load never leaves the row without it, and a reload without one keeps it.
     */
    @Test
    fun shouldWriteTheProTxHashWithTheRowAndKeepItOnAReload() = runBlocking {
        val proTxHash = ByteArray(32) { 0x5a }
        save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, masternodeProTxHash = proTxHash)
        assertTrue(proTxHash.contentEquals(dao.getByIdentityId(identityId)!!.masternodeProTxHash))

        save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true, masternodeProTxHash = null)
        assertTrue(proTxHash.contentEquals(dao.getByIdentityId(identityId)!!.masternodeProTxHash))
    }

    /**
     * #6099 review: typing the id a shared masternode is stored under (its voter identity, as
     * the detail screen shows it) loads its recorded pro_tx_hash, not that id reversed.
     */
    @Test
    fun shouldLoadAStoredVoterRowByItsRecordedProTxHash() = runBlocking {
        // Byte sequences that change when reversed, so a wrong byte order would show.
        val typedId = ByteArray(32) { it.toByte() }
        val proTxHash = ByteArray(32) { (0x40 + it).toByte() }
        assertTrue(typedId.reversedArray().contentEquals(loadTargetProTxHash(dao, typedId, testnet)))

        dao.upsert(row(type = "Masternode").copy(identityId = typedId, masternodeProTxHash = proTxHash))
        assertTrue(proTxHash.contentEquals(loadTargetProTxHash(dao, typedId, testnet)))
        // A row on another network says nothing about this one.
        assertTrue(typedId.reversedArray().contentEquals(loadTargetProTxHash(dao, typedId, mainnet)))
    }

    /** #6145: the role Rust's load rule returned is recorded with the row's own fields. */
    @Test
    fun shouldRecordWhichIdentityTheLoadRuleReturned() = runBlocking {
        val proTxHash = ByteArray(32) { (0x40 + it).toByte() }
        save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            masternodeProTxHash = proTxHash,
            masternodeRole = MasternodeVotingLoadedIdentity.Role.VOTER,
        )

        val stored = dao.getByIdentityId(identityId)!!
        assertEquals("voter", stored.masternodeRoleRaw)
        assertTrue(stored.isVotingOnlyIdentity)
        // A reload without a role keeps it.
        save(pickedType = LoadIdentityType.MASTERNODE, typePicked = true)
        assertEquals("voter", dao.getByIdentityId(identityId)!!.masternodeRoleRaw)
    }

    /**
     * #6145: once the loaded key is stored, the other rows for the same masternode get what
     * Rust's sibling rule says (a rotated-out key is marked to be checked again).
     */
    @Test
    fun shouldMarkAnotherStoredKeyOfTheSameMasternodeToBeCheckedAgain() = runBlocking {
        val proTxHash = ByteArray(32) { (0x40 + it).toByte() }
        val siblingId = ByteArray(32) { 6 }
        val siblingIdentifier = MasternodeVotingKeyStore.identifierFor(siblingId, keyB.copyOf(20))
        dao.upsert(
            IdentityEntity(
                identityId = siblingId,
                identityType = "Masternode",
                votingPrivateKeyIdentifier = siblingIdentifier,
                networkRaw = testnet,
                votingKeyCheckRaw = "valid",
                votingKeyCheckAddress = addressOf(keyB).bytes,
                masternodeProTxHash = proTxHash,
            ),
        )
        val recheck = MasternodeRecordedVotingKey(
            MasternodeVotingKeyCheck.Undetermined("Another voting key was loaded"),
            addressOf(keyB),
        )
        rules.siblingAnswer = recheck

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
            masternodeProTxHash = proTxHash,
        )

        assertEquals(LoadSaveResult.Saved, result)
        assertEquals(recheck, dao.getByIdentityId(siblingId)!!.recordedVotingKey)
    }

    @Test
    fun shouldSayWhenTheOtherKeysOfTheMasternodeCouldNotBeMarked() = runBlocking {
        entries.siblingFailure = IllegalStateException("disk full")

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
            masternodeProTxHash = ByteArray(32) { 1 },
        )

        assertEquals(
            LoadSaveResult.SavedWithWarning("The identity was loaded. " + siblingVotingKeysNotMarkedMessage("disk full")),
            result,
        )
    }

    /** A refusal of the stored key is not recorded once the network changed during the read. */
    @Test
    fun shouldNotRecordARefusalAfterTheNetworkChanged() = runBlocking {
        dao.upsert(row(type = "Masternode", votingIdentifier = identifierOf(keyA)))
        val refusal = MasternodeVotingKeyCheck.Invalid("rotated")
        rules.answer = MasternodeVotingKeyCheckOutcome(
            storeKey = false,
            record = MasternodeRecordedVotingKey(refusal, addressOf(keyA)),
        )

        assertTrue(preflight(refusal, stillCurrent = { false }) is EnteredVotingKeyCheck.Refused)
        assertNull(dao.getByIdentityId(identityId)!!.votingKeyCheckRaw)
    }

    /** A key replaced before its check is recorded is reported, not passed as a success. */
    @Test
    fun shouldSayWhenTheKeyChangedBeforeItsCheckWasSaved() = runBlocking {
        entries.beforeRecord = {
            dao.replaceVotingPrivateKeyIdentifier(identityId, testnet, identifierOf(keyA), identifierOf(keyB), 0L)
        }

        val result = save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = true,
            votingKey = keyA,
            votingKeyCheck = MasternodeVotingKeyCheck.Valid,
        )

        assertEquals(
            LoadSaveResult.SavedWithWarning(
                "The identity was loaded and its voting key was stored. $VOTING_KEY_CHANGED_BEFORE_CHECK_MESSAGE",
            ),
            result,
        )
    }

    /**
     * A rotated shared masternode reloaded by the id its detail screen shows stores the new
     * voter identity under another id: a new row is saved with the masternode's type (the
     * form's, inferred from the typed row), so the picker offers it, never as User; an existing
     * row keeps its own type unless the user picked one.
     */
    @Test
    fun shouldSaveAReplacementVoterRowWithItsMasternodesType() = runBlocking {
        val typedId = ByteArray(32) { 6 }
        assertEquals(LoadIdentityType.EVONODE, newRowTypeFor(LoadIdentityType.EVONODE, identityId, typedId))
        assertNull(newRowTypeFor(LoadIdentityType.EVONODE, typedId, typedId))
        assertNull(newRowTypeFor(LoadIdentityType.USER, identityId, typedId))

        save(
            pickedType = LoadIdentityType.EVONODE,
            typePicked = false,
            masternodeRole = MasternodeVotingLoadedIdentity.Role.VOTER,
            newRowType = newRowTypeFor(LoadIdentityType.EVONODE, identityId, typedId),
        )
        assertEquals("Evonode", dao.getByIdentityId(identityId)!!.identityType)

        // Reloading through another row's id that infers Masternode leaves this row's type.
        save(
            pickedType = LoadIdentityType.MASTERNODE,
            typePicked = false,
            newRowType = newRowTypeFor(LoadIdentityType.MASTERNODE, identityId, typedId),
        )
        assertEquals("Evonode", dao.getByIdentityId(identityId)!!.identityType)
    }

    @Test
    fun shouldWordTheUncheckedNoteFromRustsOutcome() {
        val undetermined = MasternodeVotingKeyCheck.Undetermined("Down.")
        val kept = MasternodeVotingKeyCheckOutcome(true, null)
        val written = MasternodeVotingKeyCheckOutcome(
            true,
            MasternodeRecordedVotingKey(undetermined, addressOf(keyA)),
        )
        assertEquals("Down. The result of its earlier check is kept.", uncheckedVotingKeyNote(undetermined, kept))
        assertEquals(
            "Down. It is checked again when you pick this identity to vote.",
            uncheckedVotingKeyNote(undetermined, written),
        )
        assertNull(uncheckedVotingKeyNote(MasternodeVotingKeyCheck.Valid, written))
        assertNull(uncheckedVotingKeyNote(undetermined, null))
    }

    private fun ByteArray.toHexString(): String = joinToString("") { "%02x".format(it) }
}
