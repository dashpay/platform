package org.dashfoundation.example.services

import org.dashfoundation.dashsdk.persistence.entities.PublicKeyEntity

/**
 * Client-side safety gate for the Disable Key action — 1:1 port of
 * `KeyDisableGate.swift`.
 *
 * A pre-flight UI guard, not protocol logic. It mixes two kinds of refusal:
 * - **Master key (consensus):** disabling a lone master key is rejected by
 *   consensus (`validate_master_key_uniqueness_v0`); refusing locally just
 *   avoids spending fees on a transition drive-abci will reject.
 * - **Last auth / last transfer (client-side self-brick guards):** not
 *   protocol invariants — drive-abci would accept them — but disabling the
 *   last enabled authentication or transfer key would strand the identity,
 *   so we refuse locally.
 *
 * Operates on Room [PublicKeyEntity] rows (the string-valued `purpose` /
 * `securityLevel` and nullable `disabledAt` the persister writes), so the
 * comparisons match the DPP enum names verbatim.
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

    /**
     * Evaluate the disable gate for [target] within [allKeys] (the
     * identity's current public keys). [target] must be one of them.
     */
    fun evaluate(target: PublicKeyEntity, allKeys: List<PublicKeyEntity>): Evaluation {
        if (target.disabledAt != null) return Evaluation.AlreadyDisabled

        if (target.securityLevel.equals("MASTER", ignoreCase = true)) {
            return Evaluation.Forbidden(
                "Master keys can't be disabled here — a master-key disable is rejected by " +
                    "consensus. Master-key rotation is out of scope.",
            )
        }

        if (target.purpose.equals("AUTHENTICATION", ignoreCase = true) &&
            enabledCount("AUTHENTICATION", allKeys) <= 1
        ) {
            return Evaluation.Forbidden(
                "This is the only enabled authentication key. Disabling it would leave the " +
                    "identity unable to sign — add another authentication key first (Add Key).",
            )
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
