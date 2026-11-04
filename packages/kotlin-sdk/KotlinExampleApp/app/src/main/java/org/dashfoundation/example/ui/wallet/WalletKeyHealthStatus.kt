package org.dashfoundation.example.ui.wallet

/**
 * What the key health sheet shows for one persisted identity key.
 */
internal enum class WalletKeyHealthStatus {
    /** The private key opens on this device. */
    Recoverable,

    /**
     * The private key doesn't open, and the row records the wallet slot the
     * key was derived at, so re-deriving it from the mnemonic can repair it.
     */
    Repairable,

    /**
     * The private key doesn't open, and the row records no wallet slot. The
     * wallet stores no slot for a key it doesn't derive (a Hecate signer key,
     * say), and the repair re-derives at the recorded slot only, so there is
     * nothing to repair: the key is left unchecked rather than called lost.
     */
    NotChecked,
}

/**
 * The status of a persisted key from what the probe and the row say. It
 * reads the slot Rust recorded on the row (`derivationIdentityIndex` and
 * `derivationKeyIndex`, both required, as the repair requires both) and makes
 * no decision about key types.
 */
internal fun walletKeyHealthStatus(
    isRecoverable: Boolean,
    derivationIdentityIndex: Int?,
    derivationKeyIndex: Int?,
): WalletKeyHealthStatus = when {
    isRecoverable -> WalletKeyHealthStatus.Recoverable
    derivationIdentityIndex != null && derivationKeyIndex != null -> WalletKeyHealthStatus.Repairable
    else -> WalletKeyHealthStatus.NotChecked
}
