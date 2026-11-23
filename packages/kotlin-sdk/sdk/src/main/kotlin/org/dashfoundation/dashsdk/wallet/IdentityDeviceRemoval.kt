package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.persistence.toHex
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
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
 *  1. Read the identity's row and public keys while they exist: an accepted
 *     removal in Rust emits `onPersistIdentityRemoval`, whose cascade deletes
 *     the key rows.
 *  2. [removeFromLiveWallets]: Rust removes the identity from every wallet
 *     managing it, or refuses (a contact request in flight or unconfirmed,
 *     private payments not restored) and throws. On a refusal nothing on the
 *     device is deleted. Called outside the persister's callback exclusion,
 *     which the callbacks it triggers take.
 *  3. Under the callback and private-key exclusions: durably record the key
 *     aliases as pending deletes; delete the rows in one Room transaction
 *     ([PlatformWalletPersistenceHandler.deleteIdentityDataLocked], idempotent);
 *     then settle every pending delete, this identity's and any an earlier
 *     interrupted removal left. A key some `public_keys` row still names, or
 *     another wallet's owner index claims, is kept; this identity's wallet
 *     drops its own claim on a kept key no other identity of it uses. A
 *     refusal here is reported in [IdentityDeviceRemovalResult.keysLeft] and
 *     the pending record stays for the next removal.
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
        settled: Collection<String>,
    ) -> Unit = { delete, settled -> settlePendingIdentityKeyDeletes(delete, settled) },
): IdentityDeviceRemovalResult {
    require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }
    val identityBase58 = identityId.toBase58String()

    // 1. Snapshot.
    val row = database.identityDao().getByIdentityId(identityId)
    val candidateKeys = database.publicKeyDao().getByIdentityId(identityBase58)
        .map { it.publicKeyData.toHex() }
        .toSet()

    // 2. Live wallets; a refusal throws before anything local changes.
    val wasManaged = removeFromLiveWallets(identityId)

    // 3. Rows, then identity keys.
    val keysLeft = mutableListOf<String>()
    persistenceHandler.withCallbackExclusion {
        walletStorage.withPrivateKeyExclusion {
            val ownerWalletId = row?.walletId
            recordPendingIdentityKeyDeletes(
                candidateKeys.map { PendingKeyDelete(it, ownerWalletId).encode() },
            )
            persistenceHandler.deleteIdentityDataLocked(identityId)
            try {
                val pending = pendingIdentityKeyDeletes()
                val delete = mutableListOf<String>()
                val release = mutableMapOf<String, MutableList<String>>()
                for (entry in pending) {
                    val (hex, wallet) = PendingKeyDelete.decode(entry)
                    val data = hex.hexToBytes()
                    val referenced = database.publicKeyDao()
                        .countReferencesOutsideIdentities(data, listOf(identityBase58)) > 0
                    val claimedElsewhere = isOwnedByAnotherWallet(hex, wallet ?: NO_WALLET)
                    if (!referenced && !claimedElsewhere) {
                        delete += hex
                    } else if (
                        wallet != null &&
                        database.publicKeyDao().countReferencesInWallet(data, wallet) == 0
                    ) {
                        // Kept for another wallet's identity: the removed
                        // identity's wallet no longer needs its claim.
                        release.getOrPut(wallet.toHex()) { mutableListOf() } += hex
                    }
                }
                for ((walletHex, hexes) in release) {
                    releaseOwnerClaims(walletHex.hexToBytes(), hexes)
                }
                settleKeys(delete, pending)
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

/** Stands for "no wallet" in an owner-index check: every wallet's claim counts. */
private val NO_WALLET = ByteArray(32)

/**
 * One pending identity-key delete: the alias and the wallet that owned the
 * removed identity, so a retry after the row is gone still excludes that
 * wallet's own claim. Encoded `<aliasHex>` or `<aliasHex>@<walletIdHex>`.
 */
private data class PendingKeyDelete(val aliasHex: String, val walletId: ByteArray?) {
    fun encode(): String = if (walletId == null) aliasHex else "$aliasHex@${walletId.toHex()}"

    companion object {
        fun decode(entry: String): PendingKeyDelete {
            val at = entry.indexOf('@')
            return if (at < 0) {
                PendingKeyDelete(entry, null)
            } else {
                PendingKeyDelete(entry.substring(0, at), entry.substring(at + 1).hexToBytes())
            }
        }
    }
}

private fun String.hexToBytes(): ByteArray =
    ByteArray(length / 2) { substring(it * 2, it * 2 + 2).toInt(16).toByte() }
