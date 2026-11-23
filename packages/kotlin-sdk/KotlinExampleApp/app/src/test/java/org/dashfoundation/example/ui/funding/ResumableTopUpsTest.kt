package org.dashfoundation.example.ui.funding

import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.flow.flowOf
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.wallet.WalletDrainLockListing
import org.dashfoundation.dashsdk.wallet.toWalletDrainOperationIdBytes
import org.dashfoundation.example.navigation.FundFromAssetLock
import org.dashfoundation.example.navigation.ShieldedFund
import org.dashfoundation.example.util.toHex
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.util.UUID

/**
 * Caller-side tests for the "Pending Platform Top Ups" recovery surface.
 *
 * `AssetLockResumableDaoTest` (in `:sdk`) already pins the DAO query itself —
 * that `observeResumableTopUpsByFundingType(walletId, 5)` returns the
 * `[1,3] ∪ {5}` status set. What that cannot show is whether any production
 * screen ASKS for funding type 5. It didn't: the only caller ran
 * `observeResumableAddressTopUps`, which hardcodes `fundingTypeRaw = 4`, so a
 * stalled or RecoveredFromChain shielded top-up was absent from every host
 * surface no matter how correct the query was.
 *
 * These tests pin the two halves of that wiring:
 *
 *   1. [resumableTopUpsAcrossWallets] fans out over BOTH top-up funding
 *      types, for every loaded wallet.
 *   2. [resumeRouteFor] sends each row to the resume flow that matches its
 *      funding type — a shielded lock must not land on the platform-address
 *      screen, whose submit calls the wrong FFI.
 */
class ResumableTopUpsTest {

    private val walletA = ByteArray(32) { 1 }
    private val walletB = ByteArray(32) { 2 }
    private val walletAHex = walletA.toHex()
    private val walletBHex = walletB.toHex()

    private fun lock(
        outPointHex: String,
        fundingTypeRaw: Int,
        statusRaw: Int = 5,
        operationId: ByteArray? = null,
    ): AssetLockEntity = AssetLockEntity(
        outPointHex = outPointHex,
        walletId = ByteArray(32),
        transactionBytes = ByteArray(0),
        fundingTypeRaw = fundingTypeRaw,
        identityIndexRaw = 0,
        amountDuffs = 100_000L,
        statusRaw = statusRaw,
        operationId = operationId,
    )

    /**
     * Rust's answer for a lock no wallet drain is concerned with. The real
     * rule is native (`WalletDrainRules.lockListingOrNull`), which a JVM test
     * cannot load.
     */
    private val notADrain: DrainLockListing = { _, _, _, _ -> WalletDrainLockListing.Unaffected }

    /** The table of `wallet_drain_lock_listing`, restated for the JVM. */
    private val rustListing: DrainLockListing = { fundingTypeRaw, statusRaw, hasId, storedId ->
        when {
            !hasId -> WalletDrainLockListing.Unaffected
            storedId -> WalletDrainLockListing.Hidden
            fundingTypeRaw == 5 && statusRaw == 0 -> WalletDrainLockListing.Offered
            else -> WalletDrainLockListing.Unaffected
        }
    }

    /**
     * Records every `(walletIdHex, fundingTypeRaw)` the caller asks for, and
     * serves whatever rows the test registered for that pair.
     */
    private class RecordingObserver(
        private val rows: Map<Pair<String, Int>, List<AssetLockEntity>> = emptyMap(),
    ) {
        val requested = mutableListOf<Pair<String, Int>>()

        fun observe(walletId: ByteArray, fundingTypeRaw: Int): Flow<List<AssetLockEntity>> {
            val key = walletId.toHex() to fundingTypeRaw
            requested += key
            return flowOf(rows[key] ?: emptyList())
        }
    }

    // ── funding-type fan-out ──────────────────────────────────────────

    /**
     * The blocker itself. Funding type 5 must be among the types the screen
     * observes — otherwise the parameterized DAO query has no production
     * caller and the shielded rows it can return are never requested.
     */
    @Test
    fun `observes both address and shielded top-up funding types`() = runTest {
        val observer = RecordingObserver()

        resumableTopUpsAcrossWallets(listOf(walletAHex), observer::observe, listing = notADrain).first()

        assertEquals(
            listOf(walletAHex to 4, walletAHex to 5),
            observer.requested,
        )
    }

    /**
     * The user-visible consequence: a shielded (`fundingTypeRaw == 5`)
     * RecoveredFromChain lock reaches the rendered list, tagged with its
     * owning wallet so Resume can navigate wallet-scoped.
     */
    @Test
    fun `surfaces a shielded RecoveredFromChain top-up`() = runTest {
        val shielded = lock("shielded:0", fundingTypeRaw = 5, statusRaw = 5)
        val observer = RecordingObserver(mapOf((walletAHex to 5) to listOf(shielded)))

        val rows = resumableTopUpsAcrossWallets(listOf(walletAHex), observer::observe, listing = notADrain).first()

        assertEquals(listOf(walletAHex to shielded), rows)
    }

