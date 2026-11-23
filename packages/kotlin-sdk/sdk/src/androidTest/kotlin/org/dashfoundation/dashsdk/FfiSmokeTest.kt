package org.dashfoundation.dashsdk

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.Dispatchers
import org.dashfoundation.dashsdk.ffi.AddressHistoryNative
import org.dashfoundation.dashsdk.ffi.ContractModerationNative
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.DashpayNative
import org.dashfoundation.dashsdk.ffi.DpnsGiveUpNative
import org.dashfoundation.dashsdk.ffi.IdentityRemovalNative
import org.dashfoundation.dashsdk.ffi.IdentityWithdrawalsNative
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.dashfoundation.dashsdk.ffi.TransactionsNative
import org.dashfoundation.dashsdk.ffi.SignerNative
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * A-M1 gate: proves the native library loads on-device, JNI_OnLoad ran,
 * symbols resolve, and an SDK handle round-trips create → getNetwork →
 * destroy. No network access is required — `createTrusted` only builds the
 * client; it does not connect.
 */
@RunWith(AndroidJUnit4::class)
class FfiSmokeTest {

    /**
     * The DPNS give-up exports bind with the descriptors
     * `DpnsGiveUpNative` declares: bad input is refused at the JNI boundary
     * (code 1), an unknown wallet handle is not found by Rust, and the pure
     * main-name rule answers without a wallet.
     */
    @Test
    fun dpnsGiveUpSymbolsBindAndRejectInvalidInput() {
        NativeLoader.ensureLoaded()
        val shortId = assertThrows(DashSDKException::class.java) {
            DpnsGiveUpNative.choices(0L, ByteArray(31), "alice7")
        }
        assertEquals(1, shortId.code)
        assertThrows(DashSDKException::class.java) {
            DpnsGiveUpNative.choices(0L, ByteArray(32), "alice7")
        }
        val badMethod = assertThrows(DashSDKException::class.java) {
            DpnsGiveUpNative.giveUp(0L, ByteArray(32), "alice7", ByteArray(32), 256, 1L)
        }
        assertEquals(1, badMethod.code)
        val noSigner = assertThrows(DashSDKException::class.java) {
            DpnsGiveUpNative.giveUp(0L, ByteArray(32), "alice7", ByteArray(32), 0, 0L)
        }
        assertEquals(1, noSigner.code)
        assertTrue(DpnsGiveUpNative.clearsMainName("alice7.dash", "Alice7"))
        assertFalse(DpnsGiveUpNative.clearsMainName("Keeper77", "Alice7"))
        assertFalse(DpnsGiveUpNative.clearsMainName(null, "Alice7"))
    }

    @Test
    fun documentEraseAndObserveBindsAndRejectsInvalidInput() {
        NativeLoader.ensureLoaded()
        val error = assertThrows(DashSDKException::class.java) {
            TransactionsNative.documentEraseAndObserve(
                0L, ByteArray(32), ByteArray(32), "note", ByteArray(32), 0L,
            )
        }
        assertEquals(1003, error.code)
    }

    @Test
    fun nativeLibraryLoadsAndReportsVersion() {
        NativeLoader.ensureLoaded()
        val version = SdkNative.version()
        assertTrue("version should be non-empty", version.isNotEmpty())
    }

