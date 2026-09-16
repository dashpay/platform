package org.dashfoundation.example.services

import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.update
import kotlinx.coroutines.launch

/**
 * Hub for in-flight identity registrations — 1:1 port of
 * `RegistrationCoordinator.swift`. Held on the app container (the Kotlin
 * analog of hosting it on `PlatformWalletManager`) so registrations survive
 * screen dismissal and network-toggle pressure.
 *
 * Keyed by `(walletId, identityIndex)`. The slot model enforces "one
 * registration in flight per identity slot": [startRegistration]
 * single-flights on the slot key, and [hasInFlightRegistrations] gates the
 * Settings network picker.
 *
 * All state lives behind [controllers] (a [StateFlow] map) so Compose
 * observers pick up mutations; the retention sweep runs on [scope] (the
 * app's main-immediate scope) so `.completed` rows auto-purge ~30s after
 * success. [retentionMillis] and [pollMillis] are injectable so tests can
 * drive the sweep on virtual time.
 */
class RegistrationCoordinator(
    private val scope: CoroutineScope,
    private val retentionMillis: Long = 30_000L,
    private val pollMillis: Long = 1_000L,
    private val now: () -> Long = System::currentTimeMillis,
) {
    /** Composite slot key. `walletId` is compared as 32 raw bytes (via hex). ← Swift `SlotKey`. */
    data class SlotKey(val walletIdHex: String, val identityIndex: Int)

    private val _controllers = MutableStateFlow<Map<SlotKey, IdentityRegistrationController>>(emptyMap())

    /** Active controllers keyed by slot, observable by the pending-registrations list. */
    val controllers: StateFlow<Map<SlotKey, IdentityRegistrationController>> = _controllers.asStateFlow()

    /**
     * True when at least one slot is holding itself against a fresh
     * registration — exactly `controller.phase.isActive`
     * ([Phase.PreparingKeys] / [Phase.InFlight] / [Phase.Unconfirmed]). Used
     * by the network picker's enabled gate. ← Swift `hasInFlightRegistrations`.
     */
    val hasInFlightRegistrations: Boolean
        get() = _controllers.value.values.any { it.phase.value.isActive }

    /** The controller for a slot, or null if none is active. ← Swift `controller(walletId:identityIndex:)`. */
    fun controller(walletId: ByteArray, identityIndex: Int): IdentityRegistrationController? =
        _controllers.value[SlotKey(walletId.toHex(), identityIndex)]

    /**
     * Every active controller, most-recently-submitted first — the order the
     * pending-registrations list renders. ← Swift `activeControllers()`.
     */
    fun activeControllers(): List<IdentityRegistrationController> =
        _controllers.value.values.sortedByDescending { it.lastSubmittedAt ?: Long.MIN_VALUE }

    /**
     * Start a registration for the slot, or reuse the existing controller if
     * one is already in flight. Single-flighting is enforced HERE (not just
     * inside the controller) because [IdentityRegistrationController.enterPreparingKeys]
     * unconditionally overwrites the phase — a second tap during the FFI
     * window would otherwise race two calls for the same slot.
     * ← Swift `startRegistration`.
     */
    fun startRegistration(
        walletId: ByteArray,
        identityIndex: Int,
        fundingKind: IdentityRegistrationController.FundingKind = IdentityRegistrationController.FundingKind.AssetLock,
        body: suspend () -> ByteArray,
        isUnconfirmed: (Throwable) -> ByteArray? = { null },
        isBroadcastRejected: (Throwable) -> Boolean = { false },
    ): IdentityRegistrationController {
        val key = SlotKey(walletId.toHex(), identityIndex)
        val existing = _controllers.value[key]
        if (existing != null) {
            when (existing.phase.value) {
                is IdentityRegistrationController.Phase.PreparingKeys,
                is IdentityRegistrationController.Phase.InFlight,
                is IdentityRegistrationController.Phase.Completed,
                is IdentityRegistrationController.Phase.Unconfirmed,
                -> return existing // active / just-completed / unconfirmed — don't re-enter
                is IdentityRegistrationController.Phase.Idle,
                is IdentityRegistrationController.Phase.Failed,
                -> {
                    // Legitimate restart: a fresh idle controller, or a retry after failure.
                    existing.enterPreparingKeys()
                    existing.submit(body, isUnconfirmed, isBroadcastRejected)
                    scheduleRetentionSweep(key, existing)
                    return existing
                }
            }
        }
        val controller = IdentityRegistrationController(walletId, identityIndex, fundingKind, scope, now)
        _controllers.update { it + (key to controller) }
        controller.enterPreparingKeys()
        controller.submit(body, isUnconfirmed, isBroadcastRejected)
        scheduleRetentionSweep(key, controller)
        return controller
    }

    /**
     * Manually drop a controller from the map — the UI's "Dismiss" action on
     * a [Phase.Failed] (or a synced [Phase.Unconfirmed]) row. ← Swift `dismiss`.
     */
    fun dismiss(walletId: ByteArray, identityIndex: Int) {
        val key = SlotKey(walletId.toHex(), identityIndex)
        _controllers.update { it - key }
    }

    /**
     * Auto-purge a [Phase.Completed] controller ~[retentionMillis] after the
     * success transition so the pending list doesn't accumulate stale rows.
     * [Phase.Failed] / [Phase.Unconfirmed] stay indefinitely until the user
     * dismisses them. Polls every [pollMillis]. ← Swift `scheduleRetentionSweep`.
     */
    private fun scheduleRetentionSweep(key: SlotKey, controller: IdentityRegistrationController) {
        scope.launch {
            var completedAt: Long? = null
            while (true) {
                when (controller.phase.value) {
                    is IdentityRegistrationController.Phase.Completed -> {
                        val nowMs = now()
                        if (completedAt == null) {
                            completedAt = nowMs
                        } else if (nowMs - completedAt >= retentionMillis) {
                            _controllers.update { it - key }
                            return@launch
                        }
                    }
                    is IdentityRegistrationController.Phase.Failed,
                    is IdentityRegistrationController.Phase.Unconfirmed,
                    -> return@launch // keep indefinitely; user dismisses manually
                    else -> completedAt = null
                }
                delay(pollMillis)
            }
        }
    }

    private fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it) }
}