    /** Address rows are unaffected by the widening. */
    @Test
    fun `still surfaces address top-ups alongside shielded ones`() = runTest {
        val address = lock("address:0", fundingTypeRaw = 4, statusRaw = 1)
        val shielded = lock("shielded:0", fundingTypeRaw = 5, statusRaw = 5)
        val observer = RecordingObserver(
            mapOf(
                (walletAHex to 4) to listOf(address),
                (walletAHex to 5) to listOf(shielded),
            ),
        )

        val rows = resumableTopUpsAcrossWallets(listOf(walletAHex), observer::observe, listing = notADrain).first()

        assertEquals(listOf(walletAHex to address, walletAHex to shielded), rows)
    }

    /**
     * Each wallet is observed on both funding types, and every row keeps the
     * hex of the wallet that owns it — the Identities tab is cross-wallet,
     * so mis-tagging would send Resume to the wrong wallet's screen.
     */
    @Test
    fun `tags rows with their owning wallet across wallets`() = runTest {
        val aLock = lock("a:0", fundingTypeRaw = 4, statusRaw = 2)
        val bLock = lock("b:0", fundingTypeRaw = 5, statusRaw = 5)
        val observer = RecordingObserver(
            mapOf(
                (walletAHex to 4) to listOf(aLock),
                (walletBHex to 5) to listOf(bLock),
            ),
        )

        val rows = resumableTopUpsAcrossWallets(
            listOf(walletAHex, walletBHex),
            observer::observe,
            listing = notADrain,
        ).first()

        assertEquals(listOf(walletAHex to aLock, walletBHex to bLock), rows)
        assertEquals(
            listOf(walletAHex to 4, walletAHex to 5, walletBHex to 4, walletBHex to 5),
            observer.requested,
        )
    }

    /**
     * With no wallets loaded the flow must still emit — `combine` over an
     * empty source list never emits, which would leave the section pinned to
     * its initial value instead of resolving to "nothing to recover".
     */
    @Test
    fun `emits an empty list when no wallets are loaded`() = runTest {
        val observer = RecordingObserver()

        val rows = resumableTopUpsAcrossWallets(emptyList(), observer::observe, listing = notADrain).first()

        assertEquals(emptyList<Pair<String, AssetLockEntity>>(), rows)
        assertEquals(emptyList<Pair<String, Int>>(), observer.requested)
    }

    // ── wallet drain ("Max") locks ────────────────────────────────────

    private val drainId = UUID.fromString("00112233-4455-6677-8899-aabbccddeeff")
    private val drainIdBytes = drainId.toWalletDrainOperationIdBytes()

    /**
     * A drain's lock that a stored operation id names is shown by the
     * "Saved Max transfers" row alone, at every status.
     */
    @Test
    fun `hides a drain lock the app holds a stored id for`() {
        for (status in listOf(0, 1, 2, 3, 5)) {
            assertFalse(
                "status $status",
                lock("d:0", fundingTypeRaw = 5, statusRaw = status, operationId = drainIdBytes)
                    .isOfferedForResume(setOf(drainId), rustListing),
            )
        }
    }

    /**
     * A drain's Built checkpoint with no stored id is reachable only by an
     * exact-outpoint resume, so it is offered although an ordinary Built
     * lock (a build still in flight) is not.
     */
    @Test
    fun `offers a drain's Built checkpoint that no stored id names`() {
        val checkpoint = lock("d:0", fundingTypeRaw = 5, statusRaw = 0, operationId = drainIdBytes)
        assertTrue(checkpoint.isOfferedForResume(emptySet(), rustListing))
        assertTrue(
            "a different stored id does not claim it",
            checkpoint.isOfferedForResume(setOf(UUID(1, 2)), rustListing),
        )
        assertFalse(
            "an ordinary Built lock stays hidden",
            lock("b:0", fundingTypeRaw = 5, statusRaw = 0).isOfferedForResume(emptySet(), rustListing),
        )
        assertTrue(
            "a later status follows the surface's own rule",
            lock("d:1", fundingTypeRaw = 5, statusRaw = 1, operationId = drainIdBytes)
                .isOfferedForResume(emptySet(), rustListing),
        )
        assertFalse(
            "a finished drain's receipt is never resumable",
            lock("d:2", fundingTypeRaw = 5, statusRaw = 6, operationId = drainIdBytes)
                .isOfferedForResume(emptySet(), rustListing),
        )
    }