    /**
     * The contract moderation and fee pot reads (protocol version 14) bind with the
     * descriptors `ContractModerationNative` declares: each export reaches its C function,
     * which refuses a null SDK handle, and an argument the C type cannot carry (a list or
     * status byte above 255, a limit that is neither -1 nor a `u32`) is refused by the JNI
     * layer with code 1 before any C call. The reads themselves need a protocol version 14
     * network; their Rust side is covered by the rs-sdk-ffi tests.
     */
    @Test
    fun contractModerationSymbolsBindAndRejectInvalidInput() {
        NativeLoader.ensureLoaded()
        val contract = "11111111111111111111111111111111"
        val noLimit = ContractModerationNative.NO_LIMIT
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getStatus(0L, contract, contract, null)
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getStatus(0L, contract, contract, ByteArray(0))
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getEntries(0L, contract, 0, null, noLimit)
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getDocumentRemovals(0L, contract, "note", null, 20)
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getDocumentRemovalsByIds(
                0L, contract, "note", arrayOf(contract),
            )
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getTeamActions(0L, contract, 0, null, false, noLimit)
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getTeamActionSigners(0L, contract, 1, contract)
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getActionCounts(0L, contract)
        }
        assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getFeePots(0L, contract)
        }
        val badList = assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getEntries(0L, contract, 256, null, noLimit)
        }
        assertEquals(1, badList.code)
        val badStatus = assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getTeamActionSigners(0L, contract, -1, contract)
        }
        assertEquals(1, badStatus.code)
        val badLimit = assertThrows(DashSDKException::class.java) {
            ContractModerationNative.getDocumentRemovals(0L, contract, "note", null, -2)
        }
        assertEquals(1, badLimit.code)
    }

    @Test
    fun devnetQuorumBaseUrlResolvesANameAndRefusesAReservedOne() {
        NativeLoader.ensureLoaded()
        assertEquals(
            "https://quorums.palmatum.networks.dash.org",
            SdkNative.devnetQuorumBaseUrl("palmatum"),
        )
        assertThrows(DashSDKException::class.java) {
            SdkNative.devnetQuorumBaseUrl("mainnet")
        }
    }

    @Test
    fun spvStartTellsAnEmptyPeerListFromOneWithNoUsablePeer() {
        NativeLoader.ensureLoaded()
        // No manager (handle 0), so nothing starts: the peer check runs
        // before the handle is looked up.
        fun restrictedStart(peers: Array<String>) = assertThrows(DashSDKException::class.java) {
            WalletManagerNative.spvStart(
                managerHandle = 0,
                dataDir = "/nonexistent",
                network = 1,
                userAgent = null,
                peers = peers,
                restrictToConfiguredPeers = true,
                startFromHeight = 0,
                devnetName = null,
                llmqDevnetSize = 0,
                llmqDevnetThreshold = 0,
            )
        }
        // An empty list is an offline start: it passes the peer check and
        // fails on the missing manager (platform-wallet NotFound, 98).
        assertEquals(1098, restrictedStart(emptyArray()).code)
        // A peer that cannot cross as a C string still counts as given, so
        // the start is refused (platform-wallet ErrorInvalidParameter, 2).
        assertEquals(1002, restrictedStart(arrayOf("192.0.2.1:19\u0000999")).code)
    }

    @Test
    fun sdkHandleRoundTrip() {
        NativeLoader.ensureLoaded()
        // Testnet, default DAPI address list, trusted quorum defaults.
        val handle = SdkNative.createTrusted(
            network = 1,
            dapiAddresses = null,
            quorumUrl = null,
            skipAssetLockProofVerification = false,
            requestRetryCount = 3,
            requestTimeoutMs = 30_000,
            platformVersion = 0,
        )
        try {
            assertNotEquals("createTrusted must return a live handle", 0L, handle)
            assertEquals("handle should report testnet", 1, SdkNative.getNetwork(handle))
        } finally {
            SdkNative.destroy(handle)
        }
    }

    @Test
    fun mnemonicAndPathSignerSymbolLoadsAndSigns() {
        NativeLoader.ensureLoaded()
        val mnemonicUtf8 = (
            "abandon abandon abandon abandon abandon abandon abandon abandon " +
                "abandon abandon abandon about"
        ).toByteArray(Charsets.UTF_8)
        val signature = try {
            SignerNative.signWithMnemonicAndPathInto(
                mnemonicUtf8 = mnemonicUtf8,
                derivationPath = "m/9'/5'/3'/0/0",
                network = Network.TESTNET.ffiValue,
                data = "jni signer smoke".toByteArray(Charsets.UTF_8),
            )
        } finally {
            // JNI copies the phrase into Rust-owned zeroizing memory but
            // deliberately does not mutate the JVM array. Direct callers own
            // this cleanup; the production wrapper applies the same finally.
            mnemonicUtf8.fill(0)
        }

        // Assert only what this smoke test can prove: the native symbol binds
        // and derive-and-sign returns a compact recoverable signature. The
        // array is zeroed by the test's own `finally` above, so asserting it
        // is now zero would only re-check that `fill(0)` works — it would say
        // nothing about JNI, which by contract never scrubs the caller's array.
        assertNotNull("native derive-and-sign should return a signature", signature)
        assertEquals("compact recoverable ECDSA signature", 65, signature!!.size)
    }

    /**
     * DashPay private contact requests (protocol version 15): the
     * `DashpayNative` symbols bind with the descriptors Kotlin declares, and
     * bad input surfaces as a mapped [DashSDKException] rather than a crash.
     * A null resolver is refused before any wallet lookup. The explicit
     * private send and private-reply accept are exported only by a shielded
     * build, so they are probed only there; the sync, the reads and the
     * in-kind accept are exported by every build.
     * The flows themselves need a protocol version 15 network with shielded
     * funds; their Rust side is covered by the platform-wallet and
     * platform-wallet-ffi tests.
     */
    @Test
    fun privateContactRequestSymbolsBindAndRejectInvalidInput() {
        NativeLoader.ensureLoaded()
        assertThrows(DashSDKException::class.java) {
            DashpayNative.syncPrivateContactRequests(0L, 0L, 1)
        }
        if (Sdk.hasShielded()) {
            assertThrows(DashSDKException::class.java) {
                DashpayNative.sendPrivateContactRequest(
                    0L, ByteArray(32), 0L, 0, ByteArray(32), ByteArray(32),
                    null, null, 0L,
                )
            }
            assertThrows(DashSDKException::class.java) {
                DashpayNative.acceptPrivateContactRequest(
                    0L, ByteArray(32), 0L, 0, 0L, 0L,
                )
            }
        }
        // An unknown wallet handle is NotFound.
        assertThrows(DashSDKException::class.java) {
            DashpayNative.privateContactReadiness(0L, ByteArray(32))
        }
        assertThrows(DashSDKException::class.java) {
            DashpayNative.acceptContactRequestReplyingInKind(
                0L, ByteArray(32), 0L, 0, 0L, 0L,
            )
        }
        // The private contact read is benign on a miss (null, as Swift's
        // nil); the pair-hold, public-unconfirmed and restore-failure reads
        // report an unknown wallet handle.
        assertNull(DashpayNative.managedIdentityPrivateContact(0L, ByteArray(32)))
        assertThrows(DashSDKException::class.java) {
            DashpayNative.privatePairHeld(0L, ByteArray(32), ByteArray(32))
        }
        assertThrows(DashSDKException::class.java) {
            DashpayNative.publicContactRequestUnconfirmed(0L, ByteArray(32), ByteArray(32))
        }
        assertThrows(DashSDKException::class.java) {
            DashpayNative.privatePaymentsRestoreFailure(0L, ByteArray(32))
        }
    }

    /**
     * The live-wallet identity removal binds, and an unknown manager handle
     * throws the mapped [DashSDKException] instead of crashing. The removal
     * itself is covered by the platform-wallet and platform-wallet-ffi tests.
     */
    @Test
    fun removeIdentityFromDeviceSymbolBindsAndRejectsInvalidHandle() {
        NativeLoader.ensureLoaded()
        assertThrows(DashSDKException::class.java) {
            IdentityRemovalNative.removeIdentityFromDevice(0L, ByteArray(32))
        }
    }

    /**
     * The #4069 watermark-freeze fault latch surfaced by PR #4314: the
     * `syncFaultDetected` JNI symbol binds, and an invalid manager handle
     * surfaces as the mapped [DashSDKException] rather than a crash. The
     * latch behavior itself (freeze on dropped events / rejected store) is
     * covered by the shared Rust tests in
     * `packages/rs-platform-wallet/src/changeset/core_bridge.rs`.
     */
    @Test
    fun syncFaultDetectedSymbolBindsAndRejectsInvalidHandle() {
        NativeLoader.ensureLoaded()
        assertThrows(DashSDKException::class.java) {
            WalletManagerNative.syncFaultDetected(0L)
        }
    }

    /**
     * The six `AddressHistoryNative` exports bind with their descriptors and their arguments
     * arrive in order, without touching the network: a negative height or count is refused in
     * the JNI layer (code 1); a wallet handle Rust does not know reaches
     * `platform_wallet_get_platform` and comes back as a platform-wallet code; a null SDK
     * handle reaches each rs-sdk-ffi query, which refuses it (`InvalidParameter`, 1); a
     * 31-byte expected hash is refused before the query; and the branch depth comes from Rust
     * (0 for a null handle, within the platform version's 6..9 for a live one).
     */
    @Test
    fun addressHistorySymbolsBindAndRejectInvalidInput() {
        NativeLoader.ensureLoaded()
        fun code(block: () -> Unit): Int = assertThrows(DashSDKException::class.java) { block() }.code

        assertEquals(1, code { AddressHistoryNative.balanceHistory(1L, false, -1L) })
        assertTrue(code { AddressHistoryNative.balanceHistory(Long.MAX_VALUE, true, 0L) } >= 1000)
        assertEquals(1, code { AddressHistoryNative.recentBalanceChanges(0L, 0L) })
        assertEquals(1, code { AddressHistoryNative.recentBalanceChanges(1L, -1L) })
        assertEquals(1, code { AddressHistoryNative.compactedBalanceChanges(0L, 0L) })
        assertEquals(1, code { AddressHistoryNative.trunkState(0L) })
        assertEquals(1, code { AddressHistoryNative.branchState(0L, byteArrayOf(1), 6, ByteArray(31), 1L) })
        assertEquals(1, code { AddressHistoryNative.branchState(0L, byteArrayOf(1), 6, ByteArray(32), 1L) })
        assertEquals(0, AddressHistoryNative.branchQueryDepth(0L, 0L))

        val sdk = SdkNative.createTrusted(
            network = 1,
            dapiAddresses = null,
            quorumUrl = null,
            skipAssetLockProofVerification = false,
            requestRetryCount = 3,
            requestTimeoutMs = 30_000,
            platformVersion = 0,
        )
        try {
            val depth = AddressHistoryNative.branchQueryDepth(sdk, 0L)
            assertTrue("depth $depth", depth in 6..9)
            assertTrue(AddressHistoryNative.branchQueryDepth(sdk, 1_000_000L) >= depth)
        } finally {
            SdkNative.destroy(sdk)
        }
    }

    /**
     * `identityWithdrawals` binds with the descriptor `(J[B)[B` and its arguments arrive in
     * order: an identity id that is not 32 bytes is refused in the JNI layer (code 1), and a
     * 32-byte id on a wallet handle Rust does not know reaches
     * `platform_wallet_identity_withdrawals`, which answers `NotFound` (98, thrown shifted
     * by the platform-wallet offset). Nothing reaches the network.
     */
    @Test
    fun identityWithdrawalsSymbolBindsAndRejectsInvalidInput() {
        NativeLoader.ensureLoaded()
        val shortId = assertThrows(DashSDKException::class.java) {
            IdentityWithdrawalsNative.identityWithdrawals(1L, ByteArray(31))
        }
        assertEquals(1, shortId.code)
        val unknownWallet = assertThrows(DashSDKException::class.java) {
            IdentityWithdrawalsNative.identityWithdrawals(Long.MAX_VALUE, ByteArray(32) { 7 })
        }
        assertEquals(1000 + 98, unknownWallet.code)
    }

    /**
     * The private contact request scan state needs every override it
     * depends on, and atomic changesets: JNI wires its slots only for a
     * bridge that overrides the contact upsert native calls, the
     * scan-state store, the scan-state load and the scan-state wallet list
     * (a read position vouches for the requests an inherited no-op contact
     * writer never stored, and a record kept marked after its wallet's
     * removal reaches the wallet only through that list), and
     * the wallet keeps the state only with `ATOMIC_CHANGESETS` too (an
     * un-ignore stores the sender's removal and the position rewind in one
     * changeset). JNI always wires the changeset begin and end callbacks,
     * so the atomic half is the bridge's declaration. A bridge declaring
     * both but leaving any override inherited, or declaring the scan state
     * without atomic rounds, has neither; one implementing all four and
     * declaring both, and the Room handler, keep both.
     */
    @Test
    fun privateContactScanStateCapabilityNeedsEveryOverride() {
        NativeLoader.ensureLoaded()
        val bits = NativePersistenceBridge.CAPABILITY_PRIVATE_CONTACT_REQUEST_SCAN_STATE or
            ATOMIC_CHANGESETS
        fun effective(bridge: NativePersistenceBridge): Boolean =
            WalletManagerNative.nativeEffectivePersistenceCapabilityBits(bridge) and bits == bits

        assertTrue("every override", effective(ScanStateBridge.Full()))
        assertFalse("no contact upsert", effective(ScanStateBridge.WithoutContactUpsert()))
        assertFalse("no scan-state store", effective(ScanStateBridge.WithoutScanStateStore()))
        assertFalse("no scan-state load", effective(ScanStateBridge.WithoutScanStateLoad()))
        assertFalse(
            "no scan-state wallet list",
            effective(ScanStateBridge.WithoutScanStateWalletList()),
        )
        assertFalse(
            "every override, not declared",
            effective(object : ScanStateBridge.Full() {
                override fun persistenceCapabilitiesBits(): Long = 0L
            }),
        )
        assertFalse(
            "every override, no atomic changesets",
            effective(object : ScanStateBridge.Full() {
                override fun persistenceCapabilitiesBits(): Long =
                    NativePersistenceBridge.CAPABILITY_PRIVATE_CONTACT_REQUEST_SCAN_STATE
            }),
        )

        val db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        try {
            assertTrue(
                "the Room handler",
                effective(PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)),
            )
        } finally {
            db.close()
        }
    }

    /**
     * The asset-lock operation ids need every override they depend on. JNI
     * wires the two slots only for a bridge that overrides the operation id
     * store, the operation id load and the asset lock upsert (the row an id
     * is stored on), and Rust then attests the capability only when the
     * bridge declares it. A wallet drain reads "no lock carries this id" as
     * "not sent", so a bridge that inherits any of the three no-ops, or does
     * not declare the bit, must not have it; one implementing all three and
     * declaring it has it, and the Room handler has everything a wallet
     * drain requires (`PersistenceCapabilities::WALLET_DRAIN`).
     */
    @Test
    fun assetLockOperationIdCapabilityNeedsEveryOverride() {
        NativeLoader.ensureLoaded()
        val bit = NativePersistenceBridge.CAPABILITY_ASSET_LOCK_OPERATION_IDS
        fun effective(bridge: NativePersistenceBridge): Boolean =
            WalletManagerNative.nativeEffectivePersistenceCapabilityBits(bridge) and bit == bit

        assertTrue("every override", effective(OperationIdBridge.Full()))
        assertFalse("no operation id store", effective(OperationIdBridge.WithoutStore()))
        assertFalse("no operation id load", effective(OperationIdBridge.WithoutLoad()))
        assertFalse("no asset lock upsert", effective(OperationIdBridge.WithoutUpsert()))
        assertFalse(
            "every override, not declared",
            effective(object : OperationIdBridge.Full() {
                override fun persistenceCapabilitiesBits(): Long = TRACKED_ASSET_LOCKS or WALLET_RESTORE
            }),
        )

        val db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        try {
            val handlerBits = WalletManagerNative.nativeEffectivePersistenceCapabilityBits(
                PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined),
            )
            assertEquals(
                "the Room handler attests everything a wallet drain requires",
                WALLET_DRAIN,
                handlerBits and WALLET_DRAIN,
            )
        } finally {
            db.close()
        }
    }

    /**
     * Rust↔Kotlin persistence-descriptor lockstep. The trampolines resolve
     * each bridge method by (name, descriptor) only when its slot first
     * fires — for the invitation upsert that would be mid-way through a
     * live, funded `create_invitation`. Resolving the whole table up front
     * turns a descriptor typo into a CI failure instead.
     */
    @Test
    fun persistenceBridgeDescriptorsAllResolve() {
        NativeLoader.ensureLoaded()
        val db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
        try {
            val handler = PlatformWalletPersistenceHandler(db, Dispatchers.Unconfined)
            val firstMismatch =
                WalletManagerNative.nativeVerifyPersistenceBridgeDescriptors(handler)
            assertNull(
                "bridge method failed to resolve (Rust descriptor vs Kotlin " +
                    "signature drift): $firstMismatch",
                firstMismatch,
            )
        } finally {
            db.close()
        }
    }
}

