package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.persistence.toHex
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.PendingIdentityKeyDelete
import org.dashfoundation.dashsdk.security.WalletStorage

/**
 * What [PlatformWalletManager.removeIdentityFromDevice] did.
 *
 * @property wasManaged whether a live wallet managed the identity and removed
 *   it (Rust's answer); false for an identity the app loaded on its own.
 * @property keysLeft names of the stored secrets the secure storage refused to
 *   delete after the rows were gone: [IDENTITY_KEYS] and/or [VOTING_KEY]. The
 *   identity keys stay recorded as pending, so the next removal (of any
 *   identity) deletes them; a voting key goes when the identity is loaded and
 *   removed again. Empty means everything was removed.
 */
data class IdentityDeviceRemovalResult(
    val wasManaged: Boolean,
    val keysLeft: List<String>,
) {
    companion object {
        /** The identity's private keys could not be deleted. */
        const val IDENTITY_KEYS = "identity private keys"

        /** The masternode voting key could not be deleted. */
        const val VOTING_KEY = "voting key"
    }
}

/**
 * Remove [identityId] from this device: the live wallets first, then its Room
 * rows, then its stored secrets. Port of `IdentitiesContentView.removeIdentityLocally`
 * (SwiftExampleApp, `Core/Views/IdentitiesContentView.swift`) and the
 * rows-before-keys order of `PersistentIdentity.deleteChildrenFirst` (#6084).
 * Nothing is broadcast; the identity stays on Platform. Backs
 * [PlatformWalletManager.removeIdentityFromDevice], and is a free function so
 * the Robolectric tests can run it without a native manager.
 *
 * Order:
 *  1. Durably record the identity's key aliases, with its wallet, as pending
 *     deletes. A refusal below leaves them recorded but still named by rows,
 *     so a later settle keeps them and drops the record.
 *  2. [removeFromLiveWallets]: Rust removes the identity from every wallet
 *     managing it, or refuses (a contact request in flight or unconfirmed,
 *     private payments not restored) and throws, deleting nothing. Accepted,
 *     the persister's `onPersistIdentityRemoval` records the aliases the
 *     rows hold at that moment (keys added since step 1 included) and deletes
 *     the identity's footprint in its round's transaction. Called outside the
 *     persister's callback exclusion, which those callbacks take.
 *  3. Under the callback and private-key exclusions: delete the footprint in
 *     one Room transaction
 *     ([PlatformWalletPersistenceHandler.deleteIdentityDataLocked]; the whole
 *     removal for an identity no wallet managed, a no-op otherwise). If it
 *     fails while the row is still there, the error propagates and the still
 *     listed identity can be removed again; if the row is already gone (Rust's
 *     persisted removal committed), the cleanup goes on. Then
 *     [settlePendingIdentityKeyDeletes]. A refusal there is reported in
 *     [IdentityDeviceRemovalResult.keysLeft] and the record stays.
 *  4. Outside both exclusions (the voting store takes its own lock and must
 *     not run in a transaction), delete every voting key entry of the
 *     identity ([MasternodeVotingKeyStore.removeStaleEntries]).
 */
