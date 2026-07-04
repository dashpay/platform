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
     * Start a funding for the slot. Reuses the existing controller if the
     * SAME recipient is already in flight / just completed. Rejects a
     * DIFFERENT recipient while another is in flight on the same wallet
     * ([StartFundingResult.BlockedByOtherWalletFunding]). [Phase.Idle] /
     * [Phase.Failed] on the same slot are legitimate restarts.
     * ← Swift `startFunding`.
     */
    fun startFunding(
        walletId: ByteArray,
        recipientRaw43: ByteArray,
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
            when (existing.phase.value) {
                is ShieldedFundFromAssetLockController.Phase.InFlight,
                is ShieldedFundFromAssetLockController.Phase.Completed,
                -> return StartFundingResult.Started(existing)
                is ShieldedFundFromAssetLockController.Phase.Idle,
                is ShieldedFundFromAssetLockController.Phase.Failed,
                -> {
                    existing.submit(body)
                    scheduleRetentionSweep(key, existing)
                    return StartFundingResult.Started(existing)
                }
            }
        }
        val controller = ShieldedFundFromAssetLockController(walletId, recipientRaw43, scope, now)
        _controllers.update { it + (key to controller) }
        controller.submit(body)
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
