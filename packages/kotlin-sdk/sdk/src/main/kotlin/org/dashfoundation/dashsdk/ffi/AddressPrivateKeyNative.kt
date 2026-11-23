package org.dashfoundation.dashsdk.ffi

/**
 * Raw JNI surface for revealing a core address's private key: mirrors
 * `rs-unified-sdk-jni/src/address_private_key.rs`, one export per
 * `platform-wallet-ffi` function.
 *
 * Internal: the public API is
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.coreAddressPrivateKey].
 * Rust zeroes its copies of the key; the returned array is the caller's to zero.
 */
internal object AddressPrivateKeyNative {

    /**
     * The private key of [address], one of the wallet's tracked core addresses,
     * as `u8 hexLen ‖ hex ‖ u8 wifLen ‖ wif` (ASCII). Bridges
     * `platform_wallet_address_private_key` (Swift
     * `ManagedPlatformWallet.coreAddressPrivateKey(address:)`).
     *
     * @param walletHandle the native `PlatformWallet` handle.
     * @param resolverHandle the manager's `MnemonicResolverHandle`; Rust reads
     *   the mnemonic through it only when the wallet has no resident keys.
     * @throws DashSDKException with the platform-wallet error when the wallet
     *   does not track [address], holds no key for it, or the derive fails.
     */
    external fun revealCoreAddressPrivateKey(
        walletHandle: Long,
        resolverHandle: Long,
        address: String,
    ): ByteArray
}
