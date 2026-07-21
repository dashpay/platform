package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Query
import kotlinx.coroutines.flow.Flow

/**
 * Row counts for every table — feeds the Storage Explorer screen, the
 * Android port of `StorageExplorerView.swift` which shows one row per
 * persistent model with its record count.
 */
@Dao
interface StorageCountsDao {

    @Query("SELECT COUNT(*) FROM wallets") fun countWallets(): Flow<Long>

    @Query("SELECT COUNT(*) FROM accounts") fun countAccounts(): Flow<Long>

    @Query("SELECT COUNT(*) FROM transactions") fun countTransactions(): Flow<Long>

    @Query("SELECT COUNT(*) FROM transaction_account_involvements")
    fun countTransactionAccountInvolvements(): Flow<Long>

    @Query("SELECT COUNT(*) FROM txos") fun countTxos(): Flow<Long>

    @Query("SELECT COUNT(*) FROM core_addresses") fun countCoreAddresses(): Flow<Long>

    @Query("SELECT COUNT(*) FROM asset_locks") fun countAssetLocks(): Flow<Long>

    @Query("SELECT COUNT(*) FROM identities") fun countIdentities(): Flow<Long>

    @Query("SELECT COUNT(*) FROM public_keys") fun countPublicKeys(): Flow<Long>

    @Query("SELECT COUNT(*) FROM dpns_names") fun countDpnsNames(): Flow<Long>

    @Query("SELECT COUNT(*) FROM dashpay_profiles") fun countDashpayProfiles(): Flow<Long>

    @Query("SELECT COUNT(*) FROM dashpay_contact_requests")
    fun countDashpayContactRequests(): Flow<Long>

    @Query("SELECT COUNT(*) FROM data_contracts") fun countDataContracts(): Flow<Long>

    @Query("SELECT COUNT(*) FROM document_types") fun countDocumentTypes(): Flow<Long>

    @Query("SELECT COUNT(*) FROM documents") fun countDocuments(): Flow<Long>

    @Query("SELECT COUNT(*) FROM indices") fun countIndices(): Flow<Long>

    @Query("SELECT COUNT(*) FROM keywords") fun countKeywords(): Flow<Long>

    @Query("SELECT COUNT(*) FROM properties") fun countProperties(): Flow<Long>

    @Query("SELECT COUNT(*) FROM pending_inputs") fun countPendingInputs(): Flow<Long>

    @Query("SELECT COUNT(*) FROM tokens") fun countTokens(): Flow<Long>

    @Query("SELECT COUNT(*) FROM token_balances") fun countTokenBalances(): Flow<Long>

    @Query("SELECT COUNT(*) FROM token_history_events")
    fun countTokenHistoryEvents(): Flow<Long>

    @Query("SELECT COUNT(*) FROM platform_addresses") fun countPlatformAddresses(): Flow<Long>

    @Query("SELECT COUNT(*) FROM platform_addresses_sync_states")
    fun countPlatformAddressesSyncStates(): Flow<Long>

    @Query("SELECT COUNT(*) FROM shielded_notes") fun countShieldedNotes(): Flow<Long>

    @Query("SELECT COUNT(*) FROM shielded_outgoing_notes")
    fun countShieldedOutgoingNotes(): Flow<Long>

    @Query("SELECT COUNT(*) FROM shielded_activities") fun countShieldedActivities(): Flow<Long>

    @Query("SELECT COUNT(*) FROM shielded_sync_states") fun countShieldedSyncStates(): Flow<Long>

    @Query("SELECT COUNT(*) FROM shielded_viewing_keys") fun countShieldedViewingKeys(): Flow<Long>

    @Query("SELECT COUNT(*) FROM wallet_manager_metadata")
    fun countWalletManagerMetadata(): Flow<Long>
}
