package org.dashfoundation.example.services.shielded

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * Hub for in-flight "shield funds from an asset lock" attempts — 1:1 port
 * of `ShieldedFundFromAssetLockCoordinator.swift`. Held on the app
 * container so fundings survive screen dismissal and network-toggle
 * pressure.
 *
 * Keyed by `(walletId, recipientRaw43)`. Beyond the per-slot single-flight
 * that [org.dashfoundation.example.services.assetlock.AddressFundFromAssetLockCoordinator]
 * has, this coordinator ALSO enforces **per-wallet serialization**: while
 * one recipient is shielding on a wallet, a start for a *different*
 * recipient on the same wallet is rejected as
 * [StartFundingResult.BlockedByOtherWalletFunding] (mirroring the Rust-side
 * `shield_guard` mutex — two concurrent Orchard builds on one wallet would
 * race the note-commitment tree).
 *
 * Reuse of a controller additionally requires a matching **operation
 * identity** (`operationId` — the resumed lock's outpoint, or the
 * fresh-shield marker): resumable locks normally default to the same
 * wallet-owned shielded recipient, so two different locks share one slot
 * key, and reusing the first lock's controller would silently drop the
 * second lock's resume body. A different operation is blocked while the
 * slot is in flight and replaces the retained controller once it has
 * completed.
 */
