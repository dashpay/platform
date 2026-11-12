package org.dashfoundation.dashsdk

import androidx.test.core.app.ApplicationProvider
import androidx.test.ext.junit.runners.AndroidJUnit4
import kotlinx.coroutines.Dispatchers
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.DashpayNative
import org.dashfoundation.dashsdk.ffi.NativePersistenceBridge
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
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

    @Test
    fun nativeLibraryLoadsAndReportsVersion() {
        NativeLoader.ensureLoaded()
        val version = SdkNative.version()
        assertTrue("version should be non-empty", version.isNotEmpty())
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