    /** The facts handed to Rust's rule: the row's raw values and whether a stored id names it. */
    @Test
    fun `asks the rule with the row's values and whether a stored id names it`() {
        val asked = mutableListOf<List<Any>>()
        val recording: DrainLockListing = { fundingTypeRaw, statusRaw, hasId, storedId ->
            asked += listOf(fundingTypeRaw, statusRaw, hasId, storedId)
            WalletDrainLockListing.Unaffected
        }
        lock("a:0", fundingTypeRaw = 4, statusRaw = 2).isOfferedForResume(setOf(drainId), recording)
        lock("d:0", fundingTypeRaw = 5, statusRaw = 1, operationId = drainIdBytes)
            .isOfferedForResume(setOf(drainId), recording)
        lock("d:1", fundingTypeRaw = 5, statusRaw = 1, operationId = drainIdBytes)
            .isOfferedForResume(emptySet(), recording)
        // An id that is not 16 bytes is not an operation id at all, as on
        // iOS: the row is not a drain's lock, and it names no stored one.
        lock("d:2", fundingTypeRaw = 5, statusRaw = 1, operationId = ByteArray(3))
            .isOfferedForResume(setOf(drainId), recording)

        assertEquals(
            listOf(
                listOf(4, 2, false, false),
                listOf(5, 1, true, true),
                listOf(5, 1, true, false),
                listOf(5, 1, false, false),
            ),
            asked,
        )
    }

    /** A malformed stored id cannot turn a built lock into a drain's checkpoint. */
    @Test
    fun `a malformed operation id is not offered as a drain checkpoint`() {
        assertFalse(
            lock("d:0", fundingTypeRaw = 5, statusRaw = 0, operationId = ByteArray(3))
                .isOfferedForResume(emptySet(), rustListing),
        )
        assertTrue(
            "the same row with a valid id is the checkpoint",
            lock("d:1", fundingTypeRaw = 5, statusRaw = 0, operationId = drainIdBytes)
                .isOfferedForResume(emptySet(), rustListing),
        )
    }

    /** A rule that cannot be asked offers nothing rather than guessing. */
    @Test
    fun `offers nothing when the rule cannot be asked`() {
        val broken: DrainLockListing = { _, _, _, _ -> error("native library missing") }
        assertFalse(lock("a:0", fundingTypeRaw = 4, statusRaw = 2).isOfferedForResume(emptySet(), broken))
        // What `WalletDrainRules.lockListingOrNull` returns for a funding type
        // or status Rust does not know, and for a failed call.
        val unknown: DrainLockListing = { _, _, _, _ -> null }
        assertFalse(lock("a:0", fundingTypeRaw = 4, statusRaw = 2).isOfferedForResume(emptySet(), unknown))
    }

    /** The list re-filters when a stored id appears or goes. */
    @Test
    fun `the list follows the stored ids`() = runTest {
        val drainLock = lock("d:0", fundingTypeRaw = 5, statusRaw = 1, operationId = drainIdBytes)
        val observer = RecordingObserver(mapOf((walletAHex to 5) to listOf(drainLock)))

        assertEquals(
            emptyList<Pair<String, AssetLockEntity>>(),
            resumableTopUpsAcrossWallets(
                listOf(walletAHex), observer::observe, flowOf(setOf(drainId)), rustListing,
            ).first(),
        )
        assertEquals(
            listOf(walletAHex to drainLock),
            resumableTopUpsAcrossWallets(
                listOf(walletAHex), observer::observe, flowOf(emptySet()), rustListing,
            ).first(),
        )
    }

    // ── resume routing ────────────────────────────────────────────────

    /**
     * The second half of the blocker: surfacing a shielded row is useless if
     * its Resume opens the platform-address screen, whose submit calls
     * `resumeFundFromAssetLock` rather than
     * `shieldedResumeFundFromAssetLock`.
     */
    @Test
    fun `routes a shielded lock to the shielded resume screen`() {
        val shielded = lock("shielded:0", fundingTypeRaw = 5)

        assertEquals(
            ShieldedFund(walletIdHex = walletAHex, resumeOutPointHex = "shielded:0"),
            resumeRouteFor(walletAHex, shielded),
        )
    }

    @Test
    fun `routes an address lock to the platform-address resume screen`() {
        val address = lock("address:0", fundingTypeRaw = 4)

        assertEquals(
            FundFromAssetLock(walletIdHex = walletAHex, resumeOutPointHex = "address:0"),
            resumeRouteFor(walletAHex, address),
        )
    }

    /**
     * Fail closed. Identity-family types (0..3) recover on the identity
     * screens; an unknown discriminant must not be dispatched into a resume
     * flow that would submit the wrong transition for it.
     */
    @Test
    fun `refuses to route funding types that have no resume flow here`() {
        for (fundingType in listOf(0, 1, 2, 3, 6, 99)) {
            assertNull(
                "fundingTypeRaw $fundingType must not route to a top-up resume screen",
                resumeRouteFor(walletAHex, lock("x:0", fundingTypeRaw = fundingType)),
            )
        }
    }
}
