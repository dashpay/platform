package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.flow.first
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * In-memory Room contract tests for the resumable asset-lock predicates on
 * [org.dashfoundation.dashsdk.persistence.dao.AssetLockDao].
 *
 * The behavior under test is the status domain. Rust's `AssetLockStatus`
 * is `0 Built, 1 Broadcast, 2 InstantSendLocked, 3 ChainLocked,
 * 4 Consumed, 5 RecoveredFromChain`, and the recoverable set is NOT a
 * contiguous range: `4` is the terminal tombstone that must stay hidden,
 * while `5` — written by the restore scan and the chainlock-promotion path
 * for a lock with proven Core finality and unknown Platform-side
 * consumption — must be visible.
 *
 * Expressing that as `statusRaw >= 1 AND statusRaw <= 3` dropped every
 * recovered row, so a chain-locked top-up the user really funded appeared
 * on no host surface at all. These tests pin both ends: `5` in, `4` out.
 */
@RunWith(RobolectricTestRunner::class)
class AssetLockResumableDaoTest {

    private lateinit var db: DashDatabase

    private val walletId = ByteArray(32) { 1 }
    private val otherWalletId = ByteArray(32) { 2 }

    /** `AssetLockFundingType` discriminants. */
    private val fundingAddressTopUp = 4
    private val fundingShieldedTopUp = 5
    private val fundingIdentityRegistration = 0

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        db.close()
    }

    private suspend fun insert(
        outPointHex: String,
        statusRaw: Int,
        fundingTypeRaw: Int = fundingAddressTopUp,
        owner: ByteArray = walletId,
    ) {
        db.assetLockDao().upsert(
            AssetLockEntity(
                outPointHex = outPointHex,
                walletId = owner,
                transactionBytes = ByteArray(4),
                fundingTypeRaw = fundingTypeRaw,
                identityIndexRaw = 0,
                amountDuffs = 10_000,
                statusRaw = statusRaw,
            ),
        )
    }

    private suspend fun resumableAddressOutpoints(): List<String> =
        db.assetLockDao().observeResumableAddressTopUps(walletId).first()
            .map { it.outPointHex }
            .sorted()

    // ── observeResumableAddressTopUps ─────────────────────────────────

    /**
     * The regression itself: status 5 must be returned. Before the fix the
     * upper bound of `3` hid it, and the funds it represents were
     * unreachable from the UI.
     */
    @Test
    fun resumableAddressTopUpsIncludeRecoveredFromChain() = runTest {
        insert("aa:0", statusRaw = 5)

        assertEquals(listOf("aa:0"), resumableAddressOutpoints())
    }

    /** Every recoverable status, and only those. */
    @Test
    fun resumableAddressTopUpsCoverTheWholeRecoverableDomain() = runTest {
        insert("built:0", statusRaw = 0)
        insert("broadcast:0", statusRaw = 1)
        insert("islocked:0", statusRaw = 2)
        insert("chainlocked:0", statusRaw = 3)
        insert("consumed:0", statusRaw = 4)
        insert("recovered:0", statusRaw = 5)

        assertEquals(
            listOf("broadcast:0", "chainlocked:0", "islocked:0", "recovered:0"),
            resumableAddressOutpoints(),
        )
    }

    /**
     * The terminal guard from #4347 must survive the widening. A Consumed
     * row that re-surfaced would be a perpetual-spinner the underlying
     * `resume_asset_lock` rejects with "already Consumed — nothing to
     * resume".
     */
    @Test
    fun resumableAddressTopUpsStillExcludeConsumed() = runTest {
        insert("consumed:0", statusRaw = 4)

        assertEquals(emptyList<String>(), resumableAddressOutpoints())
    }

    /** Built (0) stays out: nothing has been broadcast yet. */
    @Test
    fun resumableAddressTopUpsStillExcludeBuilt() = runTest {
        insert("built:0", statusRaw = 0)

        assertEquals(emptyList<String>(), resumableAddressOutpoints())
    }

    /** The funding-type and wallet scoping are unchanged by the widening. */
    @Test
    fun resumableAddressTopUpsStayScopedToFundingTypeFourAndWallet() = runTest {
        insert("identity:0", statusRaw = 5, fundingTypeRaw = fundingIdentityRegistration)
        insert("shielded:0", statusRaw = 5, fundingTypeRaw = fundingShieldedTopUp)
        insert("foreign:0", statusRaw = 5, owner = otherWalletId)
        insert("mine:0", statusRaw = 5)

        assertEquals(listOf("mine:0"), resumableAddressOutpoints())
    }

    // ── observeResumableTopUpsByFundingType ───────────────────────────

    /**
     * Shielded address top-ups (funding type 5) previously had no resumable
     * query at all — the address query is pinned to 4, and the
     * identity-recovery surface (`TrackedAssetLock.eligibleFromNative`)
     * deliberately admits only funding types 0..2 — so a stalled shielded
     * top-up was invisible everywhere.
     */
    @Test
    fun resumableByFundingTypeCoversShieldedTopUps() = runTest {
        insert("shielded-broadcast:0", statusRaw = 1, fundingTypeRaw = fundingShieldedTopUp)
        insert("shielded-recovered:0", statusRaw = 5, fundingTypeRaw = fundingShieldedTopUp)
        insert("shielded-consumed:0", statusRaw = 4, fundingTypeRaw = fundingShieldedTopUp)
        insert("shielded-built:0", statusRaw = 0, fundingTypeRaw = fundingShieldedTopUp)
        insert("address-recovered:0", statusRaw = 5, fundingTypeRaw = fundingAddressTopUp)

        val shielded = db.assetLockDao()
            .observeResumableTopUpsByFundingType(walletId, fundingShieldedTopUp)
            .first()
            .map { it.outPointHex }
            .sorted()

        assertEquals(listOf("shielded-broadcast:0", "shielded-recovered:0"), shielded)
    }

    /** Parameterized with 4, it agrees with the dedicated address query. */
    @Test
    fun resumableByFundingTypeAgreesWithTheAddressQuery() = runTest {
        insert("a:0", statusRaw = 1)
        insert("b:0", statusRaw = 5)
        insert("c:0", statusRaw = 4)
        insert("d:0", statusRaw = 0)

        val parameterized = db.assetLockDao()
            .observeResumableTopUpsByFundingType(walletId, fundingAddressTopUp)
            .first()
            .map { it.outPointHex }
            .sorted()

        assertEquals(resumableAddressOutpoints(), parameterized)
        assertEquals(listOf("a:0", "b:0"), parameterized)
    }
}
