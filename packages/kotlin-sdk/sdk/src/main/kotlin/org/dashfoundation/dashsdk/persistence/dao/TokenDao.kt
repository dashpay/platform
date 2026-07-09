package org.dashfoundation.dashsdk.persistence.dao

import androidx.room.Dao
import androidx.room.Delete
import androidx.room.Insert
import androidx.room.Query
import androidx.room.Update
import androidx.room.Upsert
import kotlinx.coroutines.flow.Flow
import org.dashfoundation.dashsdk.persistence.entities.TokenBalanceEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.dashsdk.persistence.entities.TokenHistoryEventEntity

/**
 * Queries over the token family — [TokenEntity], [TokenBalanceEntity],
 * [TokenHistoryEventEntity].
 *
 * Token capability reads mirror the `PersistentToken` predicate helpers
 * (`mintableTokensPredicate`, `burnableTokensPredicate`,
 * `freezableTokensPredicate`, `distributionTokensPredicate`,
 * `pausedTokensPredicate`, `tokensByContractPredicate`,
 * `tokensWithControlRulePredicate`) via the persisted capability
 * columns; [observeTokensByNetwork] mirrors the example app's
 * `token.dataContract?.networkRaw == target` join. Balance reads mirror
 * every `PersistentTokenBalance` predicate helper.
 */
@Dao
interface TokenDao {

    // MARK: Tokens

    @Query("SELECT * FROM tokens WHERE id = :id")
    fun observeTokenById(id: ByteArray): Flow<TokenEntity?>

    @Query("SELECT * FROM tokens WHERE id = :id")
    suspend fun getTokenById(id: ByteArray): TokenEntity?

    /** Mirror of `tokensByContractPredicate(contractId:)`. */
    @Query("SELECT * FROM tokens WHERE contractId = :contractId ORDER BY position")
    fun observeTokensByContract(contractId: ByteArray): Flow<List<TokenEntity>>

    /** ContractsTabView / IdentityDetailView: network via the contract join. */
    @Query(
        "SELECT tokens.* FROM tokens INNER JOIN data_contracts " +
            "ON tokens.contractId = data_contracts.id " +
            "WHERE data_contracts.networkRaw = :networkRaw ORDER BY tokens.name"
    )
    fun observeTokensByNetwork(networkRaw: Int): Flow<List<TokenEntity>>

    /** Mirror of `mintableTokensPredicate()` (`manualMintingRules != nil`). */
    @Query("SELECT * FROM tokens WHERE canManuallyMint = 1")
    fun observeMintableTokens(): Flow<List<TokenEntity>>

    /** Mirror of `burnableTokensPredicate()` (`manualBurningRules != nil`). */
    @Query("SELECT * FROM tokens WHERE canManuallyBurn = 1")
    fun observeBurnableTokens(): Flow<List<TokenEntity>>

    /** Mirror of `freezableTokensPredicate()` (`freezeRules != nil`). */
    @Query("SELECT * FROM tokens WHERE canFreeze = 1")
    fun observeFreezableTokens(): Flow<List<TokenEntity>>

    /** Mirror of `tokensWithControlRulePredicate(rule: .unfreeze)`. */
    @Query("SELECT * FROM tokens WHERE canUnfreeze = 1")
    fun observeUnfreezableTokens(): Flow<List<TokenEntity>>

    /** Mirror of `tokensWithControlRulePredicate(rule: .destroyFrozenFunds)`. */
    @Query("SELECT * FROM tokens WHERE canDestroyFrozenFunds = 1")
    fun observeDestroyFrozenFundsTokens(): Flow<List<TokenEntity>>

    /** Mirror of `tokensWithControlRulePredicate(rule: .emergencyAction)`. */
    @Query("SELECT * FROM tokens WHERE hasEmergencyActions = 1")
    fun observeEmergencyActionTokens(): Flow<List<TokenEntity>>

    /** Mirror of `tokensWithControlRulePredicate(rule: .conventions)`. */
    @Query("SELECT * FROM tokens WHERE canChangeConventions = 1")
    fun observeConventionsChangeTokens(): Flow<List<TokenEntity>>

    /** Mirror of `tokensWithControlRulePredicate(rule: .maxSupply)`. */
    @Query("SELECT * FROM tokens WHERE canChangeMaxSupply = 1")
    fun observeMaxSupplyChangeTokens(): Flow<List<TokenEntity>>

    /** Mirror of `distributionTokensPredicate()`. */
    @Query("SELECT * FROM tokens WHERE hasDistribution = 1")
    fun observeDistributionTokens(): Flow<List<TokenEntity>>

