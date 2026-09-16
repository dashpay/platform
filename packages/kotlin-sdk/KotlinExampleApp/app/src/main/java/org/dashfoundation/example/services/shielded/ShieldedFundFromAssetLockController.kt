package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Per-slot state for a single "shield funds from an asset lock" attempt —
 * 1:1 port of `ShieldedFundFromAssetLockController.swift`.
 *
 * One controller is created per `(walletId, recipientRaw43)` slot — where
 * `recipientRaw43` is the 43-byte raw Orchard payment address — when the
 * user submits the shielded fund flow. It owns the in-flight [Job],
 * exposes its [phase] via a [StateFlow], and survives view dismissal via
 * [ShieldedFundFromAssetLockCoordinator].
 *
 * The shielded body returns no balance (the Orchard note arrives on the
 * next shielded sync pass, not synchronously), so [Phase.Completed] carries
 * no payload. The 5-step build → IS/CL → shield progress is read from the
 * matching Room `AssetLockEntity` rows (fundingType shielded top-up).
 */
class ShieldedFundFromAssetLockController(
    val walletId: ByteArray,
    /** 43-byte raw Orchard payment address (11-byte diversifier + 32-byte pk_d). */
    val recipientRaw43: ByteArray,
    private val scope: CoroutineScope,
    private val now: () -> Long = System::currentTimeMillis,
) {
    /**
     * Funding phase. Flow: [Idle] → [InFlight] (submit) →
     * [Completed] | [Failed]. ← the Swift `Phase` enum, case-for-case.
     */
    sealed interface Phase {
        /** Pre-submit; controller exists but [submit] hasn't fired. */
        data object Idle : Phase

        /** FFI shield call in flight; asset-lock progress is read from Room. */
        data object InFlight : Phase

        /** Shielded; the note arrives via the next shielded sync pass. */
        data object Completed : Phase

        /** Failure terminal state; the message is shown inline. Stays until dismissed. */
        data class Failed(val message: String) : Phase

        /**
         * Whether the controller is currently holding its slot. Only
         * [InFlight] holds it (used to hide orphan resumable locks whose
         * slot is mid-flight). ← Swift `isActive`.
         */
        val isActive: Boolean
            get() = this is InFlight
    }

    private val _phase = MutableStateFlow<Phase>(Phase.Idle)
    val phase: StateFlow<Phase> = _phase.asStateFlow()

    /** Timestamp (millis) of the most recent [submit] — drives the coordinator's TTL. */
    var lastSubmittedAt: Long? = null
        private set

    /**
     * Identity of the operation currently occupying this slot — the
     * fresh-shield marker or the resumed lock's outpoint. Set by every
     * accepted [submit]. The coordinator compares it to tell a re-tap of
     * the SAME operation (reuse the controller) from a DIFFERENT
     * operation on the same `(walletId, recipient)` slot, whose body
     * [submit] would otherwise silently drop (two resumable locks
     * normally share the wallet's default shielded recipient).
     */
    var operationId: String? = null
        private set

    private var task: Job? = null

    /** Composite id for stable list diffing — wallet hex + recipient hex. */
    val slotRowId: String
        get() = walletId.toHex() + "-" + recipientRaw43.toHex()

    /**
     * Submit the funding. Defensively rejects [Phase.InFlight] and
     * [Phase.Completed]; [Phase.Idle] / [Phase.Failed] are allowed
     * restarts. [operationId] names the operation the body performs (see
     * [operationId]); [body] performs the FFI shield call (returns
     * nothing) or throws; it runs on its own dispatcher, the terminal
     * flip hops back to [scope]. ← Swift `submit`.
     */
    fun submit(operationId: String, body: suspend () -> Unit) {
        when (_phase.value) {
            is Phase.Idle, is Phase.Failed -> Unit
            is Phase.InFlight, is Phase.Completed -> return
        }
        this.operationId = operationId
        _phase.value = Phase.InFlight
        lastSubmittedAt = now()
        task = scope.launch {
            try {
                body()
                _phase.value = Phase.Completed
            } catch (e: Throwable) {
                _phase.value = Phase.Failed(e.message ?: "Shield failed")
            }
        }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
