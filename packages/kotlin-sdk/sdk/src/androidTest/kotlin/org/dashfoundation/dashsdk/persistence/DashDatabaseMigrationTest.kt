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

    /** The full chain from v1 must also land on a valid v4 schema. */
    @Test
    fun migrateAllTheWayFrom1() {
        helper.createDatabase(dbName, 1).close()
        helper.runMigrationsAndValidate(
            dbName,
            4,
            true,
            DashDatabase.MIGRATION_1_2,
            DashDatabase.MIGRATION_2_3,
            DashDatabase.MIGRATION_3_4,
        ).close()
    }
}
