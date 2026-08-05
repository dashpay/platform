package org.dashfoundation.dashsdk.services

import androidx.room.withTransaction
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.DashDatabase

/**
 * Bulk data-management operations — port of `Services/DataManager.swift`,
 * backing the Data Management screen's clear-by-category actions.
 *
 * Categories mirror the iOS grouping. Every clear runs in one transaction;
 * child tables clear before parents so FK constraints hold.
 */
class DataManager(private val db: DashDatabase) {

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

    suspend fun clear(category: Category) = db.withTransaction {
        when (category) {
            Category.WALLETS -> {
                // Children first; wallets cascade accounts but be explicit.
                db.invitationDao().deleteAll()
                // Keep the identity-index high-water marks. Data management
                // clears Room rows but does not erase the Keystore mnemonic,
                // so the same wallet can be restored and must never reuse a
                // previously issued DIP-9 derivation path.
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
