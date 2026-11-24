package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.PrivateDocumentEntryCreated
import org.dashfoundation.dashsdk.wallet.PrivateDocumentSenderAuth
import org.dashfoundation.example.ui.contracts.PrivateEntryCreateOutcome

/**
 * Owns the creates of private document entries (type 29) past the screen that started them:
 * a create proves for tens of seconds, and going back or a rotation recreates the screen
 * meanwhile. The create's job runs on the application scope; a recreated screen observes the
 * same [states] entry, sees the create still running, and then its outcome.
 *
 * It also remembers, for the life of the process, every request whose create may have gone
 * through ([held]): the same request is not sent again, so a second tap, a reopened screen or
 * an edit that changes nothing sent cannot pay for the same entry twice. A request is compared
 * by what is sent (ids as hex, the key id, the auth mode and the encoded properties), not by
 * how the form spelled it. Rust keeps the notes of such a create reserved and redrives its saved
 * transition; process death is covered there, not here.
 *
 * It holds no rule about what may be created: that is Rust's refusal.
 */
class PrivateDocumentEntryCreateCoordinator(private val scope: CoroutineScope) {

    /** Exactly what one create sends. Ids are lowercase hex. */
    data class Request(
        val walletIdHex: String,
        val senderIdentityIdHex: String,
        val contractIdHex: String,
        val documentTypeName: String,
        val recipientIdentityIdHex: String,
        val recipientKeyId: Long,
        val auth: PrivateDocumentSenderAuth,
        val propertiesJson: String,
    )

    sealed interface State {
        val request: Request

        /** The wallet call is running. */
        data class InFlight(override val request: Request) : State

        /** The call returned or threw; kept until the wallet's next create. */
        data class Done(override val request: Request, val outcome: PrivateEntryCreateOutcome) : State
    }

    /** Why [start] did not start. */
    enum class Refusal {
        /** A create from this wallet is running. */
        BUSY,

        /** This request may already have gone through: it is not sent again. */
        HELD,
    }

    private val _states = MutableStateFlow<Map<String, State>>(emptyMap())

    /** The latest create of each wallet, by lowercase hex wallet id. */
    val states: StateFlow<Map<String, State>> = _states.asStateFlow()

    private val _held = MutableStateFlow<Set<Request>>(emptySet())

    /** The requests whose create may have gone through in this process. */
    val held: StateFlow<Set<Request>> = _held.asStateFlow()

    /**
     * Start [create] for [request] unless a create from the same wallet is running or the same
     * request may already have gone through. Returns null when it started. [create] runs on the
     * application scope; its error is mapped by [PrivateEntryCreateOutcome.of].
     */
    fun start(request: Request, create: suspend () -> PrivateDocumentEntryCreated): Refusal? {
        // Launch only after this caller's own compare-and-set reserved the wallet's slot, so two
        // concurrent starts can never both pay
        while (true) {
            val current = _states.value
            if (current[request.walletIdHex] is State.InFlight) return Refusal.BUSY
            if (request in _held.value) return Refusal.HELD
            val next = current + (request.walletIdHex to State.InFlight(request))
            if (_states.compareAndSet(current, next)) break
        }
        scope.launch {
            val outcome = try {
                PrivateEntryCreateOutcome.Created(create())
            } catch (e: CancellationException) {
                // The application scope ended during the call: nothing shows it did not go through
                PrivateEntryCreateOutcome.Unconfirmed(PrivateEntryCreateOutcome.UNCONFIRMED_MESSAGE)
            } catch (e: Exception) {
                PrivateEntryCreateOutcome.of(e)
            }
            if (outcome is PrivateEntryCreateOutcome.Unconfirmed) _held.update { it + request }
            _states.update { it + (request.walletIdHex to State.Done(request, outcome)) }
        }
        return null
    }
}
