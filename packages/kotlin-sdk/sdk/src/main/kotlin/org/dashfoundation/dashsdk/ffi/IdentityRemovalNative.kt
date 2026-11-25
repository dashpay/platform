package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for removing an identity from the device's live wallets:
 * mirrors `rs-unified-sdk-jni/src/identity_removal.rs`, one export per
 * `platform-wallet-ffi` function.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.removeIdentityFromDevice].
 */
internal object IdentityRemovalNative {

    /**
     * Remove [identityId] (32 bytes) from every wallet of the manager that
     * manages it. Bridges `platform_wallet_manager_remove_identity_from_device`.
     *
     * @return true when a wallet managed and removed it, false when none did.
     * @throws DashSDKException with the platform-wallet code on a refusal (a
     *   contact request in flight or unconfirmed, private payments not
     *   restored), an unknown manager handle, or an id that is not 32 bytes.
     *   Nothing is removed then.
     */
    external fun removeIdentityFromDevice(managerHandle: Long, identityId: ByteArray): Boolean
}
