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
 * `SwiftExampleAppTests/IdentityLocalRemovalTests.swift`: every row of the
 * identity goes, a sibling identity keeps its rows and keys, a key another
 * identity references stays, the keys are deleted only after the rows are
 * gone, a failed row delete touches no key, and a refused key delete after the
 * rows are gone is reported instead of thrown.
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

    private suspend fun remove(
        deleteKeys: suspend WalletStorage.PrivateKeyExclusion.(Collection<String>) -> Unit =
            { deletePrivateKeys(it) },
    ) = deleteIdentityFromDevice(identityId, handler, db, storage, votingKeys, deleteKeys)

    @Test
    fun `should remove every row and key of the identity and keep the sibling`() = runBlocking {
        val result = remove()

        assertEquals(emptyList<String>(), result.keysLeft)
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertTrue(db.publicKeyDao().getByIdentityId(identityId.toBase58String()).isEmpty())
        assertTrue(db.dpnsNameDao().getAllByIdentity(identityId).isEmpty())
        assertNull(db.dashpayDao().getProfile(testnet, identityId))
        assertTrue(db.dashpayDao().getContactRequestsByOwner(identityId).isEmpty())
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
    fun `should keep a scan position whose private payments may be unresolved`() = runBlocking {
        db.privateContactScanStateDao().upsertPosition(
            PrivateContactRequestsPositionEntity(
                walletId, identityId, 7, ByteArray(32), privatePaymentsUnresolved = true,
            ),
        )

        remove()

        assertTrue(
            db.privateContactScanStateDao().positionsByWallet(walletId)
                .any { it.identityId.contentEquals(identityId) && it.privatePaymentsUnresolved },
        )
    }

    @Test
    fun `should delete the keys only after the rows are gone`() = runBlocking {
        var rowWhenKeysWent: IdentityEntity? = IdentityEntity(identityId = identityId, networkRaw = 0)
        val result = remove { names ->
            rowWhenKeysWent = db.identityDao().getByIdentityId(identityId)
            deletePrivateKeys(names)
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
    }

    @Test
    fun `should report keys the storage refuses after the rows are gone`() = runBlocking {
        val result = remove { throw IllegalStateException("storage locked") }

        assertEquals(listOf(IdentityDeviceRemovalResult.IDENTITY_KEYS), result.keysLeft)
        assertNull(db.identityDao().getByIdentityId(identityId))
        assertTrue(storage.hasPrivateKey(ownKey.hex()))
        // The voting key is a separate write and still goes.
        assertFalse(storage.hasPrivateKey(votingIdentifier))
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
