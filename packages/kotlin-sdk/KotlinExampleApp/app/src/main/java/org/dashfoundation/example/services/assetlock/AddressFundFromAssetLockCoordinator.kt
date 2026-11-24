package org.dashfoundation.example.services.assetlock

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet.AssetLockBuiltOutPoint

/**
 * Hub for in-flight "fund a Platform address from an asset lock" attempts
 * — 1:1 port of `AddressFundFromAssetLockCoordinator.swift`. Held on the
 * app container (the Kotlin analog of hosting it on `PlatformWalletManager`)
 * so fundings survive screen dismissal and network-toggle pressure.
 *
 * Keyed by `(walletId, platformAccountIndex, recipientHash)` — a wallet can
 * fund many addresses concurrently on the same account, so the slot key is
 * composite. [startFunding] single-flights on the slot key, and
 * [hasInFlightFundings] gates the Settings network picker alongside the
 * registration coordinator.
 *
 * Mirrors [org.dashfoundation.example.services.RegistrationCoordinator]:
 * state lives behind [controllers] (a [StateFlow] map) so Compose observers
 * pick up mutations; the retention sweep runs on [scope] so `.Completed`
 * rows auto-purge ~[retentionMillis] after success. [retentionMillis] /
 * [pollMillis] / [now] are injectable so tests drive the sweep on virtual
 * time.
 */