/** `PersistenceCapabilities::TRACKED_ASSET_LOCKS`. */
private const val TRACKED_ASSET_LOCKS: Long = 1L shl 9

/** `PersistenceCapabilities::WALLET_RESTORE`. */
private const val WALLET_RESTORE: Long = 1L shl 7

/**
 * `PersistenceCapabilities::WALLET_DRAIN` (0x2285): atomic changesets,
 * asset-lock funding indices, wallet restore, tracked asset locks and
 * asset-lock operation ids.
 */
private const val WALLET_DRAIN: Long = 0x2285

/**
 * Bridges declaring the asset-lock operation ids with some of the overrides
 * they depend on, for [FfiSmokeTest.assetLockOperationIdCapabilityNeedsEveryOverride].
 * Each concrete class declares exactly the overrides its name says.
 */
private abstract class OperationIdBridge : NativePersistenceBridge() {
    override fun persistenceCapabilitiesVersion(): Int = 1

    override fun persistenceCapabilitiesBits(): Long =
        CAPABILITY_ASSET_LOCK_OPERATION_IDS or TRACKED_ASSET_LOCKS or WALLET_RESTORE

    open class Full : OperationIdBridge() {
        @Suppress("LongParameterList")
        override fun onPersistAssetLockUpsert(
            walletId: ByteArray,
            outPoint: ByteArray,
            transactionBytes: ByteArray,
            accountIndex: Int,
            fundingType: Byte,
            identityIndex: Int,
            amountDuffs: Long,
            status: Byte,
            proofBytes: ByteArray?,
        ): Int = 0