    /** Mirror of `pausedTokensPredicate()`. */
    @Query("SELECT * FROM tokens WHERE isPaused = 1")
    fun observePausedTokens(): Flow<List<TokenEntity>>

    @Upsert
    suspend fun upsertToken(token: TokenEntity)

    @Delete
    suspend fun deleteToken(token: TokenEntity)

    @Query("DELETE FROM tokens")
    suspend fun deleteAllTokens()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM tokens")
    fun countTokens(): Flow<Long>

    // MARK: Token balances

    /** Mirror of `predicate(tokenId:identityId:)` — the upsert key. */
    @Query("SELECT * FROM token_balances WHERE tokenId = :tokenId AND identityId = :identityId")
    suspend fun getBalance(tokenId: String, identityId: ByteArray): TokenBalanceEntity?

    @Query("SELECT * FROM token_balances WHERE tokenId = :tokenId AND identityId = :identityId")
    fun observeBalance(tokenId: String, identityId: ByteArray): Flow<TokenBalanceEntity?>

    /** Mirror of `predicate(identityId:)`. */
    @Query("SELECT * FROM token_balances WHERE identityId = :identityId")
    fun observeBalancesByIdentity(identityId: ByteArray): Flow<List<TokenBalanceEntity>>

    /** Mirror of `predicate(tokenId:)`. */
    @Query("SELECT * FROM token_balances WHERE tokenId = :tokenId")
    fun observeBalancesByToken(tokenId: String): Flow<List<TokenBalanceEntity>>

    /** Mirror of `nonZeroBalancesPredicate`. */
    @Query("SELECT * FROM token_balances WHERE balance > 0")
    fun observeNonZeroBalances(): Flow<List<TokenBalanceEntity>>

    /** Mirror of `frozenBalancesPredicate`. */
    @Query("SELECT * FROM token_balances WHERE frozen = 1")
    fun observeFrozenBalances(): Flow<List<TokenBalanceEntity>>

    /** Mirror of `needsSyncPredicate(olderThan:)`; epoch millis. */
    @Query(
        "SELECT * FROM token_balances WHERE lastSyncedAt IS NULL " +
            "OR lastSyncedAt < :olderThanMillis"
    )
    suspend fun getBalancesNeedingSync(olderThanMillis: Long): List<TokenBalanceEntity>

    @Insert
    suspend fun insertBalance(balance: TokenBalanceEntity): Long

    @Update
    suspend fun updateBalance(balance: TokenBalanceEntity)

    @Delete
    suspend fun deleteBalance(balance: TokenBalanceEntity)

    /** Mirror of the persister's balance-removal pass. */
    @Query("DELETE FROM token_balances WHERE tokenId = :tokenId AND identityId = :identityId")
    suspend fun deleteBalance(tokenId: String, identityId: ByteArray)

    /**
     * Wallet teardown: drop every balance owned by an identity. The
     * `identity` FK is SET_NULL (Swift `.nullify`), so deleting the
     * identity row does NOT cascade its balances — `deleteWalletData`
     * clears them explicitly, mirroring the Swift wipe's balance pass.
     */
    @Query("DELETE FROM token_balances WHERE identityId = :identityId")
    suspend fun deleteBalancesByIdentity(identityId: ByteArray)

    @Query("DELETE FROM token_balances")
    suspend fun deleteAllBalances()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM token_balances")
    fun countBalances(): Flow<Long>

    /** StorageExplorer network-scoped row count. */
    @Query("SELECT COUNT(*) FROM token_balances WHERE networkRaw = :networkRaw")
    fun countBalancesByNetwork(networkRaw: Int): Flow<Long>

    // MARK: Token history events

    /** Events of one token (accessed via the relationship in Swift). */
    @Query(
        "SELECT * FROM token_history_events WHERE tokenRef = :tokenId " +
            "ORDER BY eventTimestamp DESC"
    )
    fun observeHistoryByToken(tokenId: ByteArray): Flow<List<TokenHistoryEventEntity>>

    @Upsert
    suspend fun upsertHistoryEvent(event: TokenHistoryEventEntity)

    @Delete
    suspend fun deleteHistoryEvent(event: TokenHistoryEventEntity)

    @Query("DELETE FROM token_history_events")
    suspend fun deleteAllHistoryEvents()

    /** StorageExplorer row count. */
    @Query("SELECT COUNT(*) FROM token_history_events")
    fun countHistoryEvents(): Flow<Long>
}
