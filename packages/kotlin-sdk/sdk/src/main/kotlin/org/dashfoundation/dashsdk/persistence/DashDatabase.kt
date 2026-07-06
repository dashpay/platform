package org.dashfoundation.dashsdk.persistence

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.TypeConverters
import androidx.room.migration.Migration
import androidx.sqlite.db.SupportSQLiteDatabase
import org.dashfoundation.dashsdk.persistence.converters.Converters
import org.dashfoundation.dashsdk.persistence.dao.AccountDao
import org.dashfoundation.dashsdk.persistence.dao.AssetLockDao
import org.dashfoundation.dashsdk.persistence.dao.CoreAddressDao
import org.dashfoundation.dashsdk.persistence.dao.DashpayDao
import org.dashfoundation.dashsdk.persistence.dao.DataContractDao
import org.dashfoundation.dashsdk.persistence.dao.DocumentDao
import org.dashfoundation.dashsdk.persistence.dao.DpnsNameDao
import org.dashfoundation.dashsdk.persistence.dao.IdentityDao
import org.dashfoundation.dashsdk.persistence.dao.PlatformAddressDao
import org.dashfoundation.dashsdk.persistence.dao.PublicKeyDao
import org.dashfoundation.dashsdk.persistence.dao.ShieldedDao
import org.dashfoundation.dashsdk.persistence.dao.StorageCountsDao
import org.dashfoundation.dashsdk.persistence.dao.TokenDao
import org.dashfoundation.dashsdk.persistence.dao.TransactionDao
import org.dashfoundation.dashsdk.persistence.dao.TxoDao
import org.dashfoundation.dashsdk.persistence.dao.WalletDao
import org.dashfoundation.dashsdk.persistence.dao.WalletManagerMetadataDao
import org.dashfoundation.dashsdk.persistence.entities.AccountEntity
import org.dashfoundation.dashsdk.persistence.entities.AssetLockEntity
import org.dashfoundation.dashsdk.persistence.entities.CoreAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayIgnoredSenderEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayPaymentEntity
import org.dashfoundation.dashsdk.persistence.entities.DashpayProfileEntity
import org.dashfoundation.dashsdk.persistence.entities.DataContractEntity
import org.dashfoundation.dashsdk.persistence.entities.DocumentEntity
import org.dashfoundation.dashsdk.persistence.entities.DocumentTypeEntity
import org.dashfoundation.dashsdk.persistence.entities.DpnsNameEntity
import org.dashfoundation.dashsdk.persistence.entities.IdentityEntity
import org.dashfoundation.dashsdk.persistence.entities.IndexEntity
import org.dashfoundation.dashsdk.persistence.entities.KeywordEntity
import org.dashfoundation.dashsdk.persistence.entities.PendingInputEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressEntity
import org.dashfoundation.dashsdk.persistence.entities.PlatformAddressesSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.PropertyEntity
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedActivityEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedOutgoingNoteEntity
import org.dashfoundation.dashsdk.persistence.entities.ShieldedSyncStateEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenHistoryEventEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TxoEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletEntity
import org.dashfoundation.dashsdk.persistence.entities.WalletManagerMetadataEntity

/**
 * The SDK's Room database — Android counterpart of
 * `DashModelContainer.swift` (SwiftData `ModelContainer` with the 28
 * persistent model types).
 *
 * Version 1 ships with exported schemas (`sdk/schemas/`) so future
 * migrations start from a recorded baseline; destructive fallback is
 * intentionally NOT enabled.
 *
 * Version 2 (DashPay ignore + contactInfo metadata, upstream #3841):
 * adds the `dashpay_ignored_senders` table (durable per-sender mute rows
 * restored into the Rust `ignored_senders` set at load) and the
 * established-row metadata columns on `dashpay_contact_requests`
 * (`paymentChannelBroken` / `contactAlias` / `contactNote` /
 * `contactHidden` / `contactAccountLabel` / `contactAcceptedAccounts`).
 *
 * Version 3 (DashPay contact-profile cache + payment history): adds the
 * `dashpay_contact_profiles` table (mirror of the Rust
 * `contact_profiles` map — persister-projected with tombstone deletes,
 * restored at load) and the `dashpay_payments` table (mirror of the Rust
 * `dashpay_payments` map — pull-persisted via `refreshDashPayPayments`,
 * restored at load).
 */