        override fun onPersistAssetLockOperationId(
            walletId: ByteArray,
            outPoint: ByteArray,
            operationId: ByteArray,
        ): Int = 0

        override fun onLoadAssetLockOperationId(walletId: ByteArray, outPoint: ByteArray): ByteArray? =
            null
    }

    class WithoutStore : OperationIdBridge() {
        @Suppress("LongParameterList")
        override fun onPersistAssetLockUpsert(
            walletId: ByteArray,
            outPoint: ByteArray,
            transactionBytes: ByteArray,
            accountIndex: Int,
            fundingType: Byte,
            identityIndex: Int,
            amountDuffs: Long,
            status: Byte,
            proofBytes: ByteArray?,
        ): Int = 0

        override fun onLoadAssetLockOperationId(walletId: ByteArray, outPoint: ByteArray): ByteArray? =
            null
    }

    class WithoutLoad : OperationIdBridge() {
        @Suppress("LongParameterList")
        override fun onPersistAssetLockUpsert(
            walletId: ByteArray,
            outPoint: ByteArray,
            transactionBytes: ByteArray,
            accountIndex: Int,
            fundingType: Byte,
            identityIndex: Int,
            amountDuffs: Long,
            status: Byte,
            proofBytes: ByteArray?,
        ): Int = 0

