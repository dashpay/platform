package org.dashfoundation.example.services

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

/**
 * Per-slot state for a single identity registration attempt — 1:1 port of
 * `IdentityRegistrationController.swift`.
 *
 * One controller is created per `(walletId, identityIndex)` slot when the
 * user submits the create-identity flow. It owns the in-flight [Job],
 * exposes its current [phase] via a [StateFlow], and survives view
 * dismissal via [RegistrationCoordinator] on the app container. Multiple
 * controllers can be active simultaneously (one per slot).
 *
 * State mutations run on [scope] (the app's main-immediate scope, the
 * Kotlin analog of Swift's `@MainActor`) so Compose observers see
 * consistent transitions; the [submit] body runs off that scope and only
 * hops back to flip the terminal phase.
 */
class IdentityRegistrationController(
    val walletId: ByteArray,
    val identityIndex: Int,
    val fundingKind: FundingKind = FundingKind.AssetLock,
    private val scope: CoroutineScope,
    private val now: () -> Long = System::currentTimeMillis,
) {
    /**
     * How this registration is funded. Drives which step set the progress
     * screen renders: the asset-lock funding paths (Core / Platform-Payment
     * / resume) walk the build → IS/CL → register steps; the shielded-pool
     * path has no asset lock — its long pole is the ZK proof — so it renders
     * its own step set. ← `IdentityRegistrationController.FundingKind`.
     */
    enum class FundingKind { AssetLock, PlatformAddresses, ShieldedPool }

    /**
     * Registration phase. Flow: [Idle] → [PreparingKeys] (caller) →
     * [InFlight] (submit) → [Completed] | [Failed] | [Unconfirmed].
     * ← the Swift `Phase` enum, case-for-case.
     */
    sealed interface Phase {
        /** Pre-submit; controller exists but [submit] hasn't fired. */
        data object Idle : Phase

        /** Step 1: pre-deriving + persisting the identity keys. */
        data object PreparingKeys : Phase

        /** Steps 2–4: the FFI registration call is in flight. */
        data object InFlight : Phase

        /** Step 5: identity registered; [identityId] is the 32-byte id. */
        data class Completed(val identityId: ByteArray) : Phase {
            override fun equals(other: Any?): Boolean =
                other is Completed && identityId.contentEquals(other.identityId)

            override fun hashCode(): Int = identityId.contentHashCode()
        }

        /** Failure terminal state; the message is shown inline. Stays until dismissed. */
        data class Failed(val message: String) : Phase

        /**
         * Broadcast-succeeded-but-confirmation-failed terminal state
         * (shielded only). The identity at [identityId] MAY already be live;
         * the slot must stay held so a re-submission can't burn funds against
         * the registered-key-hash check.
         */
        data class Unconfirmed(val identityId: ByteArray, val message: String) : Phase {
            override fun equals(other: Any?): Boolean =
                other is Unconfirmed &&
                    identityId.contentEquals(other.identityId) &&
                    message == other.message

            override fun hashCode(): Int = 31 * identityId.contentHashCode() + message.hashCode()
        }

        /**
         * Whether the controller is currently holding its slot against a
         * fresh registration. [PreparingKeys], [InFlight], [Unconfirmed] hold
         * it; [Idle], [Completed], [Failed] release it. ← Swift `isActive`.
         */
        val isActive: Boolean
            get() = this is PreparingKeys || this is InFlight || this is Unconfirmed
    }

    /**
     * Which stage a shielded [Phase.Failed] failed at, so the progress
     * screen can attribute the red marker. Only [BroadcastRejected] is
     * attributable with confidence; every other failure leaves this null and
     * the progress screen falls back to its elapsed-time heuristic.
     * ← Swift `FailureStage`.
     */
    enum class FailureStage { BroadcastRejected }

    private val _phase = MutableStateFlow<Phase>(Phase.Idle)
    val phase: StateFlow<Phase> = _phase.asStateFlow()

    private val _failureStage = MutableStateFlow<FailureStage?>(null)
    val failureStage: StateFlow<FailureStage?> = _failureStage.asStateFlow()

    /** Timestamp (millis) of the most recent [submit] — drives the coordinator's TTL. */
    var lastSubmittedAt: Long? = null
        private set

    /** Timestamp (millis) of the most recent terminal transition — freezes the shielded step heuristic. */
    var terminalAt: Long? = null
        private set

    private var task: Job? = null

    /** Composite id for stable list diffing — wallet hex + slot (← `registrationRowID`). */
    val slotRowId: String
        get() = walletId.joinToString("") { "%02x".format(it) } + "-" + identityIndex

    /** Transition to [Phase.PreparingKeys]; the caller calls this before [submit]. */
    fun enterPreparingKeys() {
        _phase.value = Phase.PreparingKeys
    }

    /**
     * Submit the registration. Defensively rejects [Phase.InFlight],
     * [Phase.Completed], and [Phase.Unconfirmed] (re-firing would race a
     * duplicate FFI call or re-register a live identity). [Phase.Idle],
     * [Phase.PreparingKeys], and [Phase.Failed] are allowed — the coordinator
     * drives the legitimate restart through them. ← Swift `submit`.
     *
     * [body] performs the actual FFI call and returns the 32-byte identity
     * id, or throws. It runs on its own dispatcher; the terminal phase flip
     * hops back to [scope].
     */
    fun submit(
        body: suspend () -> ByteArray,
        isUnconfirmed: (Throwable) -> ByteArray? = { null },
        isBroadcastRejected: (Throwable) -> Boolean = { false },
    ) {
        when (_phase.value) {
            is Phase.Idle, is Phase.PreparingKeys, is Phase.Failed -> Unit
            is Phase.InFlight, is Phase.Completed, is Phase.Unconfirmed -> return
        }
        _phase.value = Phase.InFlight
        _failureStage.value = null
        lastSubmittedAt = now()
        terminalAt = null
        task = scope.launch {
            try {
                val identityId = body()
                terminalAt = now()
                _phase.value = Phase.Completed(identityId)
            } catch (e: Throwable) {
                val unconfirmedId = isUnconfirmed(e)
                if (unconfirmedId != null) {
                    terminalAt = now()
                    _phase.value = Phase.Unconfirmed(unconfirmedId, e.message ?: "Confirmation pending")
                } else {
                    _failureStage.value = if (isBroadcastRejected(e)) FailureStage.BroadcastRejected else null
                    terminalAt = now()
                    _phase.value = Phase.Failed(e.message ?: "Registration failed")
                }
            }
        }
    }
}
