package org.dashfoundation.dashsdk.wallet

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.persistence.UInt64Value
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DpnsNameEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.PrivateContactRequestsPositionEntity
import org.dashfoundation.dashsdk.persistence.entities.PrivateShieldedAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.PlaintextIdentityKeysKeystore
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * [deleteIdentityFromDevice] over an in-memory [DashDatabase] and a
 * [WalletStorage] on a plaintext test keystore. Kotlin counterpart of
 * `SwiftExampleAppTests/IdentityLocalRemovalTests.swift`, plus the live wallet
 * step (faked here; its Rust side is tested in `rs-platform-wallet`): a live
 * wallet refusal deletes nothing, every row of the identity goes, a sibling
 * identity keeps its rows and keys, a key another identity references stays
 * until the last one is removed, the keys are deleted only after the rows are
 * gone, a failed row delete touches no key, and an interrupted key cleanup is
 * reported and finished by the next removal.
 */
@RunWith(RobolectricTestRunner::class)
class IdentityDeviceRemovalTest {

    private lateinit var db: DashDatabase
    private lateinit var handler: PlatformWalletPersistenceHandler
    private lateinit var storage: WalletStorage
    private lateinit var votingKeys: MasternodeVotingKeyStore

    private val testnet = 1
    private val walletId = ByteArray(32) { 0x11 }
    private val identityId = ByteArray(32) { 0x22 }
    private val siblingId = ByteArray(32) { 0x33 }
    private val ownKey = ByteArray(33) { 0x44 }
    private val sharedKey = ByteArray(33) { 0x55 }
    private val siblingKey = ByteArray(33) { 0x66 }
    private val votingIdentifier =
        MasternodeVotingKeyStore.identifierFor(identityId, ByteArray(20) { 0x77 })

    @Before
    fun setUp() = runBlocking {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
        storage = WalletStorage(ApplicationProvider.getApplicationContext(), PlaintextIdentityKeysKeystore())
        storage.deleteAll()
        votingKeys = MasternodeVotingKeyStore(storage)
        MasternodeVotingKeyStore.resetPendingForTesting()
        seed()
    }

    @After
    fun tearDown() {
        db.close()
    }

    private suspend fun seed() {
        db.walletDao().upsert(WalletEntity(walletId = walletId, networkRaw = testnet))
        db.identityDao().upsert(
            IdentityEntity(
                identityId = identityId,
                networkRaw = testnet,
                walletId = walletId,
                identityType = "Masternode",
                votingPrivateKeyIdentifier = votingIdentifier,
            ),
        )
        db.identityDao().upsert(
            IdentityEntity(identityId = siblingId, networkRaw = testnet, walletId = walletId),
        )
        insertKey(identityId, 0, ownKey)
        insertKey(identityId, 1, sharedKey)
        insertKey(siblingId, 0, siblingKey)
        insertKey(siblingId, 1, sharedKey)
        for (key in listOf(ownKey, sharedKey, siblingKey)) {
            storage.storePrivateKey(key.hex(), ByteArray(32) { 1 }, ownerWalletId = walletId)
        }
        storage.storePrivateKey(votingIdentifier, ByteArray(32) { 2 })

        for ((owner, label) in listOf(identityId to "alice", siblingId to "bob")) {
            db.dpnsNameDao().upsert(
                DpnsNameEntity(
                    networkRaw = testnet,
                    label = label,
                    normalizedLabel = label,
                    identityId = owner,
                ),
            )
            db.dashpayDao().upsertProfile(DashpayProfileEntity(networkRaw = testnet, identityId = owner))
            db.tokenDao().insertBalance(
                TokenBalanceEntity(
                    tokenId = "token-$label",
                    identityId = owner,
                    balance = UInt64Value(5u),
                    networkRaw = testnet,
                ),
            )
        }
        db.dashpayDao().upsertContactRequest(
            DashpayContactRequestEntity(
                networkRaw = testnet,
                ownerIdentityId = identityId,
                contactIdentityId = siblingId,
                isOutgoing = true,
                senderKeyIndex = 0,
                recipientKeyIndex = 0,
                accountReference = 0,
                encryptedPublicKey = ByteArray(96),
                coreHeightCreatedAt = 1,
                createdAtMillis = 1,
            ),
        )
        val scan = db.privateContactScanStateDao()
        scan.upsertPosition(
            PrivateContactRequestsPositionEntity(walletId, identityId, 7, ByteArray(32)),
        )
        scan.upsertPosition(
            PrivateContactRequestsPositionEntity(walletId, siblingId, 9, ByteArray(32)),
        )
        scan.upsertAddress(PrivateShieldedAddressEntity(walletId, identityId, siblingId, ByteArray(43)))
    }

