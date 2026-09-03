package org.dashfoundation.dashsdk.persistence

import androidx.room.testing.MigrationTestHelper
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Migration tests over the exported schemas (`sdk/schemas`, wired as
 * androidTest assets). Each test creates the database at the FROM
 * version, seeds representative rows, runs the migration under test, and
 * lets [MigrationTestHelper] validate the migrated schema byte-for-byte
 * against the exported TO-version JSON — catching any drift between the
 * hand-written SQL in [DashDatabase] and what Room generates.
 */
@RunWith(AndroidJUnit4::class)
class DashDatabaseMigrationTest {

    @get:Rule
    val helper = MigrationTestHelper(
        InstrumentationRegistry.getInstrumentation(),
        DashDatabase::class.java,
    )

    private val dbName = "migration-test.db"

    /**
     * v2 → v3 adds the `dashpay_contact_profiles` and `dashpay_payments`
     * tables (additive — no reshapes). Pre-existing v2 data must survive
     * untouched and the new tables must accept rows keyed by their
     * compound PKs.
     */
    @Test
    fun migrate2To3AddsContactProfileAndPaymentTables() {
        helper.createDatabase(dbName, 2).apply {
            // Seed a v2 wallet + identity so FK targets exist post-migration.
            execSQL(
                "INSERT INTO wallets (walletId, walletGroupId, networkRaw, name, birthHeight, " +
                    "syncedHeight, lastSynced, isImported, createdAt, lastUpdated) " +
                    "VALUES (x'01', x'02', 1, 'w', 0, 0, 0, 0, 0, 0)",
            )
            execSQL(
                "INSERT INTO identities (identityId, balance, revision, isLocal, identityType, " +
                    "createdAt, lastUpdated, networkRaw, identityIndex) " +
                    "VALUES (x'0A', 0, 0, 1, 'User', 0, 0, 1, 0)",
            )
            close()
        }

        val db = helper.runMigrationsAndValidate(dbName, 3, true, DashDatabase.MIGRATION_2_3)

        // Pre-existing rows survived.
        db.query("SELECT COUNT(*) FROM identities").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(1, c.getInt(0))
        }
        // The new tables accept rows under their compound keys.
        db.execSQL(
            "INSERT INTO dashpay_contact_profiles (networkRaw, ownerIdentityId, " +
                "contactIdentityId, checkedAtMs, createdAt, lastUpdated) " +
                "VALUES (1, x'0A', x'0B', 123, 0, 0)",
        )
        db.execSQL(
            "INSERT INTO dashpay_payments (networkRaw, ownerIdentityId, " +
                "counterpartyIdentityId, amountDuffs, directionRaw, statusRaw, txid, " +
                "createdAt, lastUpdated) VALUES (1, x'0A', x'0B', 5, 0, 1, 'ab', 0, 0)",
        )
        db.query("SELECT COUNT(*) FROM dashpay_contact_profiles").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(1, c.getInt(0))
        }
        db.query("SELECT COUNT(*) FROM dashpay_payments").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(1, c.getInt(0))
        }
        db.close()
    }

    /**
     * v3 → v4 rebuilds `platform_addresses` onto the composite
     * `(walletId, address)` primary key with per-wallet `addressHash`
     * uniqueness. Legacy rows must survive the create-copy-drop-rename
     * verbatim, and the reshaped table must accept the previously
     * impossible case: two wallets holding the SAME address/hash (same
     * seed imported twice) — while still rejecting a duplicate within
     * one wallet.
     */
    @Test
    fun migrate3To4ScopesPlatformAddressesByWallet() {
        helper.createDatabase(dbName, 3).apply {
            execSQL(
                "INSERT INTO platform_addresses (address, addressType, addressHash, " +
                    "publicKey, accountIndex, addressIndex, derivationPath, isUsed, " +
                    "balance, nonce, firstSeenHeight, lastSeenHeight, walletId, " +
                    "createdAt, lastUpdated, accountId) " +
                    "VALUES ('dash1a', 0, x'AA', x'', 0, 0, 'm/9h/1h/17h/0h/0h/0', 0, " +
                    "42, 0, 0, 0, x'01', 0, 0, NULL)",
            )
            close()
        }

        val db = helper.runMigrationsAndValidate(dbName, 4, true, DashDatabase.MIGRATION_3_4)

        // The legacy row survived the rebuild with its data intact.
        db.query("SELECT balance FROM platform_addresses WHERE address = 'dash1a'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(42, c.getInt(0))
        }
        // A second wallet may now hold the same address + hash…
        db.execSQL(
            "INSERT INTO platform_addresses (address, addressType, addressHash, " +
                "publicKey, accountIndex, addressIndex, derivationPath, isUsed, " +
                "balance, nonce, firstSeenHeight, lastSeenHeight, walletId, " +
                "createdAt, lastUpdated, accountId) " +
                "VALUES ('dash1a', 0, x'AA', x'', 0, 0, 'm/9h/1h/17h/0h/0h/0', 0, " +
                "7, 0, 0, 0, x'02', 0, 0, NULL)",
        )
        db.query("SELECT COUNT(*) FROM platform_addresses WHERE address = 'dash1a'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(2, c.getInt(0))
        }
        // …but the same wallet still can't hold the address twice.
        try {
            db.execSQL(
                "INSERT INTO platform_addresses (address, addressType, addressHash, " +
                    "publicKey, accountIndex, addressIndex, derivationPath, isUsed, " +
                    "balance, nonce, firstSeenHeight, lastSeenHeight, walletId, " +
                    "createdAt, lastUpdated, accountId) " +
                    "VALUES ('dash1a', 0, x'BB', x'', 0, 0, '', 0, 0, 0, 0, 0, " +
                    "x'01', 0, 0, NULL)",
            )
            org.junit.Assert.fail("expected a (walletId, address) PK violation")
        } catch (_: android.database.sqlite.SQLiteConstraintException) {
            // expected
        }
        db.close()
    }

    /**
     * v4 → v5 converts every signed INTEGER raw-bit pattern into a fixed
     * big-endian BLOB. The conversion must be lossless and BLOB ordering must
     * match unsigned numeric ordering across the sign-bit boundary.
     */
    @Test
    fun migrate4To5PreservesFullUnsignedTokenBalances() {
        helper.createDatabase(dbName, 4).apply {
            listOf(
                "zero" to 0L,
                "signed-max" to Long.MAX_VALUE,
                "high-half" to Long.MIN_VALUE,
                "unsigned-max" to -1L,
            ).forEachIndexed { index, (tokenId, rawBits) ->
                execSQL(
                    "INSERT INTO token_balances (tokenId, identityId, balance, frozen, " +
                        "createdAt, lastUpdated, networkRaw) VALUES (?, ?, ?, 0, 0, 0, 1)",
                    arrayOf(tokenId, byteArrayOf(index.toByte()), rawBits),
                )
            }
            close()
        }

        val db = helper.runMigrationsAndValidate(dbName, 5, true, DashDatabase.MIGRATION_4_5)
        val expected = listOf(
            "zero" to 0uL,
            "signed-max" to Long.MAX_VALUE.toULong(),
            "high-half" to (1uL shl 63),
            "unsigned-max" to ULong.MAX_VALUE,
        )
        db.query("SELECT tokenId, balance, typeof(balance) FROM token_balances ORDER BY balance").use { c ->
            var index = 0
            while (c.moveToNext()) {
                assertEquals(expected[index].first, c.getString(0))
                assertEquals(expected[index].second, UInt64Value.fromBigEndianBytes(c.getBlob(1)).value)
                assertEquals("blob", c.getString(2))
                index += 1
            }
            assertEquals(expected.size, index)
        }
        db.query(
            "SELECT COUNT(*) FROM token_balances " +
                "WHERE balance != X'0000000000000000'",
        ).use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(3, c.getInt(0))
        }
        db.close()
    }

    @Test
    fun migrate5To6AddsOrchardViewingKeys() {
        helper.createDatabase(dbName, 5).close()

        val db = helper.runMigrationsAndValidate(dbName, 6, true, DashDatabase.MIGRATION_5_6)
        db.execSQL(
            "INSERT INTO shielded_viewing_keys " +
                "(walletId, accountIndex, fvkBytes, lastUpdated) VALUES (?, 7, ?, 123)",
            arrayOf(ByteArray(32) { 1 }, ByteArray(96) { 2 }),
        )
        db.query(
            "SELECT accountIndex, length(fvkBytes), lastUpdated " +
                "FROM shielded_viewing_keys",
        ).use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(7, c.getInt(0))
            assertEquals(96, c.getInt(1))
            assertEquals(123L, c.getLong(2))
        }
        db.close()
    }

    @Test
    fun migrate6To7AddsProviderTransactionMembershipAndBlockPosition() {
        helper.createDatabase(dbName, 6).apply {
            execSQL(
                "INSERT INTO transactions (txid, transactionData, context, blockHeight, " +
                    "blockHash, blockTimestamp, direction, transactionType, " +
                    "transactionTypeKind, netAmount, fee, label, firstSeen, createdAt, " +
                    "lastUpdated) VALUES (x'11', x'22', 2, 10, x'33', 20, 0, " +
                    "'Provider', 2, 0, NULL, '', 30, 0, 0)",
            )
            close()
        }

        val db = helper.runMigrationsAndValidate(dbName, 7, true, DashDatabase.MIGRATION_6_7)
        db.query(
            "SELECT blockPosition, hasBlockPosition FROM transactions WHERE txid = x'11'",
        ).use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(0, c.getInt(0))
            assertEquals(0, c.getInt(1))
        }
        // Safe upgrade policy: v6 has no typed account involvement. Do not
        // guess from raw payload bytes or TXOs in SQL. v5/v6 were unreleased;
        // developer databases acquire exact rows when Core resync replays the
        // callback with Rust's enclosing typed account tuple.
        db.query("SELECT COUNT(*) FROM transaction_account_involvements").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(0, c.getInt(0))
        }
        db.execSQL(
            "INSERT INTO wallets (walletId, walletGroupId, networkRaw, name, birthHeight, " +
                "syncedHeight, lastSynced, isImported, createdAt, lastUpdated) " +
                "VALUES (x'01', x'02', 1, 'w', 0, 0, 0, 0, 0, 0)",
        )
        db.execSQL(
            "INSERT INTO accounts (walletId, accountType, accountIndex, accountTypeName, " +
                "balanceConfirmed, balanceUnconfirmed, externalHighestUsed, internalHighestUsed, " +
                "standardTag, registrationIndex, keyClass, userIdentityId, friendIdentityId, " +
                "createdAt, lastUpdated) VALUES (x'01', 9, 7, 'providerOwnerKeys', " +
                "0, 0, -1, -1, 0, 0, 0, x'', x'', 0, 0)",
        )
        db.execSQL(
            "INSERT INTO transaction_account_involvements (transactionTxid, accountId) " +
                "SELECT x'11', id FROM accounts WHERE walletId = x'01'",
        )
        db.query("SELECT COUNT(*) FROM transaction_account_involvements").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(1, c.getInt(0))
        }
        db.close()
    }

    /**
     * v7 → v8 adds the nullable derivation-breadcrumb columns on
     * `public_keys` (dashpay/platform#4060 finding 5). Pre-existing rows
     * must survive with NULL breadcrumbs (unknown — back-filled by the next
     * persist of each key), and new rows must accept explicit values.
     */
    @Test
    fun migrate7To8AddsDerivationBreadcrumbColumns() {
        helper.createDatabase(dbName, 7).apply {
            execSQL(
                "INSERT INTO wallets (walletId, walletGroupId, networkRaw, name, birthHeight, " +
                    "syncedHeight, lastSynced, isImported, createdAt, lastUpdated) " +
                    "VALUES (x'01', x'02', 1, 'w', 0, 0, 0, 0, 0, 0)",
            )
            execSQL(
                "INSERT INTO identities (identityId, balance, revision, isLocal, identityType, " +
                    "createdAt, lastUpdated, networkRaw, identityIndex, walletId) " +
                    "VALUES (x'0A', 0, 0, 1, 'User', 0, 0, 1, 0, x'01')",
            )
            execSQL(
                "INSERT INTO public_keys (keyId, purpose, securityLevel, keyType, readOnly, " +
                    "publicKeyData, identityId, createdAt, identityIdData) " +
                    "VALUES (0, '0', '0', '0', 0, x'02AB', 'id-base58', 0, x'0A')",
            )
            close()
        }

        val db = helper.runMigrationsAndValidate(dbName, 8, true, DashDatabase.MIGRATION_7_8)

        // Pre-existing rows survive with NULL breadcrumbs.
        db.query(
            "SELECT derivationIdentityIndex, derivationKeyIndex FROM public_keys WHERE keyId = 0",
        ).use { c ->
            assertTrue(c.moveToFirst())
            assertTrue(c.isNull(0))
            assertTrue(c.isNull(1))
        }
        // New rows accept explicit breadcrumbs.
        db.execSQL(
            "INSERT INTO public_keys (keyId, purpose, securityLevel, keyType, readOnly, " +
                "publicKeyData, identityId, createdAt, identityIdData, " +
                "derivationIdentityIndex, derivationKeyIndex) " +
                "VALUES (1, '0', '0', '0', 0, x'02CD', 'id-base58', 0, x'0A', 3, 5)",
        )
        db.query(
            "SELECT derivationIdentityIndex, derivationKeyIndex FROM public_keys WHERE keyId = 1",
        ).use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(3, c.getInt(0))
            assertEquals(5, c.getInt(1))
        }
        db.close()
    }

    /**
     * v8 → v9 adds the DIP-13 `invitations` table (additive). The new table
     * must accept a row under its `outPointHex` primary key, reject a
     * duplicate outpoint (upsert-in-place semantics rely on the PK), and
     * default nothing — every column is NOT NULL by schema.
     */
    @Test
    fun migrate8To9AddsInvitationsTable() {
        helper.createDatabase(dbName, 8).close()

        val db = helper.runMigrationsAndValidate(dbName, 9, true, DashDatabase.MIGRATION_8_9)
        db.execSQL(
            "INSERT INTO invitations (outPointHex, rawOutPoint, walletId, " +
                "fundingIndexRaw, amountDuffs, expiryUnix, createdAtSecs, hasInviter, " +
                "statusRaw, reclaimInFlight, createdAt, updatedAt) " +
                "VALUES ('aa:1', x'AB', x'01', 0, 3000000, 10, 5, 1, 0, 0, 0, 0)",
        )
        db.query(
            "SELECT amountDuffs, statusRaw, reclaimInFlight FROM invitations " +
                "WHERE outPointHex = 'aa:1'",
        ).use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(3_000_000L, c.getLong(0))
            assertEquals(0, c.getInt(1))
            assertEquals(0, c.getInt(2))
        }
        try {
            db.execSQL(
                "INSERT INTO invitations (outPointHex, rawOutPoint, walletId, " +
                    "fundingIndexRaw, amountDuffs, expiryUnix, createdAtSecs, hasInviter, " +
                    "statusRaw, reclaimInFlight, createdAt, updatedAt) " +
                    "VALUES ('aa:1', x'AB', x'01', 0, 1, 1, 1, 0, 0, 0, 0, 0)",
            )
            org.junit.Assert.fail("expected an outPointHex PK violation")
        } catch (_: android.database.sqlite.SQLiteConstraintException) {
            // expected
        }
        db.close()
    }

    /** v9 labels remain owned/unlisted and gain nullable marketplace ids. */
    @Test
    fun migrate9To10AddsDpnsMarketplaceState() {
        val legacy = helper.createDatabase(dbName, 9)
        legacy.execSQL(
            "INSERT INTO identities (identityId, networkRaw, balance, revision, identityIndex, " +
                "isLocal, identityType, createdAt, lastUpdated) " +
                "VALUES (x'01', 1, 0, 0, 0, 1, 'user', 0, 0)",
        )
        legacy.execSQL(
            "INSERT INTO dpns_names (networkRaw, label, normalizedLabel, parentDomainName, " +
                "normalizedParentDomainName, acquiredAt, identityId, createdAt, lastUpdated) " +
                "VALUES (1, 'Alice', 'a11ce', 'dash', 'dash', 42, x'01', 0, 0)",
        )
        legacy.close()

        val db = helper.runMigrationsAndValidate(dbName, 10, true, DashDatabase.MIGRATION_9_10)
        db.query(
            "SELECT documentId, isOwned, priceCredits, saleStatusRaw, " +
                "counterpartyIdentityId, documentCreatedAtMs, documentUpdatedAtMs, " +
                "documentTransferredAtMs, marketplaceUpdatedAt FROM dpns_names",
        ).use { cursor ->
            assertTrue(cursor.moveToFirst())
            assertTrue(cursor.isNull(0))
            assertEquals(1, cursor.getInt(1))
            assertTrue(cursor.isNull(2))
            assertEquals(0, cursor.getInt(3))
            assertTrue(cursor.isNull(4))
            assertEquals(0L, cursor.getLong(5))
            assertEquals(0L, cursor.getLong(6))
            assertEquals(0L, cursor.getLong(7))
            assertEquals(0L, cursor.getLong(8))
        }
        db.close()
    }

    /**
     * v10 → v11 adds `txos.supersededByTxid` (nullable) and
     * `pending_inputs.isSweptTombstone` (defaulted `false`) — both
     * additive. Pre-existing rows in each table must survive and read back
     * with the new columns at their defaults.
     */
    @Test
    fun migrate10To11AddsSweepClaimDurabilityColumns() {
        val legacy = helper.createDatabase(dbName, 10)
        legacy.execSQL(
            "INSERT INTO wallets (walletId, walletGroupId, networkRaw, name, birthHeight, " +
                "syncedHeight, lastSynced, isImported, createdAt, lastUpdated) " +
                "VALUES (x'01', x'02', 1, 'w', 0, 0, 0, 0, 0, 0)",
        )
        legacy.execSQL(
            "INSERT INTO transactions (txid, transactionData, context, blockHeight, " +
                "blockTimestamp, blockPosition, hasBlockPosition, direction, " +
                "transactionType, transactionTypeKind, netAmount, label, firstSeen, " +
                "createdAt, lastUpdated) " +
                "VALUES (x'02', x'00', 0, 0, 0, 0, 0, 0, 'Standard', 0, 0, '', 0, 0, 0)",
        )
        legacy.execSQL(
            "INSERT INTO txos (outpoint, vout, amount, address, scriptPubKey, height, " +
                "isCoinbase, isConfirmed, isInstantLocked, isLocked, isSpent, createdAt, " +
                "lastUpdated, walletId, txid) " +
                "VALUES (x'0201', 1, 1000, 'y', x'00', 0, 0, 0, 0, 0, 0, 0, 0, x'01', x'02')",
        )
        legacy.execSQL(
            "INSERT INTO pending_inputs (outpoint, inputIndex, spendingTxid, walletId, " +
                "createdAt) VALUES (x'0301', 0, x'02', x'01', 0)",
        )
        legacy.close()

        val db = helper.runMigrationsAndValidate(dbName, 11, true, DashDatabase.MIGRATION_10_11)
        db.query("SELECT supersededByTxid FROM txos WHERE outpoint = x'0201'").use { c ->
            assertTrue(c.moveToFirst())
            assertTrue(c.isNull(0))
        }
        db.query("SELECT isSweptTombstone FROM pending_inputs WHERE outpoint = x'0301'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(0, c.getInt(0))
        }
        db.close()
    }

    /**
     * v10 → v11 adds `transactions.isGloballySwept` (defaulted `false`) —
     * additive. Pre-existing rows must survive and read back not swept, and
     * the flag must accept an explicit `true` on write, mirroring
     * `migrate10To11AddsSweepClaimDurabilityColumns` above for the sibling
     * v11 columns.
     */
    @Test
    fun migrate11To12AddsGlobalSweptFlag() {
        val legacy = helper.createDatabase(dbName, 11)
        legacy.execSQL(
            "INSERT INTO transactions (txid, transactionData, context, blockHeight, " +
                "blockTimestamp, blockPosition, hasBlockPosition, direction, " +
                "transactionType, transactionTypeKind, netAmount, label, firstSeen, " +
                "createdAt, lastUpdated) " +
                "VALUES (x'02', x'00', 0, 0, 0, 0, 0, 0, 'Standard', 0, 0, '', 0, 0, 0)",
        )
        legacy.close()

        val db = helper.runMigrationsAndValidate(dbName, 12, true, DashDatabase.MIGRATION_11_12)
        db.query("SELECT isGloballySwept FROM transactions WHERE txid = x'02'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(0, c.getInt(0))
        }
        db.execSQL(
            "INSERT INTO transactions (txid, transactionData, context, blockHeight, " +
                "blockTimestamp, blockPosition, hasBlockPosition, direction, " +
                "transactionType, transactionTypeKind, netAmount, label, firstSeen, " +
                "createdAt, lastUpdated, isGloballySwept) " +
                "VALUES (x'03', x'00', 0, 0, 0, 0, 0, 0, 'Standard', 0, 0, '', 0, 0, 0, 1)",
        )
        db.query("SELECT isGloballySwept FROM transactions WHERE txid = x'03'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(1, c.getInt(0))
        }
        db.close()
    }

    /**
     * v12 → v13 adds `pending_inputs.winnerMinedHeight` and
     * `wallets.lastAppliedChainLockHeight` (both nullable, no default) —
     * additive. Pre-existing rows must survive and read back NULL
     * (an unstamped tombstone is never collected, and no chainlock height
     * means no finality boundary), and both columns must accept an
     * explicit value on write.
     */
    @Test
    fun migrate12To13AddsWinnerHeightAndChainLockHeight() {
        val legacy = helper.createDatabase(dbName, 12)
        legacy.execSQL(
            "INSERT INTO pending_inputs (outpoint, inputIndex, spendingTxid, " +
                "walletId, createdAt, isSweptTombstone) " +
                "VALUES (x'04', 0, x'05', x'06', 0, 1)",
        )
        legacy.execSQL(
            "INSERT INTO wallets (walletId, walletGroupId, networkRaw, name, birthHeight, " +
                "syncedHeight, lastSynced, isImported, createdAt, lastUpdated) " +
                "VALUES (x'06', x'02', 1, 'w', 0, 0, 0, 0, 0, 0)",
        )
        legacy.close()

        val db = helper.runMigrationsAndValidate(dbName, 13, true, DashDatabase.MIGRATION_12_13)
        db.query("SELECT winnerMinedHeight FROM pending_inputs WHERE outpoint = x'04'").use { c ->
            assertTrue(c.moveToFirst())
            assertTrue("pre-migration tombstones read back unstamped", c.isNull(0))
        }
        db.query("SELECT lastAppliedChainLockHeight FROM wallets WHERE walletId = x'06'").use { c ->
            assertTrue(c.moveToFirst())
            assertTrue("pre-migration wallets have no chainlock height on record", c.isNull(0))
        }
        db.execSQL(
            "INSERT INTO pending_inputs (outpoint, inputIndex, spendingTxid, " +
                "walletId, createdAt, isSweptTombstone, winnerMinedHeight) " +
                "VALUES (x'07', 0, x'05', x'06', 0, 1, 1234)",
        )
        db.query("SELECT winnerMinedHeight FROM pending_inputs WHERE outpoint = x'07'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(1234, c.getInt(0))
        }
        db.execSQL("UPDATE wallets SET lastAppliedChainLockHeight = 4321 WHERE walletId = x'06'")
        db.query("SELECT lastAppliedChainLockHeight FROM wallets WHERE walletId = x'06'").use { c ->
            assertTrue(c.moveToFirst())
            assertEquals(4321, c.getInt(0))
        }
        db.close()
    }

    /** The requested contiguous path from the pre-u64 v4 schema to latest. */
    @Test
    fun migrate4ToLatest() {
        helper.createDatabase(dbName, 4).close()
        helper.runMigrationsAndValidate(
            dbName,
            13,
            true,
            DashDatabase.MIGRATION_4_5,
            DashDatabase.MIGRATION_5_6,
            DashDatabase.MIGRATION_6_7,
            DashDatabase.MIGRATION_7_8,
            DashDatabase.MIGRATION_8_9,
            DashDatabase.MIGRATION_9_10,
            DashDatabase.MIGRATION_10_11,
            DashDatabase.MIGRATION_11_12,
            DashDatabase.MIGRATION_12_13,
        ).close()
    }

    /** The full chain from v1 must also land on a valid v13 schema. */
    @Test
    fun migrateAllTheWayFrom1() {
        helper.createDatabase(dbName, 1).close()
        helper.runMigrationsAndValidate(
            dbName,
            13,
            true,
            DashDatabase.MIGRATION_1_2,
            DashDatabase.MIGRATION_2_3,
            DashDatabase.MIGRATION_3_4,
            DashDatabase.MIGRATION_4_5,
            DashDatabase.MIGRATION_5_6,
            DashDatabase.MIGRATION_6_7,
            DashDatabase.MIGRATION_7_8,
            DashDatabase.MIGRATION_8_9,
            DashDatabase.MIGRATION_9_10,
            DashDatabase.MIGRATION_10_11,
            DashDatabase.MIGRATION_11_12,
            DashDatabase.MIGRATION_12_13,
        ).close()
    }
}
