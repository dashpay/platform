package org.dashfoundation.example.state

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class AppUiStateTest {
    private val secret = "dashpay://invite?pk=sentinel-secret"

    @Test
    fun `secret invitation URI is redacted from stringification`() {
        val wrapped = SecretInvitationUri(secret)

        assertFalse(wrapped.toString().contains(secret))
        assertFalse(AppUiState.CreateInvitationState.Ready(1, 0, wrapped).toString().contains(secret))
    }

    @Test
    fun `late create completion cannot overwrite a newer generation`() {
        val state = AppUiState()
        val first = checkNotNull(state.beginCreateInvitation(0))
        state.clearCreateInvitation()
        val second = checkNotNull(state.beginCreateInvitation(0))

        assertFalse(state.completeCreateInvitation(first, secret))
        assertTrue(state.completeCreateInvitation(second, secret))
        val ready = state.createInvitation.value as AppUiState.CreateInvitationState.Ready
        assertEquals(secret, ready.uri.reveal())
    }

    @Test
    fun `claim failure retains retry request but never stores secret in error`() {
        val state = AppUiState()
        val operationId = checkNotNull(
            state.beginInvitationClaim(0, secret, "wallet"),
        )

        assertTrue(state.failInvitationClaim(operationId, "Claiming the invitation failed."))
        val failed = state.claimInvitation.value as AppUiState.ClaimInvitationState.Failed
        assertEquals(secret, failed.request.uri.reveal())
        assertFalse(failed.safeError.contains(secret))
        assertFalse(failed.toString().contains(secret))
    }

    @Test
    fun `reclaim state single flights and rejects stale completion`() {
        val state = AppUiState()
        val snapshot = AppUiState.ReclaimSnapshot(
            networkRaw = 0,
            outPointHex = "01:0",
            target = AppUiState.ReclaimTarget.TopUp(ByteArray(32) { 1 }),
        )
        val first = checkNotNull(state.beginInvitationReclaim(snapshot))

        assertNull(state.beginInvitationReclaim(snapshot))
        state.clearInvitationReclaim()
        assertFalse(state.completeInvitationReclaim(first, "Reclaimed"))
    }

    @Test
    fun `network reset clears operation state and parked bearer`() {
        val state = AppUiState()
        state.pendingInviteUri.value = SecretInvitationUri(secret)
        state.beginCreateInvitation(0)
        state.beginInvitationClaim(0, secret, "wallet")
        state.beginInvitationReclaim(
            AppUiState.ReclaimSnapshot(0, "01:0", AppUiState.ReclaimTarget.Register),
        )

        state.clearInvitationStateForNetworkChange()

        assertTrue(state.createInvitation.value is AppUiState.CreateInvitationState.Idle)
        assertTrue(state.claimInvitation.value is AppUiState.ClaimInvitationState.Idle)
        assertTrue(state.reclaimInvitation.value is AppUiState.ReclaimInvitationState.Idle)
        assertNull(state.pendingInviteUri.value)
    }

    @Test
    fun `closing a completed claim preserves a different queued invitation`() {
        val state = AppUiState()
        val queuedSecret = "dashpay://invite?pk=queued-secret"
        state.pendingInviteUri.value = SecretInvitationUri(queuedSecret)

        state.clearPendingInviteUriIfPresented(secret)

        assertEquals(queuedSecret, state.pendingInviteUri.value?.reveal())

        state.clearPendingInviteUriIfPresented(queuedSecret)

        assertNull(state.pendingInviteUri.value)
    }
}
