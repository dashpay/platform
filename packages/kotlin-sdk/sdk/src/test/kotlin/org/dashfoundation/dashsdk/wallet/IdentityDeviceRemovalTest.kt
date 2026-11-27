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
import org.dashfoundation.dashsdk.security.PendingIdentityKeyDelete
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
        hostPass: (suspend () -> Unit)? = null,
        settleKeys: suspend WalletStorage.PrivateKeyExclusion.(
            Collection<String>,
            Collection<PendingIdentityKeyDelete>,
        ) -> Unit =
            { delete, settled -> settlePendingIdentityKeyDeletes(delete, settled) },
    ) = deleteIdentityFromDevice(
        id, handler, db, storage, liveWallets, votingKeys, settleKeys,
        hostPass ?: { handler.deleteIdentityDataLocked(id) },
    )

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
    fun `should still clean up keys when the host pass fails after the persisted removal committed`() =
        runBlocking {
            // Rust accepts and emits the removal: the real persisted callback
            // commits its round, deleting the identity's whole footprint.
            liveWallets = { id ->
                handler.onChangesetBegin(walletId)
                handler.onPersistIdentityRemoval(walletId, id)
                handler.onChangesetEnd(walletId, success = true)
                true
            }

            // The host's redundant footprint pass then fails.
            val result = remove(hostPass = { throw IllegalStateException("database closed") })

            assertTrue(result.wasManaged)
            assertEquals(listOf("token-bob"), db.tokenDao().getBalancesOrderedByAmount().map { it.tokenId })
            assertTrue(
                db.privateContactScanStateDao().positionsByWallet(walletId)
                    .none { it.identityId.contentEquals(identityId) },
            )
            assertTrue(result.keysLeft.isEmpty())
            assertNull(db.identityDao().getByIdentityId(identityId))
            assertFalse(storage.hasPrivateKey(ownKey.hex()))
            assertFalse(storage.hasPrivateKey(votingIdentifier))
            assertTrue(storage.hasPrivateKey(sharedKey.hex()))
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
    fun `should delete a shared key whose first removal could not settle once the second identity goes`() =
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

            // The first removal's settle is refused: both claims and the
            // pending entry stay.
            remove { _, _ -> throw IllegalStateException("storage locked") }
            assertTrue(storage.hasPrivateKey(crossKey.hex()))

            // Removing the second identity settles both entries for the alias
            // together, after releasing both wallets' claims, and deletes it.
            val result = remove(id = otherIdentity)
            assertTrue(result.keysLeft.isEmpty())
            assertFalse(storage.hasPrivateKey(crossKey.hex()))
            assertTrue(storage.withPrivateKeyExclusion { pendingIdentityKeyDeletes() }.isEmpty())
        }

    @Test
    fun `should delete a key persisted after the snapshot when the host pass removes the rows`() =
        runBlocking {
            val lateKey = ByteArray(33) { 0x6a }
            // A round persists a new key for the identity while Rust answers
            // that no wallet manages it.
            liveWallets = {
                insertKey(identityId, 5, lateKey)
                storage.storePrivateKey(lateKey.hex(), ByteArray(32) { 4 }, ownerWalletId = walletId)
                false
            }

            val result = remove()

            assertTrue(result.keysLeft.isEmpty())
            assertFalse(storage.hasPrivateKey(lateKey.hex()))
            assertFalse(storage.withPrivateKeyExclusion { isOwnedByAnotherWallet(lateKey.hex(), ByteArray(32)) })
        }

    @Test
    fun `should delete a key two wallets still claim when both pending entries settle together`() =
        runBlocking {
            val otherWallet = ByteArray(32) { 0x12 }
            val orphanKey = ByteArray(33) { 0x6b }
            storage.storePrivateKey(orphanKey.hex(), ByteArray(32) { 5 }, ownerWalletId = walletId)
            storage.storePrivateKey(orphanKey.hex(), ByteArray(32) { 5 }, ownerWalletId = otherWallet)
            // Two removals were interrupted after their rows went and before
            // any settle: both claims intact, no row names the key.
            storage.withPrivateKeyExclusion {
                recordPendingIdentityKeyDeletes(
                    listOf(
                        PendingIdentityKeyDelete(orphanKey.hex(), walletId),
                        PendingIdentityKeyDelete(orphanKey.hex(), otherWallet),
                    ),
                )
            }

            // Any later removal settles them.
            val result = remove(id = ByteArray(32) { 0x7d })

            assertTrue(result.keysLeft.isEmpty())
            assertFalse(storage.hasPrivateKey(orphanKey.hex()))
            assertTrue(storage.withPrivateKeyExclusion { pendingIdentityKeyDeletes() }.isEmpty())
        }

    @Test
    fun `should leave a pending delete untouched while an open round stages its alias`() = runBlocking {
        val stagedKey = ByteArray(33) { 0x6c }
        storage.storePrivateKey(stagedKey.hex(), ByteArray(32) { 6 }, ownerWalletId = walletId)
        storage.withPrivateKeyExclusion {
            recordPendingIdentityKeyDeletes(listOf(PendingIdentityKeyDelete(stagedKey.hex(), walletId)))
        }

        // A round has stored the key for this wallet and staged its row, not
        // committed yet: no committed row names it.
        storage.withPrivateKeyExclusion {
            settlePendingIdentityKeyDeletes(db, this, inFlight = setOf(stagedKey.hex()))
        }
        assertTrue(storage.hasPrivateKey(stagedKey.hex()))
        assertTrue(storage.withPrivateKeyExclusion { isOwnedByAnotherWallet(stagedKey.hex(), ByteArray(32)) })
        assertEquals(1, storage.withPrivateKeyExclusion { pendingIdentityKeyDeletes() }.size)

        // Once the round is over (here: rolled back, no row), it settles.
        storage.withPrivateKeyExclusion { settlePendingIdentityKeyDeletes(db, this, inFlight = emptySet()) }
        assertFalse(storage.hasPrivateKey(stagedKey.hex()))
        assertTrue(storage.withPrivateKeyExclusion { pendingIdentityKeyDeletes() }.isEmpty())
    }

    @Test
    fun `should retry a deferred delete when the round that staged its alias rolls back`() =
        runBlocking {
            // The handler as the manager wires it: rounds that staged a key
            // settle pending deletes when they end.
            handler = PlatformWalletPersistenceHandler(
                db,
                Dispatchers.Unconfined,
                settlePendingKeyDeletesAfterRound = { inFlight ->
                    storage.withPrivateKeyExclusion { settlePendingIdentityKeyDeletes(db, this, inFlight) }
                },
            )
            val stagedKey = ByteArray(33) { 0x6d }
            storage.storePrivateKey(stagedKey.hex(), ByteArray(32) { 7 }, ownerWalletId = walletId)
            storage.withPrivateKeyExclusion {
                recordPendingIdentityKeyDeletes(listOf(PendingIdentityKeyDelete(stagedKey.hex(), walletId)))
            }

            // An open round stages a row for the alias; a removal meanwhile
            // defers its pending delete.
            handler.onChangesetBegin(walletId)
            handler.onPersistIdentityKeyUpsert(
                walletId, siblingId, 9, 0, 0, 0, false, false, 0,
                stagedKey, ByteArray(20), true, walletId,
                true, 3, 5, 0, ByteArray(32), null,
            )
            remove(id = ByteArray(32) { 0x7c })
            assertTrue(storage.hasPrivateKey(stagedKey.hex()))

            // The round rolls back: its row never commits, and the deferred
            // delete is settled right away, with no other removal.
            handler.onChangesetEnd(walletId, success = false)
            assertFalse(storage.hasPrivateKey(stagedKey.hex()))
            assertTrue(storage.withPrivateKeyExclusion { pendingIdentityKeyDeletes() }.isEmpty())
        }

    @Test
    fun `should settle a deferred delete when the round that staged its alias is abandoned`() =
        runBlocking<Unit> {
            handler = wiredHandler()
            val stagedKey = ByteArray(33) { 0x6e }
            storage.storePrivateKey(stagedKey.hex(), ByteArray(32) { 8 }, ownerWalletId = walletId)
            storage.withPrivateKeyExclusion {
                recordPendingIdentityKeyDeletes(listOf(PendingIdentityKeyDelete(stagedKey.hex(), walletId)))
            }
            handler.onChangesetBegin(walletId)
            stageKey(stagedKey)
            remove(id = ByteArray(32) { 0x7b })
            assertTrue(storage.hasPrivateKey(stagedKey.hex()))

            // The round never ends; the next round's begin replaces it.
            handler.onChangesetBegin(walletId)
            assertFalse(storage.hasPrivateKey(stagedKey.hex()))
            handler.onChangesetEnd(walletId, success = true)
        }

    @Test
    fun `should settle on recovery what a failed round-end settle left`() = runBlocking {
        var failRoundEnd = true
        handler = PlatformWalletPersistenceHandler(
            db,
            Dispatchers.Unconfined,
            settlePendingKeyDeletesAfterRound = { inFlight ->
                check(!failRoundEnd) { "storage locked" }
                storage.withPrivateKeyExclusion { settlePendingIdentityKeyDeletes(db, this, inFlight) }
            },
        )
        val stagedKey = ByteArray(33) { 0x6f }
        storage.storePrivateKey(stagedKey.hex(), ByteArray(32) { 9 }, ownerWalletId = walletId)
        storage.withPrivateKeyExclusion {
            recordPendingIdentityKeyDeletes(listOf(PendingIdentityKeyDelete(stagedKey.hex(), walletId)))
        }
        handler.onChangesetBegin(walletId)
        stageKey(stagedKey)
        handler.onChangesetEnd(walletId, success = false)
        assertTrue("the round-end settle failed", storage.hasPrivateKey(stagedKey.hex()))

        // The manager's load-time recovery settles it, with no removal.
        failRoundEnd = false
        handler.settlePendingIdentityKeyDeletesNow()
        assertFalse(storage.hasPrivateKey(stagedKey.hex()))
        assertTrue(storage.withPrivateKeyExclusion { pendingIdentityKeyDeletes() }.isEmpty())
    }

    /** The handler as the manager wires it: rounds settle pending deletes when they end. */
    private fun wiredHandler() = PlatformWalletPersistenceHandler(
        db,
        Dispatchers.Unconfined,
        settlePendingKeyDeletesAfterRound = { inFlight ->
            storage.withPrivateKeyExclusion { settlePendingIdentityKeyDeletes(db, this, inFlight) }
        },
    )

    /** Stage an identity-key row for [key] (on the sibling) in the open round. */
    private fun stageKey(key: ByteArray) {
        handler.onPersistIdentityKeyUpsert(
            walletId, siblingId, 9, 0, 0, 0, false, false, 0,
            key, ByteArray(20), true, walletId,
            true, 3, 5, 0, ByteArray(32), null,
        )
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
