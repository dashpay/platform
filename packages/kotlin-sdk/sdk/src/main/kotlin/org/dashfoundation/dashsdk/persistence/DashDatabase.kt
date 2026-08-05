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
import org.dashfoundation.dashsdk.persistence.dao.IdentityIndexStateDao
import org.dashfoundation.dashsdk.persistence.dao.InvitationDao
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
import org.dashfoundation.dashsdk.persistence.entities.IdentityIndexStateEntity
import org.dashfoundation.dashsdk.persistence.entities.IndexEntity
import org.dashfoundation.dashsdk.persistence.entities.InvitationEntity
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
import org.dashfoundation.dashsdk.persistence.entities.ShieldedViewingKeyEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenHistoryEventEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionEntity
import org.dashfoundation.dashsdk.persistence.entities.TransactionAccountInvolvementEntity
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
 *
 * Version 4 (wallet-scoped platform addresses): rebuilds
 * `platform_addresses` with the composite `(walletId, address)` primary
 * key and per-wallet `addressHash` uniqueness, replacing the global
 * `address` PK / global `addressHash` unique index that let one wallet's
 * pool-emit steal another wallet's row (same seed imported twice derives
 * identical addresses).
 *
 * Version 5 (unsigned token balances): rebuilds `token_balances.balance`
 * from a signed SQLite INTEGER raw-bit carrier to an order-preserving
 * fixed-width big-endian BLOB. Every legacy signed bit pattern is retained.
 *
 * Version 6 (seedless Orchard rebind): adds one raw 96-byte full viewing
 * key per `(walletId, accountIndex)`.
 *
 * Version 7 (provider restore): adds transaction block position and an
 * explicit transaction↔typed-account involvement table for payload-only
 * provider transactions.
 *
 * Version 8 (durable repair signal, dashpay/platform#4060): adds the
 * nullable `public_keys.derivationIdentityIndex` / `derivationKeyIndex`
 * derivation-breadcrumb columns, so identity keys whose private half is
 * missing or undecryptable can re-seed the pending-repair state after a
 * process restart. Room is the durability substrate deliberately: the
 * wallet-deletion cascade removes these rows, so pending entries die with
 * their wallet automatically (a DataStore side-table would leak).
 *
 * Version 9 (DIP-13 invitations): adds the `invitations` table (mirror of
 * the Swift `PersistentInvitation` model — push-persisted by the
 * `on_persist_invitations_fn` callback, no Rust rehydrate, no secret
 * column; the "Sent invitations" list reads it via a Room `Flow`). Also
 * adds the per-wallet monotonic identity-index state used to prevent a
 * derived registration slot from being reused after process death.
 */
@Database(
    version = 9,
    exportSchema = true,
    entities = [
        WalletEntity::class,
        AccountEntity::class,
        TransactionEntity::class,
        TransactionAccountInvolvementEntity::class,
        TxoEntity::class,
        CoreAddressEntity::class,
        AssetLockEntity::class,
        InvitationEntity::class,
        IdentityIndexStateEntity::class,
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
        ShieldedViewingKeyEntity::class,
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
    abstract fun invitationDao(): InvitationDao
    abstract fun identityIndexStateDao(): IdentityIndexStateDao
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
         * v3 → v4: reshape `platform_addresses` from a global `address`
         * primary key (+ globally unique `addressHash`) to the composite
         * `(walletId, address)` identity with per-wallet `addressHash`
         * uniqueness. SQLite can't alter a primary key in place, so this
         * is the standard table rebuild: create-copy-drop-rename. The
         * copy needs no dedup — the old constraints were strictly
         * TIGHTER (globally-unique address/hash implies per-wallet
         * unique), so every legacy row keeps its slot. The standalone
         * `walletId` index is dropped: both new composite indexes lead
         * with `walletId`, so prefix scans cover the wallet-scoped
         * queries. SQL mirrors the exported `schemas/.../4.json`
         * `createSql` exactly.
         */
        val MIGRATION_3_4: Migration = object : Migration(3, 4) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `_new_platform_addresses` (" +
                        "`address` TEXT NOT NULL, " +
                        "`addressType` INTEGER NOT NULL, " +
                        "`addressHash` BLOB NOT NULL, " +
                        "`publicKey` BLOB NOT NULL, " +
                        "`accountIndex` INTEGER NOT NULL, " +
                        "`addressIndex` INTEGER NOT NULL, " +
                        "`derivationPath` TEXT NOT NULL, " +
                        "`isUsed` INTEGER NOT NULL, " +
                        "`balance` INTEGER NOT NULL, " +
                        "`nonce` INTEGER NOT NULL, " +
                        "`firstSeenHeight` INTEGER NOT NULL, " +
                        "`lastSeenHeight` INTEGER NOT NULL, " +
                        "`walletId` BLOB NOT NULL, " +
                        "`createdAt` INTEGER NOT NULL, " +
                        "`lastUpdated` INTEGER NOT NULL, " +
                        "`accountId` INTEGER, " +
                        "PRIMARY KEY(`walletId`, `address`), " +
                        "FOREIGN KEY(`accountId`) REFERENCES `accounts`(`id`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE )",
                )
                db.execSQL(
                    "INSERT INTO `_new_platform_addresses` (" +
                        "`address`, `addressType`, `addressHash`, `publicKey`, " +
                        "`accountIndex`, `addressIndex`, `derivationPath`, `isUsed`, " +
                        "`balance`, `nonce`, `firstSeenHeight`, `lastSeenHeight`, " +
                        "`walletId`, `createdAt`, `lastUpdated`, `accountId`) " +
                        "SELECT `address`, `addressType`, `addressHash`, `publicKey`, " +
                        "`accountIndex`, `addressIndex`, `derivationPath`, `isUsed`, " +
                        "`balance`, `nonce`, `firstSeenHeight`, `lastSeenHeight`, " +
                        "`walletId`, `createdAt`, `lastUpdated`, `accountId` " +
                        "FROM `platform_addresses`",
                )
                db.execSQL("DROP TABLE `platform_addresses`")
                db.execSQL(
                    "ALTER TABLE `_new_platform_addresses` RENAME TO `platform_addresses`",
                )
                db.execSQL(
                    "CREATE UNIQUE INDEX IF NOT EXISTS " +
                        "`index_platform_addresses_walletId_addressHash` " +
                        "ON `platform_addresses` (`walletId`, `addressHash`)",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS `index_platform_addresses_accountId` " +
                        "ON `platform_addresses` (`accountId`)",
                )
            }
        }

        /**
         * v4 → v5: make token balance storage unsigned-safe. The initial
         * INSERT preserves all non-balance columns; a prepared statement then
         * rewrites each legacy INTEGER's raw bits into the canonical 8-byte
         * big-endian representation before the old table is removed.
         */
        val MIGRATION_4_5: Migration = object : Migration(4, 5) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `_new_token_balances` (" +
                        "`id` INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL, " +
                        "`tokenId` TEXT NOT NULL, `identityId` BLOB NOT NULL, " +
                        "`balance` BLOB NOT NULL, `frozen` INTEGER NOT NULL, " +
                        "`createdAt` INTEGER NOT NULL, `lastUpdated` INTEGER NOT NULL, " +
                        "`lastSyncedAt` INTEGER, `tokenName` TEXT, `tokenSymbol` TEXT, " +
                        "`tokenDecimals` INTEGER, `networkRaw` INTEGER NOT NULL, " +
                        "`identityRef` BLOB, `tokenRef` BLOB, " +
                        "FOREIGN KEY(`identityRef`) REFERENCES `identities`(`identityId`) " +
                        "ON UPDATE NO ACTION ON DELETE SET NULL , " +
                        "FOREIGN KEY(`tokenRef`) REFERENCES `tokens`(`id`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE )",
                )
                db.execSQL(
                    "INSERT INTO `_new_token_balances` (`id`, `tokenId`, `identityId`, " +
                        "`balance`, `frozen`, `createdAt`, `lastUpdated`, `lastSyncedAt`, " +
                        "`tokenName`, `tokenSymbol`, `tokenDecimals`, `networkRaw`, " +
                        "`identityRef`, `tokenRef`) SELECT `id`, `tokenId`, `identityId`, " +
                        "zeroblob(8), `frozen`, `createdAt`, `lastUpdated`, `lastSyncedAt`, " +
                        "`tokenName`, `tokenSymbol`, `tokenDecimals`, `networkRaw`, " +
                        "`identityRef`, `tokenRef` FROM `token_balances`",
                )
                val update = db.compileStatement(
                    "UPDATE `_new_token_balances` SET `balance` = ? WHERE `id` = ?",
                )
                db.query("SELECT `id`, `balance` FROM `token_balances`").use { cursor ->
                    while (cursor.moveToNext()) {
                        update.clearBindings()
                        update.bindBlob(
                            1,
                            UInt64Value.fromRawLongBits(cursor.getLong(1)).toBigEndianBytes(),
                        )
                        update.bindLong(2, cursor.getLong(0))
                        update.executeUpdateDelete()
                    }
                }
                db.execSQL("DROP TABLE `token_balances`")
                db.execSQL("ALTER TABLE `_new_token_balances` RENAME TO `token_balances`")
                listOf(
                    "networkRaw",
                    "tokenId_identityId",
                    "identityId",
                    "identityRef",
                    "tokenRef",
                ).forEach { suffix ->
                    val columns = when (suffix) {
                        "tokenId_identityId" -> "`tokenId`, `identityId`"
                        else -> "`$suffix`"
                    }
                    db.execSQL(
                        "CREATE INDEX IF NOT EXISTS `index_token_balances_$suffix` " +
                            "ON `token_balances` ($columns)",
                    )
                }
            }
        }

        /** v5 → v6: add the per-subwallet Orchard full-viewing-key table. */
        val MIGRATION_5_6: Migration = object : Migration(5, 6) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `shielded_viewing_keys` (" +
                        "`walletId` BLOB NOT NULL, `accountIndex` INTEGER NOT NULL, " +
                        "`fvkBytes` BLOB NOT NULL, `lastUpdated` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`walletId`, `accountIndex`))",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS `index_shielded_viewing_keys_walletId` " +
                        "ON `shielded_viewing_keys` (`walletId`)",
                )
            }
        }

        /**
         * v6 → v7: provider-special transaction restore foundation.
         *
         * The involvement table intentionally starts empty. Payload-only
         * account ownership cannot be inferred from SQL without duplicating
         * Rust consensus decoding, and TXO membership is not a safe proxy for
         * provider-key ownership. The intermediate v5/v6 schemas were created
         * during this unreleased SDK work and were never shipped. Developer
         * databases upgraded from them must replay/resync Core transactions so
         * Rust emits the typed account tuple and v7 records exact involvement;
         * migration never fabricates ownership for legacy provider rows.
         */
        val MIGRATION_6_7: Migration = object : Migration(6, 7) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "ALTER TABLE `transactions` ADD COLUMN `blockPosition` " +
                        "INTEGER NOT NULL DEFAULT 0",
                )
                db.execSQL(
                    "ALTER TABLE `transactions` ADD COLUMN `hasBlockPosition` " +
                        "INTEGER NOT NULL DEFAULT 0",
                )
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `transaction_account_involvements` (" +
                        "`transactionTxid` BLOB NOT NULL, `accountId` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`transactionTxid`, `accountId`), " +
                        "FOREIGN KEY(`transactionTxid`) REFERENCES `transactions`(`txid`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE , " +
                        "FOREIGN KEY(`accountId`) REFERENCES `accounts`(`id`) " +
                        "ON UPDATE NO ACTION ON DELETE CASCADE )",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS " +
                        "`index_transaction_account_involvements_accountId` " +
                        "ON `transaction_account_involvements` (`accountId`)",
                )
            }
        }

        /**
         * v7 → v8: additive nullable derivation-breadcrumb columns on
         * `public_keys` (dashpay/platform#4060 finding 5). NULL for every
         * pre-existing row — correct, because a legacy row's breadcrumbs are
         * unknown; the persist callback back-fills them on the next upsert of
         * each key, after which the pending-repair reconstruction can see it.
         */
        val MIGRATION_7_8: Migration = object : Migration(7, 8) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "ALTER TABLE `public_keys` ADD COLUMN `derivationIdentityIndex` INTEGER",
                )
                db.execSQL(
                    "ALTER TABLE `public_keys` ADD COLUMN `derivationKeyIndex` INTEGER",
                )
            }
        }

        /**
         * v8 → v9: add the DIP-13 `invitations` table. Purely additive. SQL
         * mirrors the exported `schemas/.../9.json` `createSql` exactly
         * (column order = entity field order).
         */
        val MIGRATION_8_9: Migration = object : Migration(8, 9) {
            override fun migrate(db: SupportSQLiteDatabase) {
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `invitations` (" +
                        "`outPointHex` TEXT NOT NULL, " +
                        "`rawOutPoint` BLOB NOT NULL, " +
                        "`walletId` BLOB NOT NULL, " +
                        "`fundingIndexRaw` INTEGER NOT NULL, " +
                        "`amountDuffs` INTEGER NOT NULL, " +
                        "`expiryUnix` INTEGER NOT NULL, " +
                        "`createdAtSecs` INTEGER NOT NULL, " +
                        "`hasInviter` INTEGER NOT NULL, " +
                        "`statusRaw` INTEGER NOT NULL, " +
                        "`reclaimInFlight` INTEGER NOT NULL, " +
                        "`createdAt` INTEGER NOT NULL, " +
                        "`updatedAt` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`outPointHex`))",
                )
                db.execSQL(
                    "CREATE INDEX IF NOT EXISTS `index_invitations_walletId` " +
                        "ON `invitations` (`walletId`)",
                )
                db.execSQL(
                    "CREATE TABLE IF NOT EXISTS `identity_index_state` (" +
                        "`walletId` BLOB NOT NULL, " +
                        "`lastIssuedIndex` INTEGER NOT NULL, " +
                        "PRIMARY KEY(`walletId`))",
                )
                db.execSQL(
                    "INSERT INTO `identity_index_state` (`walletId`, `lastIssuedIndex`) " +
                        "SELECT `walletId`, MAX(`identityIndex`) FROM (" +
                        "SELECT `walletId`, `identityIndex` AS `identityIndex` " +
                        "FROM `identities` WHERE `walletId` IS NOT NULL " +
                        "UNION ALL " +
                        "SELECT `walletId`, `identityIndexRaw` AS `identityIndex` " +
                        "FROM `asset_locks` WHERE `fundingTypeRaw` = 0" +
                        ") GROUP BY `walletId`",
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
                .addMigrations(
                    MIGRATION_1_2,
                    MIGRATION_2_3,
                    MIGRATION_3_4,
                    MIGRATION_4_5,
                    MIGRATION_5_6,
                    MIGRATION_6_7,
                    MIGRATION_7_8,
                    MIGRATION_8_9,
                )
                .build()

        /** In-memory variant for tests. */
        fun createInMemory(context: Context): DashDatabase =
            Room.inMemoryDatabaseBuilder(context, DashDatabase::class.java)
                .allowMainThreadQueries()
                .build()
    }
}
