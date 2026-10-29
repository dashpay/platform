package org.dashfoundation.example.services

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.identity.IdentityUpdates
import org.dashfoundation.dashsdk.identity.KeyDisableRefusal
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity

/**
 * Client-side safety gate for the Disable Key action — 1:1 port of
 * `KeyDisableGate.swift`.
 *
 * A pre-flight UI guard. It decides nothing about DIP-11 itself:
 * - **Last valid key of a security level (DIP-11):** Rust judges it
 *   (`IdentityUpdates.checkKeysCanBeDisabled`, dpp's
 *   `check_keys_can_be_disabled` at the device clock) and the caller hands
 *   the verdict in as [RuleVerdict]. The identity update builder refuses the
 *   same updates before signing, so the gate only spares the user a doomed
 *   attempt.
 * - **Last transfer key (client-side self-brick guard):** not a protocol or
 *   DIP rule, and not in Rust — drive-abci would accept it — but disabling
 *   the last enabled transfer key would break credit withdrawals, so we
 *   refuse locally.
 *
 * Operates on Room [PublicKeyEntity] rows (the string-valued `purpose` and
 * nullable `disabledAt` the persister writes).
 */
object KeyDisableGate {

    /** Result of evaluating whether a key may be disabled. ← Swift `Evaluation`. */
    sealed interface Evaluation {
        /** The key is already disabled on-chain — no action to take. */
        data object AlreadyDisabled : Evaluation

        /** The key may be disabled. */
        data object Allowed : Evaluation

        /** The key may not be disabled; [reason] is user-facing copy. */
        data class Forbidden(val reason: String) : Evaluation
    }

    /** Rust's DIP-11 verdict for disabling the target key alone. ← Swift `RuleVerdict`. */
    sealed interface RuleVerdict {
        /** DIP-11 allows disabling the key. */
        data object Allowed : RuleVerdict

        /** DIP-11 forbids it: [refusal] names the level left without a valid key. */
        data class Refused(val refusal: KeyDisableRefusal) : RuleVerdict

        /**
         * No verdict: the wallet is not loaded, or the check failed or is
         * still running. [reason] is user-facing copy.
         */
        data class Unavailable(val reason: String) : RuleVerdict
    }

    /**
     * Ask Rust whether [keyId] of [identityId] may be disabled under DIP-11
     * at the device clock. Without a loaded wallet, or when the check fails,
     * there is no verdict and the gate forbids the action. ← Swift
     * `ruleVerdict(wallet:identityId:keyId:)`.
     */
    suspend fun ruleVerdict(
        identityUpdates: IdentityUpdates?,
        walletHandle: Long?,
        identityId: ByteArray,
        keyId: Int,
    ): RuleVerdict {
        if (identityUpdates == null || walletHandle == null) {
            return RuleVerdict.Unavailable(
                "Identity not attached to a loaded wallet — cannot check or sign the disable " +
                    "transition.",
            )
        }
        return try {
            identityUpdates.checkKeysCanBeDisabled(walletHandle, identityId, listOf(keyId))
                ?.let { RuleVerdict.Refused(it) }
                ?: RuleVerdict.Allowed
        } catch (e: CancellationException) {
            throw e
        } catch (e: Exception) {
            RuleVerdict.Unavailable(
                e.message ?: "Could not check whether this key may be disabled.",
            )
        }
    }

    /**
     * Evaluate the disable gate for [target] within [allKeys] (the
     * identity's current public keys), given Rust's [ruleVerdict] for
     * disabling [target] alone. [target] must be one of [allKeys].
     */
    fun evaluate(
        target: PublicKeyEntity,
        allKeys: List<PublicKeyEntity>,
        ruleVerdict: RuleVerdict,
    ): Evaluation {
        if (target.disabledAt != null) return Evaluation.AlreadyDisabled

        when (ruleVerdict) {
            is RuleVerdict.Refused -> return Evaluation.Forbidden(
                "No other valid authentication key at the " +
                    "${ruleVerdict.refusal.securityLevel.name} level would remain. Disabling " +
                    "this key would leave the identity unable to sign what needs that level — " +
                    "add another key first (Add Key).",
            )
            is RuleVerdict.Unavailable -> return Evaluation.Forbidden(ruleVerdict.reason)
            RuleVerdict.Allowed -> Unit
        }

        if (target.purpose.equals("TRANSFER", ignoreCase = true) &&
            enabledCount("TRANSFER", allKeys) <= 1
        ) {
            return Evaluation.Forbidden(
                "This is the only enabled transfer key. Disabling it would break credit " +
                    "withdrawals — add another transfer key first (Add Key).",
            )
        }

        return Evaluation.Allowed
    }

    /** Count of enabled (non-disabled) keys with the given [purpose]. */
    private fun enabledCount(purpose: String, keys: List<PublicKeyEntity>): Int =
        keys.count { it.purpose.equals(purpose, ignoreCase = true) && it.disabledAt == null }
}
