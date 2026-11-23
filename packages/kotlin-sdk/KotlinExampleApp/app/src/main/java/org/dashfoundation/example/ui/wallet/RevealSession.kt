package org.dashfoundation.example.ui.wallet

/**
 * The state of one private-key reveal on [CoreAddressDetailScreen], kept apart
 * from Compose so its rules can be unit tested. Port of the `revealRequest`
 * token in `CoreAddressDetailView` (`StorageRecordDetailViews.swift`).
 *
 * A reveal is [Phase.AUTHORIZING] while the device prompt is up and
 * [Phase.DERIVING] once the owner authenticated and the screen resumed. Only a
 * stop while [Phase.DERIVING] invalidates it: the device-credential screen of
 * the prompt stops the activity itself, so a stop while authorizing is the
 * prompt's own. A derivation that finishes after such a stop is refused by
 * [finish], so its key is wiped instead of shown to whoever resumes the app.
 */
internal class RevealSession {

    enum class Phase { IDLE, AUTHORIZING, DERIVING }

    var phase: Phase = Phase.IDLE
        private set

    private var request = 0L

    /** Start a reveal: its token, or null while another reveal runs. */
    fun begin(): Long? {
        if (phase != Phase.IDLE) return null
        request += 1
        phase = Phase.AUTHORIZING
        return request
    }

    /**
     * The owner authenticated for [token] and the screen is resumed again:
     * true when the derivation may start.
     */
    fun authorized(token: Long): Boolean {
        if (token != request || phase != Phase.AUTHORIZING) return false
        phase = Phase.DERIVING
        return true
    }

    /** The prompt for [token] was cancelled or failed. */
    fun abandon(token: Long) {
        if (token == request) phase = Phase.IDLE
    }

    /** The derivation for [token] returned: true when its key may be shown. */
    fun finish(token: Long): Boolean {
        val current = token == request && phase == Phase.DERIVING
        if (token == request) phase = Phase.IDLE
        return current
    }

    /** The activity stopped. */
    fun onStop() {
        if (phase == Phase.DERIVING) invalidate()
    }

    /** Drop any reveal in flight; its late result is refused. */
    fun invalidate() {
        request += 1
        phase = Phase.IDLE
    }
}
