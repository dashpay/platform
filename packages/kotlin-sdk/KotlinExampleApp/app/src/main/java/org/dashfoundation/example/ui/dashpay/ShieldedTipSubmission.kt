package org.dashfoundation.example.ui.dashpay

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

/** Application-owned submissions survive sheet dismissal, navigation and activity recreation. */
class ShieldedTipSubmissions(private val scope: CoroutineScope) {
    private val wallets = mutableMapOf<String, ShieldedTipSubmission>()

    /** Called on the main thread; one guard per network/wallet, across all of its identities. */
    fun forWallet(network: Int, walletId: String): ShieldedTipSubmission =
        wallets.getOrPut("$network:$walletId") { ShieldedTipSubmission(scope) }
}

/** Main-thread state. A cancelled UI must never imply that blocking JNI stopped broadcasting. */
class ShieldedTipSubmission(private val scope: CoroutineScope) {
    enum class Status { Ready, Sending, Sent, Uncertain }

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
                status = if (canReviewShieldedTipAfterFailure(error)) Status.Ready else Status.Uncertain
                message = if (status == Status.Uncertain) {
                    "The tip may have been sent. Check shielded activity before sending again. ${error.message.orEmpty()}"
                } else {
                    error.message ?: "Unable to send tip"
                }
                if (error is kotlinx.coroutines.CancellationException) throw error
            }
        }
    }

    /** Starting another payment requires an explicit action after a confirmed successful return. */
    fun startNewTip() {
        if (status != Status.Sent) return
        status = Status.Ready
        message = null
    }
}