    private suspend fun insertKey(owner: ByteArray, keyId: Int, data: ByteArray) {
        db.publicKeyDao().insert(
            PublicKeyEntity(
                keyId = keyId,
                purpose = "AUTHENTICATION",
                securityLevel = "HIGH",
                keyType = "ECDSA_SECP256K1",
                publicKeyData = data,
                identityId = owner.toBase58String(),
                identityIdData = owner,
            ),
        )
    }

    /** What the live wallets answer: false (unmanaged) unless a test says otherwise. */
    private var liveWallets: suspend (ByteArray) -> Boolean = { false }

    private suspend fun remove(
        id: ByteArray = identityId,
        settleKeys: suspend WalletStorage.PrivateKeyExclusion.(Collection<String>, Collection<String>) -> Unit =
            { delete, settled -> settlePendingIdentityKeyDeletes(delete, settled) },
    ) = deleteIdentityFromDevice(id, handler, db, storage, liveWallets, votingKeys, settleKeys)

    @Test
    fun `should remove every row and key of the identity and keep the sibling`() = runBlocking {
        val result = remove()

        assertEquals(emptyList<String>(), result.keysLeft)
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertTrue(db.publicKeyDao().getByIdentityId(identityId.toBase58String()).isEmpty())
        assertTrue(db.dpnsNameDao().getAllByIdentity(identityId).isEmpty())
        assertNull(db.dashpayDao().getProfile(testnet, identityId))
        assertTrue(db.dashpayDao().getContactRequestsByOwner(identityId).isEmpty())
        assertEquals(listOf("token-bob"), db.tokenDao().getBalancesOrderedByAmount().map { it.tokenId })
        val scan = db.privateContactScanStateDao()
        assertTrue(scan.positionsByWallet(walletId).none { it.identityId.contentEquals(identityId) })
        assertTrue(scan.addressesByWallet(walletId).isEmpty())
        assertFalse(storage.hasPrivateKey(ownKey.hex()))
        assertFalse(storage.hasPrivateKey(votingIdentifier))

        // A key the sibling also references stays, as do the sibling's rows.
        assertTrue(storage.hasPrivateKey(sharedKey.hex()))
        assertTrue(storage.hasPrivateKey(siblingKey.hex()))
        assertNotNull(db.identityDao().getByIdentityId(siblingId))
        assertEquals(2, db.publicKeyDao().getByIdentityId(siblingId.toBase58String()).size)
        assertEquals(1, db.dpnsNameDao().getAllByIdentity(siblingId).size)
        assertNotNull(db.dashpayDao().getProfile(testnet, siblingId))
        assertTrue(scan.positionsByWallet(walletId).any { it.identityId.contentEquals(siblingId) })
    }

    @Test
    fun `should delete nothing when the live wallet refuses`() = runBlocking {
        liveWallets = { throw IllegalStateException("contact request in flight") }

        assertThrows(IllegalStateException::class.java) { runBlocking { remove() } }

        assertNotNull(db.identityDao().getByIdentityId(identityId))
        assertEquals(2, db.publicKeyDao().getByIdentityId(identityId.toBase58String()).size)
        assertTrue(storage.hasPrivateKey(ownKey.hex()))
        assertTrue(storage.hasPrivateKey(votingIdentifier))
    }

    @Test
    fun `should report whether a live wallet managed the identity`() = runBlocking {
        var asked: ByteArray? = null
        liveWallets = { id -> asked = id; true }

        assertTrue(remove().wasManaged)
        assertTrue(identityId.contentEquals(asked))
    }

