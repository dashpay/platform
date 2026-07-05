package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.IdentityNative

/**
 * Thin wrapper over the identity JNI surface (`IdentityNative`) — port of
 * the identity-registration slice of `ManagedPlatformWallet.swift`.
 *
 * Every method is a single FFI call marshalled onto [Dispatchers.IO] and
 * routed through [mapNativeErrors]; no orchestration lives here (see
 * `packages/kotlin-sdk/CLAUDE.md`). The handles are supplied by the caller
 * — `PlatformWalletManager` owns the `signerHandle` /
 * `mnemonicResolverHandle`, and `ManagedPlatformWallet` owns the wallet
 * handle.
 */
class IdentityRegistration internal constructor() {

    /**
     * Derive the identity-registration keys for a wallet slot (path +
     * public key + private scalar). Pure compute — no Platform RPCs. The
     * caller persists each row's private key to the Keystore before
     * registering; see `IdentityKeyPreview`.
     *
     * @param count number of consecutive slots to derive; < 0 uses the
     *   Rust gap-limit default.
     */
    suspend fun previewRegistrationKeys(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        startIndex: Int,
        count: Int = -1,
    ): List<IdentityKeyPreview> = withContext(Dispatchers.IO) {
        val blob = mapNativeErrors {
            IdentityNative.previewRegistrationKeys(
                walletHandle,
                mnemonicResolverHandle,
                startIndex,
                count,
            )
        }
        IdentityKeyPreview.decodeAll(blob)
    }

    /**
     * Register a new identity funded from the wallet's Core balance. The
     * single entry point the registration coordinator's body calls. Keys
     * must already be derived + persisted (see [previewRegistrationKeys]).
     *
     * @return the 32-byte identity id.
     */
    suspend fun registerWithWalletFunding(
        walletHandle: Long,
        amountDuffs: Long,
        accountIndex: Int,
        identityIndex: Int,
        keys: List<IdentityKeyPreview>,
        signerHandle: Long,
        coreSignerHandle: Long,
    ): ByteArray = withContext(Dispatchers.IO) {
        require(amountDuffs > 0) { "amountDuffs must be positive, got $amountDuffs" }
        require(accountIndex >= 0) { "accountIndex must be non-negative, got $accountIndex" }
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        mapNativeErrors {
            IdentityNative.registerIdentityWithFunding(
                walletHandle,
                amountDuffs,
                accountIndex,
                identityIndex,
                IdentityKeyPreview.encodeForRegistration(keys),
                signerHandle,
                coreSignerHandle,
            )
        }
    }

    /**
     * Scan the wallet for registered identities (gap-limit walk). Returns
     * the discovered 32-byte identity ids.
     *
     * @param startIndex first slot to probe; < 0 uses the Rust default.
     */
    suspend fun discoverIdentities(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        startIndex: Int = -1,
        gapLimit: Int = 5,
    ): List<ByteArray> = withContext(Dispatchers.IO) {
        val flat = mapNativeErrors {
            IdentityNative.discoverIdentities(
                walletHandle,
                mnemonicResolverHandle,
                startIndex,
                gapLimit,
            )
        }
        flat.asList().chunked(32) { it.toByteArray() }.filter { it.size == 32 }
    }

    /**
     * Register a DPNS name for [identityId] (32 bytes), signed via
     * [signerHandle]. Returns the full domain name (e.g. `"alice.dash"`).
     */
    suspend fun registerDpnsName(
        walletHandle: Long,
        identityId: ByteArray,
        label: String,
        signerHandle: Long,
    ): String = withContext(Dispatchers.IO) {
        mapNativeErrors {
            IdentityNative.registerDpnsName(walletHandle, identityId, label, signerHandle)
        }
    }
}
