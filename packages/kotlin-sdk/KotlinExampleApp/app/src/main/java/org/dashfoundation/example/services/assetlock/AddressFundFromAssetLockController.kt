package org.dashfoundation.example.services.assetlock

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Per-slot state for a single "fund a Platform address from an asset lock"
 * attempt — 1:1 port of `AddressFundFromAssetLockController.swift`.
 *
 * One controller is created per
 * `(walletId, platformAccountIndex, recipientHash)` slot when the user
 * submits the fund-from-asset-lock flow. It owns the in-flight [Job],
 * exposes its current [phase] via a [StateFlow], and survives view
 * dismissal via [AddressFundFromAssetLockCoordinator] on the app
 * container. Multiple controllers can be active simultaneously (one per
 * slot).
 *
 * Unlike identity registration there is no "unconfirmed" terminal — the
 * asset-lock funding either credits the address ([Phase.Completed] with
 * the proof-attested balance) or fails; the funding step-set + IS/CL proof
 * progress is read from the matching Room `AssetLockEntity` rows, not from
 * this controller.
 */
class AddressFundFromAssetLockController(
    val walletId: ByteArray,
    val platformAccountIndex: Int,
    val recipientHash: ByteArray,
    /** 0 = P2PKH, 1 = P2SH. Carried for the FFI recipient-entry encoding. */
    val recipientType: Int,
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

        /** FFI funding call in flight; the asset-lock progress is read from Room. */
        data object InFlight : Phase

        /** Address credited; [newBalance] is the proof-attested credit balance. */
        data class Completed(val newBalance: Long) : Phase

        /** Failure terminal state; the message is shown inline. Stays until dismissed. */
        data class Failed(val message: String) : Phase

        /**
         * Whether the controller is currently holding its slot against a
         * fresh funding. Only [InFlight] holds it. ← Swift `isActive`.
         */
        val isActive: Boolean
            get() = this is InFlight
    }

    private val _phase = MutableStateFlow<Phase>(Phase.Idle)
    val phase: StateFlow<Phase> = _phase.asStateFlow()

    /** Timestamp (millis) of the most recent [submit] — drives the coordinator's TTL. */
    var lastSubmittedAt: Long? = null
        private set

    private var task: Job? = null

    /** Composite id for stable list diffing — wallet hex + account + recipient hex. */
    val slotRowId: String
        get() = walletId.toHex() + "-" + platformAccountIndex + "-" + recipientHash.toHex()

    /**
     * Submit the funding. Defensively rejects [Phase.InFlight] and
     * [Phase.Completed] (re-firing would race a duplicate FFI call or
     * re-fund the address). [Phase.Idle] and [Phase.Failed] are allowed —
     * the coordinator drives the legitimate restart through them.
     * ← Swift `submit`.
     *
     * [body] performs the actual FFI call and returns the new credit
     * balance, or throws. It runs on its own dispatcher; the terminal
     * phase flip hops back to [scope].
     */
    fun submit(body: suspend () -> Long) {
        when (_phase.value) {
            is Phase.Idle, is Phase.Failed -> Unit
            is Phase.InFlight, is Phase.Completed -> return
        }
        _phase.value = Phase.InFlight
        lastSubmittedAt = now()
        task = scope.launch {
            try {
                val newBalance = body()
                _phase.value = Phase.Completed(newBalance)
            } catch (e: Throwable) {
                _phase.value = Phase.Failed(e.message ?: "Funding failed")
            }
        }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
