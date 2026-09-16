package org.dashfoundation.dashsdk.persistence

import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * In-memory Room contract tests for the three transaction-label resolver
 * queries: [org.dashfoundation.dashsdk.persistence.dao.TransactionDao]'s
 * `transactionKindForTxid` / `transactionKindForDisplayTxid` and
 * [org.dashfoundation.dashsdk.persistence.dao.AssetLockDao]'s
 * `fundingTypeForTxid`.
 *
 * The fixture txid is deliberately NON-palindromic (`0x01..0x20`), so the
 * display↔wire byte reversal in `displayHexToWireTxid` is pinned: deleting
 * (or double-applying) the reversal loop flips these assertions, which a
 * symmetric fixture like `"ab" * 32` would not catch.
 */
@RunWith(RobolectricTestRunner::class)
class TransactionLabelResolverDaoTest {

    private lateinit var db: DashDatabase

    private val walletId = ByteArray(32) { 1 }

    /** Raw little-endian WIRE txid — the `transactions.txid` PK form. */
    private val wireTxid = ByteArray(32) { (it + 1).toByte() }

    /** Explorer DISPLAY hex: wire bytes reversed, lowercase. */
    private val displayHex = hex(wireTxid.reversedArray())

    /** The unreversed hex of the wire bytes — display form of a DIFFERENT tx. */
    private val wireHexUnreversed = hex(wireTxid)

    /** `TransactionType` discriminant for AssetUnlock (withdraw/unshield). */
    private val kindAssetUnlock = 7

    /** `AssetLockFundingType` discriminant for AssetLockAddressTopUp. */
    private val fundingTypeAddressTopUp = 4

    @Before
    fun setUp() {
        db = DashDatabase.createInMemory(ApplicationProvider.getApplicationContext())
    }

    @After
    fun tearDown() {
        db.close()
    }

    private fun hex(bytes: ByteArray): String =
        bytes.joinToString("") { "%02x".format(it) }

    private suspend fun insertTransaction() {
        db.transactionDao().upsert(
            TransactionEntity(
                txid = wireTxid,
                transactionData = ByteArray(4),
                transactionTypeKind = kindAssetUnlock,
            ),
        )
    }

    private suspend fun insertAssetLock(outPointHex: String, fundingTypeRaw: Int) {
        db.assetLockDao().upsert(
            AssetLockEntity(
                outPointHex = outPointHex,
                walletId = walletId,
                transactionBytes = ByteArray(4),
                fundingTypeRaw = fundingTypeRaw,
                identityIndexRaw = 0,
                amountDuffs = 10_000,
                statusRaw = 1,
            ),
        )
    }

    // ── transactionKindForTxid (wire BLOB PK) ─────────────────────────

    @Test
    fun transactionKindForTxidMatchesWireBytes() = runTest {
        insertTransaction()
        assertEquals(kindAssetUnlock, db.transactionDao().transactionKindForTxid(wireTxid))
    }

    @Test
    fun transactionKindForTxidReturnsNullForUnknownTxid() = runTest {
        insertTransaction()
        assertNull(db.transactionDao().transactionKindForTxid(ByteArray(32) { 9 }))
    }

    // ── transactionKindForDisplayTxid (display hex → wire reversal) ───

    @Test
    fun displayTxidLookupReversesToWireOrder() = runTest {
        insertTransaction()
        // Display hex resolves the row — pins the byte reversal on a
        // non-palindromic txid.
        assertEquals(
            kindAssetUnlock,
            db.transactionDao().transactionKindForDisplayTxid(displayHex),
        )
        // The UNREVERSED wire hex must miss: it denotes a different
        // (reversed) txid. If the reversal loop were deleted, this lookup
        // would incorrectly succeed and the one above would fail.
        assertNull(db.transactionDao().transactionKindForDisplayTxid(wireHexUnreversed))
    }

    @Test
    fun displayTxidLookupAcceptsUppercaseHex() = runTest {
        insertTransaction()
        assertEquals(
            kindAssetUnlock,
            db.transactionDao().transactionKindForDisplayTxid(displayHex.uppercase()),
        )
    }

    @Test
    fun displayTxidLookupRejectsMalformedInput() = runTest {
        insertTransaction()
        assertNull(db.transactionDao().transactionKindForDisplayTxid(""))
        assertNull(db.transactionDao().transactionKindForDisplayTxid(displayHex.dropLast(1)))
        assertNull(db.transactionDao().transactionKindForDisplayTxid(displayHex + "00"))
        assertNull(
            db.transactionDao()
                .transactionKindForDisplayTxid("z" + displayHex.drop(1)),
        )
    }

    // ── fundingTypeForTxid (outPointHex prefix) ───────────────────────

    @Test
    fun fundingTypeForTxidMatchesExactOutpointPrefix() = runTest {
        insertAssetLock("$displayHex:0", fundingTypeAddressTopUp)
        assertEquals(
            fundingTypeAddressTopUp,
            db.assetLockDao().fundingTypeForTxid(displayHex),
        )
        // Uppercase canonical txids are accepted (`lower()` in the query).
        assertEquals(
            fundingTypeAddressTopUp,
            db.assetLockDao().fundingTypeForTxid(displayHex.uppercase()),
        )
        // A different txid misses even with rows present.
        assertNull(db.assetLockDao().fundingTypeForTxid(wireHexUnreversed))
    }

    @Test
    fun fundingTypeForTxidRejectsWildcardAndMalformedInput() = runTest {
        insertAssetLock("$displayHex:0", fundingTypeAddressTopUp)
        // SQLite LIKE would have treated these as wildcards; the exact
        // substr comparison + hex GLOB guard must return null instead of
        // an arbitrary row's funding type.
        assertNull(db.assetLockDao().fundingTypeForTxid("%"))
        assertNull(db.assetLockDao().fundingTypeForTxid("_".repeat(64)))
        assertNull(db.assetLockDao().fundingTypeForTxid("%".repeat(64)))
        assertNull(db.assetLockDao().fundingTypeForTxid(displayHex.dropLast(1)))
        assertNull(db.assetLockDao().fundingTypeForTxid(""))
    }

    @Test
    fun fundingTypeForTxidIsStableAcrossMultipleVoutsOfOneTx() = runTest {
        // DIP-0027 permits several asset-lock outputs per tx; rows from the
        // same funding flow share fundingTypeRaw, so LIMIT 1 stays
        // value-stable regardless of which row the scan picks.
        insertAssetLock("$displayHex:0", fundingTypeAddressTopUp)
        insertAssetLock("$displayHex:1", fundingTypeAddressTopUp)
        // An unrelated lock with a different funding type must not bleed in.
        insertAssetLock("${hex(ByteArray(32) { 9 }.reversedArray())}:0", 5)
        assertEquals(
            fundingTypeAddressTopUp,
            db.assetLockDao().fundingTypeForTxid(displayHex),
        )
    }
}