class ShieldedFundFromAssetLockCoordinator(
    private val scope: CoroutineScope,
    private val retentionMillis: Long = 30_000L,
    private val pollMillis: Long = 1_000L,
    private val now: () -> Long = System::currentTimeMillis,
) {
    /** Composite slot key. Binary ids compared as hex. ← Swift `SlotKey`. */
    data class SlotKey(val walletIdHex: String, val recipientRaw43Hex: String)

    /** Result of a [startFunding] attempt. ← Swift `StartFundingResult`. */
    sealed interface StartFundingResult {
        /** Fresh or restarted controller for this slot. */
        data class Started(val controller: ShieldedFundFromAssetLockController) : StartFundingResult

        /**
         * Another recipient is already shielding on this wallet; the
         * blocker is returned so the UI can name it. No new funding starts.
         */
        data class BlockedByOtherWalletFunding(
            val blocker: ShieldedFundFromAssetLockController,
        ) : StartFundingResult
    }

    private val _controllers =
        MutableStateFlow<Map<SlotKey, ShieldedFundFromAssetLockController>>(emptyMap())

    /** Active controllers keyed by slot, observable by the pending-fundings list. */
    val controllers: StateFlow<Map<SlotKey, ShieldedFundFromAssetLockController>> =
        _controllers.asStateFlow()

    /** True when at least one slot is InFlight — network-picker gate. ← Swift. */
    val hasInFlightFundings: Boolean
        get() = _controllers.value.values.any { it.phase.value.isActive }

    /** The controller for a slot, or null. ← Swift `controller(...)`. */
    fun controller(
        walletId: ByteArray,
        recipientRaw43: ByteArray,
    ): ShieldedFundFromAssetLockController? =
        _controllers.value[key(walletId, recipientRaw43)]

    /** Every active controller, most-recently-submitted first. ← Swift. */
    fun activeControllers(): List<ShieldedFundFromAssetLockController> =
        _controllers.value.values.sortedByDescending { it.lastSubmittedAt ?: Long.MIN_VALUE }

    /**
     * Start a funding for the slot. Reuses the existing controller only
     * when the SAME operation ([operationId]) is already in flight / just
     * completed on the slot — a re-tap. A DIFFERENT operation on an
     * in-flight slot is reported as
     * [StartFundingResult.BlockedByOtherWalletFunding] rather than
     * silently reusing the controller (which would drop its body: two
     * resumable locks normally share the wallet's default shielded
     * recipient, so the slot key alone cannot tell them apart). A
     * different operation on a *completed* slot is a fresh start — the
     * retained controller is replaced. Rejects a DIFFERENT recipient
     * while another is in flight on the same wallet. [Phase.Idle] /
     * [Phase.Failed] on the same slot are legitimate restarts.
     * ← Swift `startFunding`.
     *
     * [operationId] is the identity of the requested operation: the
     * resumed lock's outpoint for a resume, a fixed marker for a fresh
     * shield. Wallet-wide serialization is unchanged — at most one
     * shield-class operation runs per wallet either way.
     */
    fun startFunding(
        walletId: ByteArray,
        recipientRaw43: ByteArray,
        operationId: String,
        body: suspend () -> Unit,
    ): StartFundingResult {
        val key = key(walletId, recipientRaw43)
        val existing = _controllers.value[key]

        // Per-wallet serialization: a DIFFERENT recipient in flight blocks.
        val walletHex = walletId.toHex()
        val blocker = _controllers.value.entries.firstOrNull { (k, c) ->
            k.walletIdHex == walletHex && k != key && c.phase.value.isActive
        }?.value
        if (blocker != null) {
            return StartFundingResult.BlockedByOtherWalletFunding(blocker)
        }

        if (existing != null) {
            when (val phase = existing.phase.value) {
                is ShieldedFundFromAssetLockController.Phase.InFlight,
                is ShieldedFundFromAssetLockController.Phase.Completed,
                -> {
                    if (existing.operationId == operationId) {
                        // Re-tap of the same operation: bind to its state.
                        return StartFundingResult.Started(existing)
                    }
                    if (phase is ShieldedFundFromAssetLockController.Phase.InFlight) {
                        // A different operation while one is running on this
                        // wallet: same serialization verdict as a different
                        // recipient — the caller gets the blocker, not a
                        // controller that will never run its body.
                        return StartFundingResult.BlockedByOtherWalletFunding(existing)
                    }
                    // Completed slot + different operation: a fresh start,
                    // not a re-tap. Fall through and REPLACE the retained
                    // controller (its sweep is identity-guarded, so the old
                    // retention timer can't evict the replacement).
                }
                is ShieldedFundFromAssetLockController.Phase.Idle,
                is ShieldedFundFromAssetLockController.Phase.Failed,
                -> {
                    existing.submit(operationId, body)
                    scheduleRetentionSweep(key, existing)
                    return StartFundingResult.Started(existing)
                }
            }
        }
        val controller = ShieldedFundFromAssetLockController(walletId, recipientRaw43, scope, now)
        _controllers.update { it + (key to controller) }
        controller.submit(operationId, body)
        scheduleRetentionSweep(key, controller)
        return StartFundingResult.Started(controller)
    }

    /** Manually drop a controller — the UI's "Dismiss" on a failed row. ← Swift. */
    fun dismiss(walletId: ByteArray, recipientRaw43: ByteArray) {
        _controllers.update { it - key(walletId, recipientRaw43) }
    }

    /**
     * Auto-purge a [Phase.Completed] controller ~[retentionMillis] after
     * success; [Phase.Failed] stays until dismissed. Polls every
     * [pollMillis]. ← Swift `scheduleRetentionSweep`.
     */
    private fun scheduleRetentionSweep(
        key: SlotKey,
        controller: ShieldedFundFromAssetLockController,
    ) {
        scope.launch {
            var completedAt: Long? = null
            while (true) {
                // The slot may have been handed to a REPLACEMENT controller
                // (a different operation started during this controller's
                // completed-retention window). This sweep then owns nothing:
                // exit without touching the slot, leaving the replacement's
                // own sweep in charge.
                if (_controllers.value[key] !== controller) return@launch
                when (controller.phase.value) {
                    is ShieldedFundFromAssetLockController.Phase.Completed -> {
                        val nowMs = now()
                        if (completedAt == null) {
                            completedAt = nowMs
                        } else if (nowMs - completedAt >= retentionMillis) {
                            _controllers.update { it - key }
                            return@launch
                        }
                    }
                    is ShieldedFundFromAssetLockController.Phase.Failed -> return@launch
                    else -> completedAt = null
                }
                delay(pollMillis)
            }
        }
    }

    private fun key(walletId: ByteArray, recipientRaw43: ByteArray): SlotKey =
        SlotKey(walletId.toHex(), recipientRaw43.toHex())

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
