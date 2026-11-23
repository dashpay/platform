package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.dashfoundation.example.ui.shielded.ShieldedPaymentOutcome

/**
 * Owns the in-flight payment between an identity and the shielded pool, so
 * it outlives the screen that started it: a shield from an identity's
 * balance (type 21) and a top-up of an identity from the shielded pool
 * (type 22) prove for tens of seconds, and a rotation or a tab switch
 * recreates the screen meanwhile. The payment's job runs on the
 * application scope; a recreated screen observes the same [states] entry,
 * sees the payment still running and cannot start another for the same
 * wallet and identity until it has finished and its outcome was shown.
 *
 * It holds no payment rule: whether a payment may be made is Rust's
 * refusal, and the outcome is the wallet call's own. Process death is not
 * covered here; the recovery lists read what the wallet saved.
 */
class ShieldedIdentityPaymentCoordinator(private val scope: CoroutineScope) {

    enum class Kind { SHIELD_FROM_IDENTITY, TOP_UP_FROM_POOL }

    /** One slot per kind, wallet and identity (lowercase hex ids). */
    data class Key(val kind: Kind, val walletIdHex: String, val identityIdHex: String)

    sealed interface State {
        val amount: Long

        /** The wallet call is running. */
        data class InFlight(override val amount: Long) : State

        /** The call returned or threw; kept until the screen showed it. */
        data class Done(override val amount: Long, val outcome: ShieldedPaymentOutcome) : State
    }

    private val _states = MutableStateFlow<Map<Key, State>>(emptyMap())
    val states: StateFlow<Map<Key, State>> = _states.asStateFlow()

    /**
     * Start [pay] for [key] unless that slot is in flight or holds an outcome
     * not shown yet. Returns whether it started. [pay] runs on the
     * application scope and maps its own result to an outcome.
     */
    fun start(key: Key, amount: Long, pay: suspend () -> ShieldedPaymentOutcome): Boolean {
        var started = false
        _states.update { current ->
            if (current.containsKey(key)) {
                current
            } else {
                started = true
                current + (key to State.InFlight(amount))
            }
        }
        if (!started) return false
        scope.launch {
            val outcome = try {
                pay()
            } catch (e: Exception) {
                ShieldedPaymentOutcome.Failed(e.message ?: "The payment failed.")
            }
            _states.update { it + (key to State.Done(amount, outcome)) }
        }
        return true
    }

    /** The screen showed [key]'s outcome; the slot is free again. A running payment stays. */
    fun acknowledge(key: Key) {
        _states.update { current ->
            if (current[key] is State.Done) current - key else current
        }
    }
}