@Database(
    version = 3,
    exportSchema = true,
    entities = [
        WalletEntity::class,
        AccountEntity::class,
        TransactionEntity::class,
        TxoEntity::class,
        CoreAddressEntity::class,
        AssetLockEntity::class,
        IdentityEntity::class,
        PublicKeyEntity::class,
        DpnsNameEntity::class,
        DashpayProfileEntity::class,
        DashpayContactRequestEntity::class,
        DashpayIgnoredSenderEntity::class,
        DashpayContactProfileEntity::class,
        DashpayPaymentEntity::class,
        DataContractEntity::class,
        DocumentTypeEntity::class,
        DocumentEntity::class,
        IndexEntity::class,
        KeywordEntity::class,
        PropertyEntity::class,
        PendingInputEntity::class,
        TokenEntity::class,
        TokenBalanceEntity::class,
        TokenHistoryEventEntity::class,
        PlatformAddressEntity::class,
        PlatformAddressesSyncStateEntity::class,
        ShieldedNoteEntity::class,
        ShieldedOutgoingNoteEntity::class,
        ShieldedActivityEntity::class,
        ShieldedSyncStateEntity::class,
        WalletManagerMetadataEntity::class,
    ],
)
@TypeConverters(Converters::class)
abstract class DashDatabase : RoomDatabase() {

    abstract fun walletDao(): WalletDao
    abstract fun accountDao(): AccountDao
    abstract fun transactionDao(): TransactionDao
    abstract fun txoDao(): TxoDao
    abstract fun coreAddressDao(): CoreAddressDao
    abstract fun assetLockDao(): AssetLockDao
    abstract fun identityDao(): IdentityDao
    abstract fun publicKeyDao(): PublicKeyDao
    abstract fun dpnsNameDao(): DpnsNameDao
    abstract fun dashpayDao(): DashpayDao
    abstract fun dataContractDao(): DataContractDao
    abstract fun documentDao(): DocumentDao
    abstract fun tokenDao(): TokenDao
    abstract fun platformAddressDao(): PlatformAddressDao
    abstract fun shieldedDao(): ShieldedDao
    abstract fun walletManagerMetadataDao(): WalletManagerMetadataDao
    abstract fun storageCountsDao(): StorageCountsDao

