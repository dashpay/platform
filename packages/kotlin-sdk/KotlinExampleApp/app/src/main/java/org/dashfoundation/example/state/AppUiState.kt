package org.dashfoundation.example.state

import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.update

/** In-memory bearer value whose diagnostic representation never reveals the URI. */
class SecretInvitationUri(private val value: String) {
    fun reveal(): String = value

    override fun equals(other: Any?): Boolean =
        other is SecretInvitationUri && value == other.value

    override fun hashCode(): Int = value.hashCode()

    override fun toString(): String = "SecretInvitationUri([REDACTED])"
}

/** Application-scoped, deliberately non-saveable invitation UI state. */
class AppUiState {
    val showWalletsSyncDetails = MutableStateFlow(false)

    /** Bearer secret parked only in process memory until a claim sheet can consume it. */
    val pendingInviteUri = MutableStateFlow<SecretInvitationUri?>(null)

    sealed interface CreateInvitationState {
        data object Idle : CreateInvitationState
        data class InFlight(val operationId: Long, val networkRaw: Int) : CreateInvitationState
        data class Ready(
            val operationId: Long,
            val networkRaw: Int,
            val uri: SecretInvitationUri,
        ) : CreateInvitationState
        data class Failed(
            val operationId: Long,
            val networkRaw: Int,
            val safeError: String,
        ) : CreateInvitationState
    }

    data class ClaimRequest(
        val networkRaw: Int,
        val uri: SecretInvitationUri,
        val walletIdHex: String,
    )

    sealed interface ClaimInvitationState {
        data object Idle : ClaimInvitationState
        data class InFlight(val operationId: Long, val request: ClaimRequest) : ClaimInvitationState
        data class Completed(
            val operationId: Long,
            val networkRaw: Int,
            val walletIdHex: String,
        ) : ClaimInvitationState
        data class Failed(
            val operationId: Long,
            val request: ClaimRequest,
            val safeError: String,
        ) : ClaimInvitationState
        data class ContactPrompt(
            val operationId: Long,
            val networkRaw: Int,
            val walletIdHex: String,
            val identityId: ByteArray,
            val username: String,
        ) : ClaimInvitationState
        data class ContactSending(
            val operationId: Long,
            val networkRaw: Int,
            val walletIdHex: String,
            val identityId: ByteArray,
            val username: String,
        ) : ClaimInvitationState
        data class ContactFailed(
            val operationId: Long,
            val networkRaw: Int,
            val walletIdHex: String,
            val identityId: ByteArray,
            val username: String,
            val safeError: String,
        ) : ClaimInvitationState
    }

    sealed interface ReclaimTarget {
        data class TopUp(val identityId: ByteArray) : ReclaimTarget
        data object Register : ReclaimTarget
    }

    data class ReclaimSnapshot(
        val networkRaw: Int,
        val outPointHex: String,
        val target: ReclaimTarget,
    )

    sealed interface ReclaimInvitationState {
        data object Idle : ReclaimInvitationState
        data class InFlight(
            val operationId: Long,
            val snapshot: ReclaimSnapshot,
        ) : ReclaimInvitationState
        data class Completed(
            val operationId: Long,
            val networkRaw: Int,
            val outPointHex: String,
            val message: String,
        ) : ReclaimInvitationState
        data class Failed(
            val operationId: Long,
            val snapshot: ReclaimSnapshot,
            val safeError: String,
        ) : ReclaimInvitationState
    }

    val createInvitation = MutableStateFlow<CreateInvitationState>(CreateInvitationState.Idle)
    val claimInvitation = MutableStateFlow<ClaimInvitationState>(ClaimInvitationState.Idle)
    val reclaimInvitation = MutableStateFlow<ReclaimInvitationState>(ReclaimInvitationState.Idle)

    private var nextOperationId = 1L

    @Synchronized
    fun beginCreateInvitation(networkRaw: Int): Long? {
        if (createInvitation.value is CreateInvitationState.InFlight) return null
        val id = nextId()
        createInvitation.value = CreateInvitationState.InFlight(id, networkRaw)
        return id
    }

    @Synchronized
    fun completeCreateInvitation(operationId: Long, uri: String): Boolean {
        val current = createInvitation.value as? CreateInvitationState.InFlight ?: return false
        if (current.operationId != operationId) return false
        createInvitation.value = CreateInvitationState.Ready(
            operationId,
            current.networkRaw,
            SecretInvitationUri(uri),
        )
        return true
    }

    @Synchronized
    fun failCreateInvitation(operationId: Long, safeError: String): Boolean {
        val current = createInvitation.value as? CreateInvitationState.InFlight ?: return false
        if (current.operationId != operationId) return false
        createInvitation.value =
            CreateInvitationState.Failed(operationId, current.networkRaw, safeError)
        return true
    }

    @Synchronized
    fun clearCreateInvitation() {
        createInvitation.value = CreateInvitationState.Idle
    }