        override fun onPersistAssetLockOperationId(
            walletId: ByteArray,
            outPoint: ByteArray,
            operationId: ByteArray,
        ): Int = 0
    }

    class WithoutUpsert : OperationIdBridge() {
        override fun onPersistAssetLockOperationId(
            walletId: ByteArray,
            outPoint: ByteArray,
            operationId: ByteArray,
        ): Int = 0

        override fun onLoadAssetLockOperationId(walletId: ByteArray, outPoint: ByteArray): ByteArray? =
            null
    }
}

/**
 * Bridges declaring the private contact request scan state with some of the
 * overrides it depends on, for
 * [FfiSmokeTest.privateContactScanStateCapabilityNeedsEveryOverride]. Each
 * concrete class declares exactly the overrides its name says.
 */
private abstract class ScanStateBridge : NativePersistenceBridge() {
    override fun persistenceCapabilitiesVersion(): Int = 1

    override fun persistenceCapabilitiesBits(): Long =
        CAPABILITY_PRIVATE_CONTACT_REQUEST_SCAN_STATE or ATOMIC_CHANGESETS

    open class Full : ScanStateBridge() {
        @Suppress("LongParameterList")
        override fun onPersistContactUpsert(
            walletId: ByteArray,
            ownerId: ByteArray,
            contactId: ByteArray,
            isOutgoing: Boolean,
            senderKeyIndex: Int,
            recipientKeyIndex: Int,
            accountReference: Int,
            encryptedPublicKey: ByteArray,
            encryptedAccountLabel: ByteArray?,
            autoAcceptProof: ByteArray?,
            coreHeightCreatedAt: Int,
            createdAt: Long,
            paymentChannelBroken: Boolean,
            alias: String?,
            note: String?,
            isHidden: Boolean,
            contactAccountLabel: String?,
            acceptedAccounts: IntArray,
            hasExternalAccountReference: Boolean,
            externalAccountReference: Int,
        ): Int = 0