    companion object {
        const val DATABASE_NAME: String = "dash-sdk.db"

        /**
         * v1 → v2: the DashPay ignore + contactInfo-metadata reshape
         * (upstream #3841). New `dashpay_ignored_senders` table + six
         * metadata columns on `dashpay_contact_requests`. SQL mirrors the
         * exported `schemas/.../2.json` `createSql` exactly.
         */
        val MIGRATION_1_2: Migration = object : Migration(1, 2) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `dashpay_ignored_senders` (" +
                        "`networkRaw` INTEGER NOT NULL, " +
                        "`ownerIdentityId` BLOB NOT NULL, " +
                        "`ignoredSenderId` BLOB NOT NULL, " +
                        "`ignoredAt` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`networkRaw`, `ownerIdentityId`, `ignoredSenderId`), " +
                        "FOREIGN KEY(`ownerIdentityId`) REFERENCES `identities`(`identityId`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE )",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS `index_dashpay_ignored_senders_ownerIdentityId` " +
                        "ON `dashpay_ignored_senders` (`ownerIdentityId`)",
                )
                db.execSQL(
                    "ALTER TABLE `dashpay_contact_requests` " +
                        "ADD COLUMN `paymentChannelBroken` INTEGER NOT NULL DEFAULT 0",
                )
                db.execSQL(
                    "ALTER TABLE `dashpay_contact_requests` ADD COLUMN `contactAlias` TEXT",
                )
                db.execSQL(
                    "ALTER TABLE `dashpay_contact_requests` ADD COLUMN `contactNote` TEXT",
                )
                db.execSQL(
                    "ALTER TABLE `dashpay_contact_requests` " +
                        "ADD COLUMN `contactHidden` INTEGER NOT NULL DEFAULT 0",
                )
                db.execSQL(
                    "ALTER TABLE `dashpay_contact_requests` ADD COLUMN `contactAccountLabel` TEXT",
                )
                db.execSQL(
                    "ALTER TABLE `dashpay_contact_requests` " +
                        "ADD COLUMN `contactAcceptedAccounts` BLOB",
                )
            }
        }

        /**
         * v2 → v3: the DashPay contact-profile cache + payment-history
         * tables. Purely additive. SQL mirrors the exported
         * `schemas/.../3.json` `createSql` exactly (column order = entity
         * field order).
         */
        val MIGRATION_2_3: Migration = object : Migration(2, 3) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `dashpay_contact_profiles` (" +
                        "`networkRaw` INTEGER NOT NULL, " +
                        "`ownerIdentityId` BLOB NOT NULL, " +
                        "`contactIdentityId` BLOB NOT NULL, " +
                        "`displayName` TEXT, " +
                        "`publicMessage` TEXT, " +
                        "`bio` TEXT, " +
                        "`avatarUrl` TEXT, " +
                        "`avatarHash` BLOB, " +
                        "`avatarFingerprint` BLOB, " +
                        "`checkedAtMs` INTEGER NOT NULL, " +
                        "`createdAt` INTEGER NOT NULL, " +
                        "`lastUpdated` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`networkRaw`, `ownerIdentityId`, `contactIdentityId`), " +
                        "FOREIGN KEY(`ownerIdentityId`) REFERENCES `identities`(`identityId`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE )",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS `index_dashpay_contact_profiles_ownerIdentityId` " +
                        "ON `dashpay_contact_profiles` (`ownerIdentityId`)",
                )
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `dashpay_payments` (" +
                        "`networkRaw` INTEGER NOT NULL, " +
                        "`ownerIdentityId` BLOB NOT NULL, " +
                        "`counterpartyIdentityId` BLOB NOT NULL, " +
                        "`amountDuffs` INTEGER NOT NULL, " +
                        "`directionRaw` INTEGER NOT NULL, " +
                        "`statusRaw` INTEGER NOT NULL, " +
                        "`txid` TEXT NOT NULL, " +
                        "`memo` TEXT, " +
                        "`createdAt` INTEGER NOT NULL, " +
                        "`lastUpdated` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`networkRaw`, `ownerIdentityId`, `txid`), " +
                        "FOREIGN KEY(`ownerIdentityId`) REFERENCES `identities`(`identityId`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE )",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS `index_dashpay_payments_ownerIdentityId` " +
                        "ON `dashpay_payments` (`ownerIdentityId`)",
                )
            }
        }

        /**
         * Build the on-disk database. WAL is Room's default journal mode on
         * API 16+; writes go through the persistence handler inside
         * `withTransaction`, mirroring the changeset bracketing contract of
         * `platform-wallet-ffi`.
         */
        fun create(context: Context): DashDatabase =
            Room.databaseBuilder(context, DashDatabase::class.java, DATABASE_NAME)
                .addMigrations(MIGRATION_1_2, MIGRATION_2_3)
                .build()

        /** In-memory variant for tests. */
        fun createInMemory(context: Context): DashDatabase =
            Room.inMemoryDatabaseBuilder(context, DashDatabase::class.java)
                .allowMainThreadQueries()
                .build()
    }
}