class AddressFundFromAssetLockCoordinator(
    private val scope: CoroutineScope,
    private val retentionMillis: Long = 30_000L,
    private val pollMillis: Long = 1_000L,
    private val now: () -> Long = System::currentTimeMillis,
) {
    /** Composite slot key. Binary ids compared as hex. ← Swift `SlotKey`. */
    data class SlotKey(
        val walletIdHex: String,
        val platformAccountIndex: Int,
        val recipientHashHex: String,
    )

    private val _controllers =
        MutableStateFlow<Map<SlotKey, AddressFundFromAssetLockController>>(emptyMap())

    /** Active controllers keyed by slot, observable by the pending-fundings list. */
    val controllers: StateFlow<Map<SlotKey, AddressFundFromAssetLockController>> =
        _controllers.asStateFlow()

    /**
     * True when at least one slot is [AddressFundFromAssetLockController.Phase.InFlight]
     * — used by the network picker's enabled gate. ← Swift `hasInFlightFundings`.
     */
    val hasInFlightFundings: Boolean
        get() = _controllers.value.values.any { it.phase.value.isActive }

    /** The controller for a slot, or null if none is active. ← Swift `controller(...)`. */
    fun controller(
        walletId: ByteArray,
        platformAccountIndex: Int,
        recipientHash: ByteArray,
    ): AddressFundFromAssetLockController? =
        _controllers.value[key(walletId, platformAccountIndex, recipientHash)]

    /**
     * Every active controller, most-recently-submitted first — the order
     * the pending-fundings list renders. ← Swift `activeControllers()`.
     */
    fun activeControllers(): List<AddressFundFromAssetLockController> =
        _controllers.value.values.sortedByDescending { it.lastSubmittedAt ?: Long.MIN_VALUE }

    /**
     * Start a funding for the slot, or reuse the existing controller if one
     * is already in flight. Single-flighting is enforced HERE: a second tap
     * during the FFI window returns the existing controller and does NOT
     * re-fire the body. [Phase.Idle] / [Phase.Failed] are legitimate
     * restarts. ← Swift `startFunding`.
     */
    fun startFunding(
        walletId: ByteArray,
        platformAccountIndex: Int,
        recipientHash: ByteArray,
        recipientType: Int,
        resuming: AssetLockBuiltOutPoint? = null,
        body: suspend () -> Long,
    ): AddressFundFromAssetLockController {
        val key = key(walletId, platformAccountIndex, recipientHash)
        val existing = _controllers.value[key]
        if (existing != null) {
            when (existing.phase.value) {
                is AddressFundFromAssetLockController.Phase.InFlight,
                is AddressFundFromAssetLockController.Phase.Completed,
                -> return existing // active / just-completed — don't re-enter
                is AddressFundFromAssetLockController.Phase.Idle,
                is AddressFundFromAssetLockController.Phase.Failed,
                -> {
                    // The failure's class forbids this operation (the payment
                    // may have left, or it holds a lock this call does not
                    // resume): keep the failed state instead of paying again.
                    if (!existing.phase.value.permitsSubmit(resuming)) return existing
                    existing.submit(body, resuming)
                    scheduleRetentionSweep(key, existing)
                    return existing
                }
            }
        }
        val controller = AddressFundFromAssetLockController(
            walletId,
            platformAccountIndex,
            recipientHash,
            recipientType,
            scope,
            now,
        )
        _controllers.update { it + (key to controller) }
        controller.submit(body, resuming)
        scheduleRetentionSweep(key, controller)
        return controller
    }

    /**
     * Manually drop a controller from the map — the UI's "Dismiss" action
     * on a [Phase.Failed] row. ← Swift `dismiss`.
     *
     * A failed slot is dropped only when its failure releases it
     * (`PaymentFailure.holdsSlot`, any NOT_SENT): [startFunding] gates a
     * restart only on a slot it still holds, so dropping a failure that may
     * have left, holds a lock, or has no class would let a fresh lock be
     * funded for the same recipient. Such a slot stays until its lock is
     * resumed, the user releases it after reconciling ([release]), or the
     * app restarts. Returns whether the controller was dropped
     * (← [RegistrationCoordinator.dismiss]).
     */
    fun dismiss(walletId: ByteArray, platformAccountIndex: Int, recipientHash: ByteArray): Boolean {
        val key = key(walletId, platformAccountIndex, recipientHash)
        val phase = _controllers.value[key]?.phase?.value
        if (phase is AddressFundFromAssetLockController.Phase.Failed && phase.failure.holdsSlot) return false
        _controllers.update { it - key }
        return true
    }

    /**
     * The exit of a hold [dismiss] refuses, once the user reconciled the
     * original payment from the history: drops a [Phase.Failed] controller
     * only when its failure holds the slot without a held lock
     * (`PaymentFailure.releasableAfterReconciliation`). A held lock is
     * resumed instead. Returns whether the controller was dropped.
     * ← Swift `release`.
     */
    fun release(walletId: ByteArray, platformAccountIndex: Int, recipientHash: ByteArray): Boolean {
        val key = key(walletId, platformAccountIndex, recipientHash)
        val phase = _controllers.value[key]?.phase?.value
        if (phase !is AddressFundFromAssetLockController.Phase.Failed ||
            !phase.failure.releasableAfterReconciliation
        ) {
            return false
        }
        _controllers.update { it - key }
        return true
    }

    /**
     * Auto-purge a [Phase.Completed] controller ~[retentionMillis] after
     * the success transition so the pending list doesn't accumulate stale
     * rows. [Phase.Failed] stays indefinitely until the user dismisses it.
     * Polls every [pollMillis]. ← Swift `scheduleRetentionSweep`.
     */
    private fun scheduleRetentionSweep(
        key: SlotKey,
        controller: AddressFundFromAssetLockController,
    ) {
        scope.launch {
            var completedAt: Long? = null
            while (true) {
                when (controller.phase.value) {
                    is AddressFundFromAssetLockController.Phase.Completed -> {
                        val nowMs = now()
                        if (completedAt == null) {
                            completedAt = nowMs
                        } else if (nowMs - completedAt >= retentionMillis) {
                            _controllers.update { it - key }
                            return@launch
                        }
                    }
                    is AddressFundFromAssetLockController.Phase.Failed -> return@launch
                    else -> completedAt = null
                }
                delay(pollMillis)
            }
        }
    }

    private fun key(
        walletId: ByteArray,
        platformAccountIndex: Int,
        recipientHash: ByteArray,
    ): SlotKey = SlotKey(walletId.toHex(), platformAccountIndex, recipientHash.toHex())

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