        override fun onPersistPrivateContactScanState(
            walletId: ByteArray,
            positions: ByteArray,
            addresses: ByteArray,
            removedAddresses: ByteArray,
            clearedIdentities: ByteArray,
        ): Int = 0

        override fun onLoadPrivateContactScanState(walletId: ByteArray): Array<ByteArray> =
            arrayOf(ByteArray(0), ByteArray(0))

        override fun onLoadPrivateContactScanStateWallets(): ByteArray = ByteArray(0)
    }

    class WithoutContactUpsert : ScanStateBridge() {
        override fun onPersistPrivateContactScanState(
            walletId: ByteArray,
            positions: ByteArray,
            addresses: ByteArray,
            removedAddresses: ByteArray,
            clearedIdentities: ByteArray,
        ): Int = 0

        override fun onLoadPrivateContactScanState(walletId: ByteArray): Array<ByteArray> =
            arrayOf(ByteArray(0), ByteArray(0))

        override fun onLoadPrivateContactScanStateWallets(): ByteArray = ByteArray(0)
    }

    class WithoutScanStateStore : ScanStateBridge() {
        @Suppress("LongParameterList")
        override fun onPersistContactUpsert(
            walletId: ByteArray,
            ownerId: ByteArray,
            contactId: ByteArray,
            isOutgoing: Boolean,
            senderKeyIndex: Int,
            recipientKeyIndex: Int,
            accountReference: Int,
            encryptedPublicKey: ByteArray,
            encryptedAccountLabel: ByteArray?,
            autoAcceptProof: ByteArray?,
            coreHeightCreatedAt: Int,
            createdAt: Long,
            paymentChannelBroken: Boolean,
            alias: String?,
            note: String?,
            isHidden: Boolean,
            contactAccountLabel: String?,
            acceptedAccounts: IntArray,
            hasExternalAccountReference: Boolean,
            externalAccountReference: Int,
        ): Int = 0

