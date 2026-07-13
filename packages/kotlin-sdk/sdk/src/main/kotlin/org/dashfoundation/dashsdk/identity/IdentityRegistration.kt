package org.dashfoundation.dashsdk.identity

import org.dashfoundation.dashsdk.wallet.op

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.credits.FundingInput
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
class IdentityRegistration internal constructor(
    private val gate: org.dashfoundation.dashsdk.wallet.TeardownGate? = null,
) {

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
     * Derive the full identity-registration key SET for a single identity
     * — keyId 0..[count] at the fixed [identityIndex]. This is the call
     * the create-identity flow uses: it fixes the identity index and walks
     * the key index, so a freshly created identity is provisioned with the
     * whole canonical key set (keyId 0 MASTER/AUTH, 1 CRITICAL/AUTH, 2
     * HIGH/AUTH, 3 TRANSFER/CRITICAL) rather than just the MASTER key.
     *
     * Without the full set, all Platform writes fail validation right
     * after creation: document / DPNS / token / contract transitions need
     * a HIGH-or-CRITICAL AUTHENTICATION key, and credit transfers /
     * withdrawals need the TRANSFER key ("no transfer public key").
     *
     * The per-key DPP role is applied Rust-side by keyId at registration
     * time; each row here carries only the derived keypair. The caller
     * persists each row's private key to the Keystore before registering.
     *
     * @param count number of keys to derive; < 0 uses the canonical
     *   default set (4 keys). ([previewRegistrationKeys], by contrast,
     *   walks the *identity* index at the MASTER slot for the discovery
     *   preview.)
     */
    suspend fun previewRegistrationKeySet(
        walletHandle: Long,
        mnemonicResolverHandle: Long,
        identityIndex: Int,
        count: Int = -1,
    ): List<IdentityKeyPreview> = withContext(Dispatchers.IO) {
        val blob = mapNativeErrors {
            IdentityNative.previewRegistrationKeySet(
                walletHandle,
                mnemonicResolverHandle,
                identityIndex,
                count,
            )
        }
        IdentityKeyPreview.decodeAll(blob)
    }

    /**
     * Register a new identity funded from the wallet's Core balance. The
     * single entry point the registration coordinator's body calls. Keys
     * must already be derived + persisted (see [previewRegistrationKeySet]).
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
    ): ByteArray = gate.op {
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
     * Register a new identity funded by the wallet's already-committed
     * Platform-payment (DIP-17) address balances — the ID-08 path, distinct
     * from [registerWithWalletFunding] (ID-01) which builds a new Core asset
     * lock. Keys must already be derived + persisted (see
     * [previewRegistrationKeySet]); [inputs] are the funding addresses (the
     * greedily-packed balance-carrying Platform-payment addresses). The same
     * [signerHandle] drives both the identity-key and platform-address
     * signing roles. Nonces are auto-fetched Rust-side.
     *
     * @return the 32-byte identity id.
     */
    suspend fun registerFromAddresses(
        walletHandle: Long,
        identityIndex: Int,
        keys: List<IdentityKeyPreview>,
        signerHandle: Long,
        inputs: List<FundingInput>,
    ): ByteArray = gate.op {
        require(identityIndex >= 0) { "identityIndex must be non-negative, got $identityIndex" }
        require(inputs.isNotEmpty()) { "inputs must not be empty" }
        mapNativeErrors {
            IdentityNative.registerIdentityFromAddresses(
                walletHandle,
                identityIndex,
                IdentityKeyPreview.encodeForRegistration(keys),
                signerHandle,
                FundingInput.encode(inputs),
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
    ): String = gate.op {
        mapNativeErrors {
            IdentityNative.registerDpnsName(walletHandle, identityId, label, signerHandle)
        }
    }
}
