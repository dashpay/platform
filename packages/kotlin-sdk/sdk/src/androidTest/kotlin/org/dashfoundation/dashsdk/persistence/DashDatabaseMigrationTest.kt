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

    /** The full chain from v1 must also land on a valid v3 schema. */
    @Test
    fun migrateAllTheWayFrom1() {
        helper.createDatabase(dbName, 1).close()
        helper.runMigrationsAndValidate(
            dbName,
            3,
            true,
            DashDatabase.MIGRATION_1_2,
            DashDatabase.MIGRATION_2_3,
        ).close()
    }
}
