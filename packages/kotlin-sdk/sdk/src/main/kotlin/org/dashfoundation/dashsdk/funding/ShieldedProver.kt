package org.dashfoundation.dashsdk.funding

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.FundingNative

/**
 * Thin wrapper over the process-global shielded-funding support JNI
 * surface (`FundingNative`) — the Halo 2 prover warm-up / readiness probe
 * that backs the shielded funding screens' prover-status indicator.
 *
 * The shielded fee estimator lives on
 * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.estimateShieldedFee]
 * (it resolves the manager's network-tracked platform version, so it needs
 * the manager handle); only its [FeeKind] selector is declared here.
 *
 * Only meaningful on a shielded build
 * ([org.dashfoundation.dashsdk.Sdk.hasShielded]); the caller must gate on
 * that before use.
 */
object ShieldedProver {

    /**
     * Fee-kind selector for
     * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.estimateShieldedFee].
     */
    enum class FeeKind(val raw: Int) {
        /** ShieldedTransfer / Shield (the base flat fee). */
        TransferOrShield(0),

        /** Unshield (base + the flat balance-to-address output cost). */
        Unshield(1),

        /** ShieldedWithdrawal (base + the flat Core withdrawal-document cost). */
        Withdrawal(2),

        /**
         * ShieldFromIdentity (Type 21): the shielded COMPUTE fee floor
         * (`compute_shielded_verification_fee`: proof verification + per-action
         * processing). Unlike the pool-paid kinds above it carries no storage
         * term: the identity-funded shield meters its writes through GroveDB
         * against the identity balance, so only the compute portion is flat.
         * Backs
         * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedShieldFromIdentity].
         */
        ShieldFromIdentity(3),

        /**
         * IdentityTopUpFromShieldedPool (Type 22): base plus the flat
         * identity-balance write cost, carved from the value balance like
         * the other pool-paid kinds. Backs
         * [org.dashfoundation.dashsdk.wallet.PlatformWalletManager.shieldedIdentityTopUpFromPool].
         */
        IdentityTopUpFromPool(4),
    }

    /** Kick the ~30s Halo 2 proving-key build onto a background thread. Idempotent. */
    suspend fun warmUp() = withContext(Dispatchers.IO) {
        mapNativeErrors { FundingNative.warmUpProver() }
    }

    /** Whether the Halo 2 proving key is already built. */
    suspend fun isReady(): Boolean = withContext(Dispatchers.IO) {
        mapNativeErrors { FundingNative.proverIsReady() }
    }
}
