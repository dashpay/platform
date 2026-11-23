package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.PlatformWalletPersistenceHandler
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.persistence.toHex
import org.dashfoundation.dashsdk.security.MasternodeVotingKeyStore
import org.dashfoundation.dashsdk.security.WalletStorage

/**
 * What [PlatformWalletManager.removeIdentityFromDevice] could not delete after
 * the identity's rows were gone. Empty [keysLeft] means everything was removed.
 *
 * @property keysLeft names of the stored secrets the secure storage refused to
 *   delete: [IDENTITY_KEYS] and/or [VOTING_KEY]. They stay in storage with no
 *   row naming them.
 */
data class IdentityDeviceRemovalResult(val keysLeft: List<String>) {
    companion object {
        /** The identity's private keys could not be deleted. */
        const val IDENTITY_KEYS = "identity private keys"

        /** The masternode voting key could not be deleted. */
        const val VOTING_KEY = "voting key"
    }
}

/**
 * Remove [identityId] from this device: its Room rows first, its stored
 * secrets after. Port of `IdentitiesContentView.removeIdentityLocally`
 * (SwiftExampleApp, `Core/Views/IdentitiesContentView.swift`) and the
 * `PersistentIdentity.deleteChildrenFirst` order it uses. Nothing is
 * broadcast; the identity stays on Platform. Backs
 * [PlatformWalletManager.removeIdentityFromDevice], and is a free function so
 * the Robolectric tests can run it without a native manager.
 *
 * Order, so a failure part way never leaves a row that names a deleted key:
 *  1. Under the persister's callback exclusion and the private-key exclusion,
 *     read the identity's public keys while their rows still exist.
 *  2. Delete the rows in one Room transaction
 *     ([PlatformWalletPersistenceHandler.deleteIdentityDataLocked]). If it
 *     fails, nothing was deleted and the error propagates.
 *  3. Delete the private key of each public key that no other identity row
 *     references and no other wallet's owner index claims, in one atomic
 *     secure-storage write. A refusal is reported in
 *     [IdentityDeviceRemovalResult.keysLeft], not thrown: the rows are gone.
 *  4. Outside both exclusions (the voting store takes its own lock), delete
 *     every voting key entry of the identity
 *     ([MasternodeVotingKeyStore.removeStaleEntries], which keeps none once
 *     the row is gone). A failure is reported the same way.
 *
 * Removing an identity that has no row still runs step 4, so a removal that
 * left a voting key behind can be repeated.
 *
 * Not done here: the identity stays in the running Rust wallet, and a later
 * DashPay sync can store it again (dashpay/platform#6098, shared with iOS).
 */
internal suspend fun deleteIdentityFromDevice(
    identityId: ByteArray,
    persistenceHandler: PlatformWalletPersistenceHandler,
    database: DashDatabase,
    walletStorage: WalletStorage,
    votingKeys: MasternodeVotingKeyStore = MasternodeVotingKeyStore(walletStorage),
    // Test seam: the one atomic delete of step 3.
    deleteKeys: suspend WalletStorage.PrivateKeyExclusion.(Collection<String>) -> Unit =
        { deletePrivateKeys(it) },
): IdentityDeviceRemovalResult {
    require(identityId.size == 32) { "identityId must be 32 bytes, got ${identityId.size}" }
    val keysLeft = mutableListOf<String>()
    val identityBase58 = identityId.toBase58String()

    persistenceHandler.withCallbackExclusion {
        walletStorage.withPrivateKeyExclusion {
            val row = database.identityDao().getByIdentityId(identityId)
                ?: return@withPrivateKeyExclusion
            val publicKeys = database.publicKeyDao().getByIdentityId(identityBase58)

            persistenceHandler.deleteIdentityDataLocked(identityId)

            try {
                // A wallet-less identity (a manual load) owns no index entry,
                // so every wallet's claim keeps a key.
                val ownerWalletId = row.walletId ?: ByteArray(32)
                val toDelete = publicKeys
                    .map { it.publicKeyData }
                    .distinctBy { it.toHex() }
                    .filter { data ->
                        database.publicKeyDao()
                            .countReferencesOutsideIdentities(data, listOf(identityBase58)) == 0 &&
                            !isOwnedByAnotherWallet(data.toHex(), ownerWalletId)
                    }
                    .map { it.toHex() }
                deleteKeys(toDelete)
            } catch (e: CancellationException) {
                throw e
            } catch (_: Exception) {
                keysLeft += IdentityDeviceRemovalResult.IDENTITY_KEYS
            }
        }
    }

    if (!votingKeys.removeStaleEntries(database, identityId)) {
        keysLeft += IdentityDeviceRemovalResult.VOTING_KEY
    }
    return IdentityDeviceRemovalResult(keysLeft)
}
