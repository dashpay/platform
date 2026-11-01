package org.dashfoundation.example.ui.dashpay

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.tokens.ShieldedTipDisposition
import org.dashfoundation.dashsdk.tokens.ShieldedTipFailure

/**
 * Application-owned submissions survive sheet dismissal, navigation and activity recreation.
 * Restart safety is the wallet's: it records each tip as unresolved before broadcast and
 * refuses another until it settles. Mirrors Swift's `ShieldedTipSubmissions`.
 */
class ShieldedTipSubmissions(private val scope: CoroutineScope) {
    private val wallets = mutableMapOf<String, ShieldedTipSubmission>()

    /** Called on the main thread; one submission per network/wallet, across all of its identities. */
    fun forWallet(network: Int, walletId: String): ShieldedTipSubmission =
        wallets.getOrPut("$network:$walletId") { ShieldedTipSubmission(scope) }
}

/** Main-thread state that renders the wallet's decisions and makes none. */
class ShieldedTipSubmission(private val scope: CoroutineScope) {
    enum class Status { Ready, Sending, Sent, Unresolved }

    var status by mutableStateOf(Status.Ready)
        private set
    var message by mutableStateOf<String?>(null)
        private set
    val busy: Boolean get() = status == Status.Sending
    val submitted: Boolean get() = status != Status.Ready

    fun submit(send: suspend () -> Unit) {
        if (submitted) return
        // Lock synchronously, before launching, so two UI events cannot submit twice.
        status = Status.Sending
        message = null
        scope.launch {
            try {
                send()
                status = Status.Sent
                message = "Shielded tip sent."
            } catch (error: Exception) {
                status = if (wasNotSent(error)) Status.Ready else Status.Unresolved
                message = if (status == Status.Unresolved) {
                    "The tip may have been sent. Shielded sync settles it; the wallet refuses another tip " +
                        "to this recipient until then. ${error.message.orEmpty()}"
                } else {
                    error.message ?: "Unable to send tip"
                }
                if (error is kotlinx.coroutines.CancellationException) throw error
            }
        }
    }

    /**
     * Starting another payment is an explicit action after a finished send. After an unresolved
     * one, the wallet still refuses a tip to that recipient until shielded sync settles it.
     */
    fun startNewTip() {
        if (status != Status.Sent && status != Status.Unresolved) return
        status = Status.Ready
        message = null
    }

    private companion object {
        /** The wallet's disposition decides; the SDK reports every send failure as [ShieldedTipFailure]. */
        fun wasNotSent(error: Exception): Boolean =
            (error as? ShieldedTipFailure)?.disposition == ShieldedTipDisposition.NOT_SENT
    }
}
