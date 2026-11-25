package org.dashfoundation.dashsdk.services

import androidx.room.withTransaction
import kotlinx.coroutines.NonCancellable
import kotlinx.coroutines.flow.Flow
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.WalletStorage

/**
 * Bulk data-management operations — port of `Services/DataManager.swift`,
 * backing the Data Management screen's clear-by-category actions.
 *
 * Categories mirror the iOS grouping. Every clear runs in one transaction;
 * child tables clear before parents so FK constraints hold.
 *
 * Clearing [Category.IDENTITIES] also deletes every masternode voting key imported
 * through Load Identity ([MasternodeVotingKeyStore.deleteAll]) from the [WalletStorage]
 * given, BEFORE the rows: an interrupted or failed clear then leaves rows whose voting keys
 * are gone (a vote says the key is not stored), the category still counts rows, and clearing
 * again finishes the job. The reverse order could leave keys no row names and no count
 * shows. Both steps run under [MasternodeVotingKeyStore.withVotingKeyLock], so an import
 * cannot store and record a key between them. Each clear runs to the end once started, even if its caller is cancelled.
 * Identity keys of wallets and mnemonics stay in secure storage.
 */
class DataManager internal constructor(
    private val db: DashDatabase,
    private val deleteImportedVotingKeys: suspend () -> Unit,
) {

    /** @param walletStorage the store the app's imported voting keys live in. */
    constructor(db: DashDatabase, walletStorage: WalletStorage) :
        this(db, { MasternodeVotingKeyStore(walletStorage).deleteAll() })

    enum class Category(val displayName: String) {
        WALLETS("Wallets & Accounts"),
        TRANSACTIONS("Transactions & UTXOs"),
        IDENTITIES("Identities & Keys"),
        DPNS("DPNS Names"),
        DASHPAY("DashPay"),
        CONTRACTS("Data Contracts & Documents"),
        TOKENS("Tokens"),
        PLATFORM_ADDRESSES("Platform Addresses"),
        SHIELDED("Shielded Pool"),
        ASSET_LOCKS("Asset Locks"),
    }

    /** Row count Flow per category (sum of its tables), for the UI. */
    fun count(category: Category): Flow<Long> {
        val c = db.storageCountsDao()
        return when (category) {
            Category.WALLETS -> c.countWallets()
            Category.TRANSACTIONS -> c.countTransactions()
            Category.IDENTITIES -> c.countIdentities()
            Category.DPNS -> c.countDpnsNames()
            Category.DASHPAY -> c.countDashpayContactRequests()
            Category.CONTRACTS -> c.countDataContracts()
            Category.TOKENS -> c.countTokens()
            Category.PLATFORM_ADDRESSES -> c.countPlatformAddresses()
            Category.SHIELDED -> c.countShieldedNotes()
            Category.ASSET_LOCKS -> c.countAssetLocks()
        }
    }

    suspend fun clear(category: Category) = withContext(NonCancellable) {
        if (category == Category.IDENTITIES) {
            // One voting key critical section: no import can store and record a key between
            // the key sweep and the row delete.
            MasternodeVotingKeyStore.withVotingKeyLock {
                deleteImportedVotingKeys()
                clearRows(category)
            }
        } else {
            clearRows(category)
        }
    }

    private suspend fun clearRows(category: Category) = db.withTransaction {
        when (category) {
            Category.WALLETS -> {
                // Children first; wallets cascade accounts but be explicit.
                db.accountDao().deleteAll()
                db.walletDao().deleteAll()
                db.walletManagerMetadataDao().deleteAll()
            }
            Category.TRANSACTIONS -> {
                db.txoDao().deleteAll()
                db.transactionDao().deleteAll()
                db.coreAddressDao().deleteAll()
            }
            Category.IDENTITIES -> {
                db.publicKeyDao().deleteAll()
                db.identityDao().deleteAll()
            }
            Category.DPNS -> db.dpnsNameDao().deleteAll()
            Category.DASHPAY -> {
                db.dashpayDao().deleteAllContactRequests()
                db.dashpayDao().deleteAllProfiles()
            }
            Category.CONTRACTS -> {
                db.documentDao().deleteAllPendingInputs()
                db.documentDao().deleteAllProperties()
                db.documentDao().deleteAllKeywords()
                db.documentDao().deleteAllIndices()
                db.documentDao().deleteAllDocuments()
                db.documentDao().deleteAllDocumentTypes()
                // Tokens FK-reference contracts; clear them first.
                db.tokenDao().deleteAllHistoryEvents()
                db.tokenDao().deleteAllBalances()
                db.tokenDao().deleteAllTokens()
                db.dataContractDao().deleteAll()
            }
            Category.TOKENS -> {
                db.tokenDao().deleteAllHistoryEvents()
                db.tokenDao().deleteAllBalances()
                db.tokenDao().deleteAllTokens()
            }
            Category.PLATFORM_ADDRESSES -> {
                db.platformAddressDao().deleteAllSyncStates()
                db.platformAddressDao().deleteAll()
            }
            Category.SHIELDED -> {
                db.shieldedDao().deleteAllActivity()
                db.shieldedDao().deleteAllOutgoingNotes()
                db.shieldedDao().deleteAllNotes()
                db.shieldedDao().deleteAllSyncStates()
            }
            Category.ASSET_LOCKS -> db.assetLockDao().deleteAll()
        }
    }

    /** Wipe every table — the iOS "Clear All Data" action. */
    suspend fun clearAll() {
        // Order matters only within a transaction; clearing categories in
        // dependency order covers all FKs.
        for (category in listOf(
            Category.SHIELDED,
            Category.ASSET_LOCKS,
            Category.PLATFORM_ADDRESSES,
            Category.TOKENS,
            Category.CONTRACTS,
            Category.DASHPAY,
            Category.DPNS,
            Category.IDENTITIES,
            Category.TRANSACTIONS,
            Category.WALLETS,
        )) {
            clear(category)
        }
    }
}
