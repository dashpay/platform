package org.dashfoundation.dashsdk.persistence

import android.content.Context
import androidx.room.Database
import androidx.room.Room
import androidx.room.RoomDatabase
import androidx.room.TypeConverters
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
import org.dashfoundation.dashsdk.persistence.entities.DashpayContactRequestEntity
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
 */
@Database(
    version = 1,
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
         * Build the on-disk database. WAL is Room's default journal mode on
         * API 16+; writes go through the persistence handler inside
         * `withTransaction`, mirroring the changeset bracketing contract of
         * `platform-wallet-ffi`.
         */
        fun create(context: Context): DashDatabase =
            Room.databaseBuilder(context, DashDatabase::class.java, DATABASE_NAME)
                .build()

        /** In-memory variant for tests. */
        fun createInMemory(context: Context): DashDatabase =
            Room.inMemoryDatabaseBuilder(context, DashDatabase::class.java)
                .allowMainThreadQueries()
                .build()
    }
}