internal suspend fun deleteIdentityFromDevice(
    identityId: ByteArray,
    persistenceHandler: PlatformWalletPersistenceHandler,
    database: DashDatabase,
    walletStorage: WalletStorage,
    removeFromLiveWallets: suspend (identityId: ByteArray) -> Boolean,
    votingKeys: MasternodeVotingKeyStore = MasternodeVotingKeyStore(walletStorage),
    // Test seam: the one atomic secure-storage write of step 3.
    settleKeys: suspend WalletStorage.PrivateKeyExclusion.(
        delete: Collection<String>,
        settled: Collection<PendingIdentityKeyDelete>,
    ) -> Unit = { delete, settled -> settlePendingIdentityKeyDeletes(delete, settled) },
    // Test seam: the host's footprint pass of step 3.
    hostPass: suspend () -> Unit = { persistenceHandler.deleteIdentityDataLocked(identityId) },
): IdentityDeviceRemovalResult {
    require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }

    // 1. Record the aliases before anything can delete their rows.
    val aliases = currentAliases(database, identityId)
    walletStorage.withPrivateKeyExclusion { recordPendingIdentityKeyDeletes(aliases) }

    // 2. Live wallets; a refusal throws before anything local changes.
    val wasManaged = removeFromLiveWallets(identityId)

    // 3. Rows, then identity keys.
    val keysLeft = mutableListOf<String>()
    persistenceHandler.withCallbackExclusion {
        walletStorage.withPrivateKeyExclusion {
            // Record the aliases the rows hold now, under both exclusions, so a
            // key persisted after step 1 is covered when the host pass is the
            // one deleting the rows (an identity no wallet managed).
            recordPendingIdentityKeyDeletes(currentAliases(database, identityId))
            try {
                hostPass()
            } catch (e: CancellationException) {
                throw e
            } catch (e: Exception) {
                // Still listed: nothing of the footprint went with this
                // transaction, and removing again retries (Rust, having
                // removed it already, answers "not managed"). Gone: Rust's
                // persisted removal already deleted the whole footprint in its
                // own round, so only this redundant pass failed; the keys and
                // the voting key are still to clean up below.
                if (database.identityDao().getByIdentityId(identityId) != null) throw e
            }
            try {
                settlePendingIdentityKeyDeletes(database, this, settleKeys)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                keysLeft += IdentityDeviceRemovalResult.IDENTITY_KEYS
            }
        }
    }

    // 4. Voting key.
    if (!votingKeys.removeStaleEntries(database, identityId)) {
        keysLeft += IdentityDeviceRemovalResult.VOTING_KEY
    }
    return IdentityDeviceRemovalResult(wasManaged, keysLeft)
}

/**
 * Settle every recorded identity-key delete, alias by alias. For one alias:
 * each recorded wallet that no longer has a `public_keys` row for it (an
 * identity with no wallet link counts for every wallet) first drops its
 * owner-index claim; then the alias is deleted when no row names it and no
 * wallet still claims it, and kept otherwise (a surviving identity or a
 * pending registration needs it). Every entry is settled in the same atomic
 * write as the deletes; on a failure nothing is settled and the next call
 * retries. The caller holds the private-key exclusion ([scope]) and the
 * persister's callback exclusion, and has committed the row deletes.
 */
internal suspend fun settlePendingIdentityKeyDeletes(
    database: DashDatabase,
    scope: WalletStorage.PrivateKeyExclusion,
    settleKeys: suspend WalletStorage.PrivateKeyExclusion.(
        delete: Collection<String>,
        settled: Collection<PendingIdentityKeyDelete>,
    ) -> Unit = { delete, settled -> settlePendingIdentityKeyDeletes(delete, settled) },
) {
    val pending = scope.pendingIdentityKeyDeletes()
    if (pending.isEmpty()) return
    val delete = mutableListOf<String>()
    for ((alias, entries) in pending.groupBy { it.aliasHex }) {
        val data = alias.hexToBytes()
        for (wallet in entries.mapNotNull { it.walletId }.distinctBy { it.toHex() }) {
            if (database.publicKeyDao().countReferencesForWallet(data, wallet) == 0) {
                scope.releaseOwnerClaims(wallet, listOf(alias))
            }
        }
        val referenced = database.publicKeyDao().countReferences(data) > 0
        val claimed = scope.isOwnedByAnotherWallet(alias, NO_WALLET)
        if (!referenced && !claimed) delete += alias
    }
    scope.settleKeys(delete, pending)
}

/** The identity's key aliases its rows hold now, each with the identity's wallet. */
private suspend fun currentAliases(
    database: DashDatabase,
    identityId: ByteArray,
): List<PendingIdentityKeyDelete> {
    val walletId = database.identityDao().getByIdentityId(identityId)?.walletId
    return database.publicKeyDao().getByIdentityId(identityId.toBase58String())
        .map { PendingIdentityKeyDelete(it.publicKeyData.toHex(), walletId) }
}

/** Stands for "no wallet" in an owner-index check: every wallet's claim counts. */
private val NO_WALLET = ByteArray(32)

private fun String.hexToBytes(): ByteArray =
    ByteArray(length / 2) { substring(it * 2, it * 2 + 2).toInt(16).toByte() }
