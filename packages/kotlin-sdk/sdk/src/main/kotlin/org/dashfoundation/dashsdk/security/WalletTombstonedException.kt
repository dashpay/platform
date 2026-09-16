package org.dashfoundation.dashsdk.security

/**
 * Thrown by [WalletStorage.storePrivateKey] / [WalletStorage.storeIfAbsent]
 * when [walletId] was tombstoned by a wallet deletion still in effect (see
 * [WalletStorage.PrivateKeyExclusion.tombstoneWallet]) — the write is
 * rejected rather than silently resurrecting a deleted wallet's owner-index
 * entry with fresh ciphertext.
 */
class WalletTombstonedException(val walletId: ByteArray) : IllegalStateException(
    "wallet ${walletId.joinToString("") { "%02x".format(it) }} was deleted; store rejected",
)
