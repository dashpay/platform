package org.dashfoundation.example.ui.identity

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
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
    fun shouldStartThePickerFromAStoredRowOnThisNetworkOnly() {
        val evonode = row(type = "Evonode")
        assertEquals(LoadIdentityType.EVONODE, initialLoadIdentityType(evonode, testnet))
        assertNull(initialLoadIdentityType(evonode, mainnet))
        assertNull(initialLoadIdentityType(null, testnet))
        assertNull(initialLoadIdentityType(row(type = "Something else"), testnet))
    }

    @Test
    fun shouldWriteAnUnpickedTypeOnlyForANewRow() {
        assertEquals("Evonode", identityTypeToSave("Evonode", LoadIdentityType.USER, typePicked = false))
        assertEquals("User", identityTypeToSave("Evonode", LoadIdentityType.USER, typePicked = true))
        assertEquals("User", identityTypeToSave(null, LoadIdentityType.USER, typePicked = false))
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
        val result = save(pickedType = LoadIdentityType.MASTERNODE, typePicked = false)

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

        val result = replaceVotingKey(dao, entries, identityId, identifierOf(keyA), keyB)

        assertEquals(LoadSaveResult.Saved, result)
        assertEquals(identifierOf(keyB), dao.getByIdentityId(identityId)!!.votingPrivateKeyIdentifier)
        assertEquals(listOf(identifierOf(keyA)), entries.deletedIdentifiers)
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

    // ── Helpers ──────────────────────────────────────────────────────────

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

    /** Records the entries written and deleted; no native library, no Keystore. */
    private inner class RecordedEntries : VotingKeyEntries {
        val storedIdentifiers = mutableListOf<String>()
        val deletedIdentifiers = mutableListOf<String>()
        var failure: Exception? = null
        var onStore: (suspend () -> Unit)? = null

        override fun identifierFor(identityId: ByteArray, votingKey: ByteArray): String =
            MasternodeVotingKeyStore.identifierFor(identityId, votingKey.copyOf(20))

        override suspend fun store(identityId: ByteArray, votingKey: ByteArray): String {
            failure?.let { throw it }
            val identifier = identifierFor(identityId, votingKey)
            storedIdentifiers += identifier
            onStore?.invoke()
            return identifier
        }

        override suspend fun delete(identifier: String) {
            deletedIdentifiers += identifier
        }
    }
}