    @Synchronized
    fun beginInvitationClaim(networkRaw: Int, uri: String, walletIdHex: String): Long? {
        when (claimInvitation.value) {
            is ClaimInvitationState.InFlight,
            is ClaimInvitationState.ContactSending,
            -> return null
            else -> Unit
        }
        val id = nextId()
        claimInvitation.value = ClaimInvitationState.InFlight(
            id,
            ClaimRequest(networkRaw, SecretInvitationUri(uri), walletIdHex),
        )
        return id
    }

    @Synchronized
    fun failInvitationClaim(operationId: Long, safeError: String): Boolean {
        val current = claimInvitation.value as? ClaimInvitationState.InFlight ?: return false
        if (current.operationId != operationId) return false
        claimInvitation.value = ClaimInvitationState.Failed(operationId, current.request, safeError)
        return true
    }

    @Synchronized
    fun completeInvitationClaim(
        operationId: Long,
        identityId: ByteArray,
        username: String?,
    ): Boolean {
        val current = claimInvitation.value as? ClaimInvitationState.InFlight ?: return false
        if (current.operationId != operationId) return false
        claimInvitation.value = if (username == null) {
            ClaimInvitationState.Completed(
                operationId,
                current.request.networkRaw,
                current.request.walletIdHex,
            )
        } else {
            ClaimInvitationState.ContactPrompt(
                operationId,
                current.request.networkRaw,
                current.request.walletIdHex,
                identityId.copyOf(),
                username,
            )
        }
        return true
    }

    @Synchronized
    fun beginInvitationContactSend(): ClaimInvitationState.ContactSending? {
        val prompt = when (val current = claimInvitation.value) {
            is ClaimInvitationState.ContactPrompt -> current
            is ClaimInvitationState.ContactFailed -> ClaimInvitationState.ContactPrompt(
                current.operationId,
                current.networkRaw,
                current.walletIdHex,
                current.identityId,
                current.username,
            )
            else -> return null
        }
        return ClaimInvitationState.ContactSending(
            prompt.operationId,
            prompt.networkRaw,
            prompt.walletIdHex,
            prompt.identityId.copyOf(),
            prompt.username,
        ).also { claimInvitation.value = it }
    }

    @Synchronized
    fun failInvitationContactSend(operationId: Long, safeError: String): Boolean {
        val current = claimInvitation.value as? ClaimInvitationState.ContactSending ?: return false
        if (current.operationId != operationId) return false
        claimInvitation.value = ClaimInvitationState.ContactFailed(
            current.operationId,
            current.networkRaw,
            current.walletIdHex,
            current.identityId.copyOf(),
            current.username,
            safeError,
        )
        return true
    }

    @Synchronized
    fun completeInvitationContactSend(operationId: Long): Boolean {
        val current = claimInvitation.value as? ClaimInvitationState.ContactSending ?: return false
        if (current.operationId != operationId) return false
        claimInvitation.value = ClaimInvitationState.Completed(
            current.operationId,
            current.networkRaw,
            current.walletIdHex,
        )
        return true
    }

    @Synchronized
    fun clearInvitationClaim() {
        claimInvitation.value = ClaimInvitationState.Idle
    }

    fun clearPendingInviteUriIfPresented(presentedUri: String?) {
        val uri = presentedUri ?: return
        pendingInviteUri.update { pending ->
            pending?.takeUnless { it.reveal() == uri }
        }
    }

    @Synchronized
    fun beginInvitationReclaim(snapshot: ReclaimSnapshot): Long? {
        if (reclaimInvitation.value is ReclaimInvitationState.InFlight) return null
        val id = nextId()
        reclaimInvitation.value = ReclaimInvitationState.InFlight(id, snapshot.copy())
        return id
    }

    @Synchronized
    fun completeInvitationReclaim(operationId: Long, message: String): Boolean {
        val current = reclaimInvitation.value as? ReclaimInvitationState.InFlight ?: return false
        if (current.operationId != operationId) return false
        reclaimInvitation.value = ReclaimInvitationState.Completed(
            operationId,
            current.snapshot.networkRaw,
            current.snapshot.outPointHex,
            message,
        )
        return true
    }

    @Synchronized
    fun failInvitationReclaim(operationId: Long, safeError: String): Boolean {
        val current = reclaimInvitation.value as? ReclaimInvitationState.InFlight ?: return false
        if (current.operationId != operationId) return false
        reclaimInvitation.value =
            ReclaimInvitationState.Failed(operationId, current.snapshot, safeError)
        return true
    }

    @Synchronized
    fun clearInvitationReclaim() {
        reclaimInvitation.value = ReclaimInvitationState.Idle
    }

    @Synchronized
    fun clearInvitationStateForNetworkChange() {
        pendingInviteUri.value = null
        createInvitation.value = CreateInvitationState.Idle
        claimInvitation.value = ClaimInvitationState.Idle
        reclaimInvitation.value = ReclaimInvitationState.Idle
        scanResultSink = null
    }

    private fun nextId(): Long = nextOperationId++

    /** One-shot scan callback; never saved to Android instance state. */
    var scanResultSink: ((String) -> Unit)? = null
}
