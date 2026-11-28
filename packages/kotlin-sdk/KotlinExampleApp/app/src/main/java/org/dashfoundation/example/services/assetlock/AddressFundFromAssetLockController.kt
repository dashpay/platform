package org.dashfoundation.example.services.assetlock

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint
import org.dashfoundation.example.util.PaymentFailure

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

        /**
         * Failure terminal state, presented by [PaymentFailure] from the
         * native wallet's class; its action says what may run again (a
         * retry, or only the resume of the held lock). Stays until dismissed.
         */
        data class Failed(val failure: PaymentFailure) : Phase {
            val message: String get() = failure.message
        }

        /**
         * Whether the controller is currently holding its slot against a
         * fresh funding. Only [InFlight] holds it; what may run on a failed
         * slot again is [permitsSubmit]'s to decide. ← Swift `isActive`.
         */
        val isActive: Boolean
            get() = this is InFlight

        /**
         * Whether an operation resuming [resuming] (null for a fresh
         * funding) may start from this phase: from [Idle], and from
         * [Failed] only as its class allows. ← Swift `permitsSubmit`.
         */
        fun permitsSubmit(resuming: AssetLockBuiltOutPoint?): Boolean = when (this) {
            is Idle -> true
            is Failed -> failure.permitsRestart(resuming)
            is InFlight, is Completed -> false
        }
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
     * re-fund the address), and a [Phase.Failed] whose class does not allow
     * this operation ([Phase.permitsSubmit]). [resuming] names the lock
     * [body] resumes, null for a fresh funding. ← Swift `submit`.
     *
     * [body] performs the actual FFI call and returns the new credit
     * balance, or throws. It runs on its own dispatcher; the terminal
     * phase flip hops back to [scope].
     */
    fun submit(body: suspend () -> Long, resuming: AssetLockBuiltOutPoint? = null) {
        if (!_phase.value.permitsSubmit(resuming)) return
        _phase.value = Phase.InFlight
        lastSubmittedAt = now()
        task = scope.launch {
            try {
                val newBalance = body()
                _phase.value = Phase.Completed(newBalance)
            } catch (e: Throwable) {
                _phase.value = Phase.Failed(PaymentFailure.of(e))
            }
        }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