        override fun onLoadPrivateContactScanState(walletId: ByteArray): Array<ByteArray> =
            arrayOf(ByteArray(0), ByteArray(0))

        override fun onLoadPrivateContactScanStateWallets(): ByteArray = ByteArray(0)
    }

    class WithoutScanStateLoad : ScanStateBridge() {
        @Suppress("LongParameterList")
        override fun onPersistContactUpsert(
            walletId: ByteArray,
            ownerId: ByteArray,
            contactId: ByteArray,
            isOutgoing: Boolean,
            senderKeyIndex: Int,
            recipientKeyIndex: Int,
            accountReference: Int,
            encryptedPublicKey: ByteArray,
            encryptedAccountLabel: ByteArray?,
            autoAcceptProof: ByteArray?,
            coreHeightCreatedAt: Int,
            createdAt: Long,
            paymentChannelBroken: Boolean,
            alias: String?,
            note: String?,
            isHidden: Boolean,
            contactAccountLabel: String?,
            acceptedAccounts: IntArray,
            hasExternalAccountReference: Boolean,
            externalAccountReference: Int,
        ): Int = 0

        override fun onPersistPrivateContactScanState(
            walletId: ByteArray,
            positions: ByteArray,
            addresses: ByteArray,
            removedAddresses: ByteArray,
            clearedIdentities: ByteArray,
        ): Int = 0

        override fun onLoadPrivateContactScanStateWallets(): ByteArray = ByteArray(0)
    }

    class WithoutScanStateWalletList : ScanStateBridge() {
        @Suppress("LongParameterList")
        override fun onPersistContactUpsert(
            walletId: ByteArray,
            ownerId: ByteArray,
            contactId: ByteArray,
            isOutgoing: Boolean,
            senderKeyIndex: Int,
            recipientKeyIndex: Int,
            accountReference: Int,
            encryptedPublicKey: ByteArray,
            encryptedAccountLabel: ByteArray?,
            autoAcceptProof: ByteArray?,
            coreHeightCreatedAt: Int,
            createdAt: Long,
            paymentChannelBroken: Boolean,
            alias: String?,
            note: String?,
            isHidden: Boolean,
            contactAccountLabel: String?,
            acceptedAccounts: IntArray,
            hasExternalAccountReference: Boolean,
            externalAccountReference: Int,
        ): Int = 0

        override fun onPersistPrivateContactScanState(
            walletId: ByteArray,
            positions: ByteArray,
            addresses: ByteArray,
            removedAddresses: ByteArray,
            clearedIdentities: ByteArray,
        ): Int = 0

        override fun onLoadPrivateContactScanState(walletId: ByteArray): Array<ByteArray> =
            arrayOf(ByteArray(0), ByteArray(0))
    }
}

/** `PersistenceCapabilities::ATOMIC_CHANGESETS` (bit 0). */
private const val ATOMIC_CHANGESETS: Long = 1L
