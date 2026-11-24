package org.dashfoundation.example.services

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.identity.IdentityUpdates
import org.dashfoundation.dashsdk.identity.KeyDisableRefusal
import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity

/**
 * Client-side safety gate for the Disable Key action — 1:1 port of
 * `KeyDisableGate.swift`.
 *
 * A pre-flight UI guard that decides nothing itself. Which keys may be
 * disabled is the wallet's call (`IdentityUpdates.checkKeysCanBeDisabled`:
 * the last valid authentication key of a security level under DIP-11, and
 * the last enabled transfer key), judged in Rust and handed in as a
 * [RuleVerdict]. The wallet's identity-update entry points
 * refuse the same updates, so the gate only spares the user a doomed attempt
 * and turns the structured refusal into copy.
 *
 * Operates on Room [PublicKeyEntity] rows (the nullable `disabledAt` the
 * persister writes).
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

    /** Rust's verdict for disabling the target key alone. ← Swift `RuleVerdict`. */
    sealed interface RuleVerdict {
        /** The wallet allows disabling the key. */
        data object Allowed : RuleVerdict

        /** The wallet refuses it, for the reason [refusal] names. */
        data class Refused(val refusal: KeyDisableRefusal) : RuleVerdict

        /**
         * No verdict: the wallet is not loaded, or the check failed or is
         * still running. [reason] is user-facing copy.
         */
        data class Unavailable(val reason: String) : RuleVerdict
    }

    /**
     * Ask Rust whether the wallet lets [keyId] of [identityId] be disabled.
     * Without a loaded wallet, or when the check fails, there is no verdict
     * and the gate forbids the action. ← Swift
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
     * Evaluate the disable gate for [target], given Rust's [ruleVerdict] for
     * disabling it alone.
     */
    fun evaluate(target: PublicKeyEntity, ruleVerdict: RuleVerdict): Evaluation {
        if (target.disabledAt != null) return Evaluation.AlreadyDisabled

        return when (ruleVerdict) {
            RuleVerdict.Allowed -> Evaluation.Allowed
            is RuleVerdict.Unavailable -> Evaluation.Forbidden(ruleVerdict.reason)
            is RuleVerdict.Refused -> Evaluation.Forbidden(reason(ruleVerdict.refusal))
        }
    }

    /** User-facing copy for a [refusal]. */
    private fun reason(refusal: KeyDisableRefusal): String = when (refusal) {
        is KeyDisableRefusal.LastAuthenticationKey ->
            "No other valid authentication key at the ${refusal.securityLevel.name} level " +
                "would remain. Disabling this key would leave the identity unable to sign " +
                "what needs that level — add another key first (Add Key)."
        is KeyDisableRefusal.LastEnabledKey -> {
            val purpose = refusal.purpose.name.lowercase()
            "No other enabled $purpose key able to sign would remain. Disabling this key " +
                "would break credit withdrawals and transfers — add another $purpose key " +
                "first (Add Key)."
        }
    }
}