    @Test
    fun `should keep an unresolved scan position but rewind it to the start`() = runBlocking {
        db.privateContactScanStateDao().upsertPosition(
            PrivateContactRequestsPositionEntity(
                walletId, identityId, 7, ByteArray(32), privatePaymentsUnresolved = true,
            ),
        )

        remove()

        val kept = db.privateContactScanStateDao().positionsByWallet(walletId)
            .single { it.identityId.contentEquals(identityId) }
        assertTrue(kept.privatePaymentsUnresolved)
        assertEquals(0L, kept.position)
    }

    @Test
    fun `should delete the keys only after the rows are gone`() = runBlocking {
        var rowWhenKeysWent: IdentityEntity? = IdentityEntity(identityId = identityId, networkRaw = 0)
        val result = remove { delete, settled ->
            rowWhenKeysWent = db.identityDao().getByIdentityId(identityId)
            settlePendingIdentityKeyDeletes(delete, settled)
        }

        assertTrue(result.keysLeft.isEmpty())
        assertNull(rowWhenKeysWent)
    }

    @Test
    fun `should delete no key when the row delete fails`() = runBlocking {
        db.openHelper.writableDatabase.execSQL(
            "CREATE TRIGGER refuse_identity_delete BEFORE DELETE ON identities " +
                "BEGIN SELECT RAISE(ABORT, 'refused'); END",
        )

        assertThrows(Exception::class.java) { runBlocking { remove() } }

        assertNotNull(db.identityDao().getByIdentityId(identityId))
        assertEquals(1, db.dpnsNameDao().getAllByIdentity(identityId).size)
        assertEquals(2, db.publicKeyDao().getByIdentityId(identityId.toBase58String()).size)
        assertTrue(storage.hasPrivateKey(ownKey.hex()))
        assertTrue(storage.hasPrivateKey(votingIdentifier))

        // The keys recorded as pending are still named by the rows, so the next
        // removal keeps them rather than deleting a live identity's key.
        db.openHelper.writableDatabase.execSQL("DROP TRIGGER refuse_identity_delete")
        remove(id = ByteArray(32) { 0x7e })
        assertTrue(storage.hasPrivateKey(ownKey.hex()))
    }

    @Test
    fun `should finish an interrupted key cleanup when removed again`() = runBlocking {
        val first = remove { _, _ -> throw IllegalStateException("storage locked") }

        assertEquals(listOf(IdentityDeviceRemovalResult.IDENTITY_KEYS), first.keysLeft)
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertTrue(storage.hasPrivateKey(ownKey.hex()))
        // The voting key is a separate write and still goes.
        assertFalse(storage.hasPrivateKey(votingIdentifier))

        // Retrying the same id, whose rows are already gone, finishes the job.
        val retry = remove()
        assertTrue(retry.keysLeft.isEmpty())
        assertFalse(storage.hasPrivateKey(ownKey.hex()))
        assertTrue(storage.hasPrivateKey(sharedKey.hex()))
    }

    @Test
    fun `should delete a key two wallets shared once the last identity using it is removed`() =
        runBlocking {
            val otherWallet = ByteArray(32) { 0x12 }
            val otherIdentity = ByteArray(32) { 0x34 }
            val crossKey = ByteArray(33) { 0x56 }
            db.walletDao().upsert(WalletEntity(walletId = otherWallet, networkRaw = testnet))
            db.identityDao().upsert(
                IdentityEntity(identityId = otherIdentity, networkRaw = testnet, walletId = otherWallet),
            )
            insertKey(identityId, 2, crossKey)
            insertKey(otherIdentity, 0, crossKey)
            storage.storePrivateKey(crossKey.hex(), ByteArray(32) { 3 }, ownerWalletId = walletId)
            storage.storePrivateKey(crossKey.hex(), ByteArray(32) { 3 }, ownerWalletId = otherWallet)

            remove()
            assertTrue("kept for the other wallet's identity", storage.hasPrivateKey(crossKey.hex()))

            remove(id = otherIdentity)
            assertFalse("gone once no identity uses it", storage.hasPrivateKey(crossKey.hex()))
        }

    @Test
    fun `should finish a voting key an earlier removal left when removed again`() = runBlocking {
        remove()
        storage.storePrivateKey(votingIdentifier, ByteArray(32) { 2 })

        val result = remove()

        assertTrue(result.keysLeft.isEmpty())
        assertFalse(storage.hasPrivateKey(votingIdentifier))
    }

    private fun ByteArray.hex(): String = joinToString("") { "%02x".format(it) }
}
