package org.dashfoundation.dashsdk.security

import java.security.GeneralSecurityException

/**
 * `KeyguardManager` lock state sampled at a single instant — carried by
 * [KeystoreDeviceLockedException] so a log line can distinguish a
 * genuinely-locked device from the false-locked Keystore2 defect (see the
 * exception's KDoc).
 *
 * - [isDeviceLocked] — `KeyguardManager.isDeviceLocked`: the device is locked
 *   AND secured (a credential is needed to get in). This is the signal the
 *   Keystore's `setUnlockedDeviceRequired` gate is SUPPOSED to track.
 * - [isKeyguardLocked] — `KeyguardManager.isKeyguardLocked`: the keyguard
 *   (lock screen) is showing, secured or not. Can be true while
 *   [isDeviceLocked] is false (e.g. a non-secure swipe screen).
 */
data class DeviceLockState(
    val isDeviceLocked: Boolean,
    val isKeyguardLocked: Boolean,
)

/**
 * The Android Keystore denied an operation on a lock-screen-bound key
 * because ITS device-locked tracking says the device is locked — thrown by
 * [KeystoreManager.encrypt] / [KeystoreManager.decrypt] for the aliases
 * that carry `setUnlockedDeviceRequired(true)` on lock-screen devices and
 * NO `setUserAuthenticationRequired` gate, so a Keystore "user not
 * authenticated" denial there can only mean the device-locked gate
 * ([KeystoreManager.MASTER_ALIAS], the AES key; and
 * [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND], the non-auth-gated identity
 * keypair — MO-972), and by the `PlatformWalletManager.createWallet`
 * pre-check before any native wallet exists.
 *
 * **RETRYABLE AFTER UNLOCK.** This is never a permanent failure of the key
 * or the data: the exact same operation succeeds once the Keystore
 * considers the device unlocked. Callers should retry rather than treat the
 * secret as lost.
 *
 * [lockState] is `KeyguardManager`'s view sampled AT THROW TIME, so field
 * logs can separate the two classes this exception covers:
 *
 * - **Genuinely locked** ([DeviceLockState.isDeviceLocked] true): the
 *   denial is correct. Retrying before the user unlocks cannot succeed —
 *   fail fast and retry after the next unlock.
 * - **False-locked** ([DeviceLockState.isDeviceLocked] false): the defect
 *   observed in the field (two QA devices, wallet creation) — the device is
 *   demonstrably unlocked but Keystore2's internal lock-state tracking
 *   still says "locked". A short bounded retry is worthwhile (see
 *   [WalletStorage.storeMnemonic]); recurrence past that schedule means
 *   the defect is PERSISTENT for the unlock session (an OEM unlock class
 *   that never satisfies `UNLOCKED_DEVICE_REQUIRED` — observed on
 *   HONOR/MagicOS Android 16; same mechanism as Google Issue Tracker
 *   506989112), at which point [WalletStorage.storeMnemonic] degrades the
 *   write to the never-lock-bound
 *   [KeystoreManager.MASTER_ALIAS_UNBOUND] instead of throwing this — so
 *   escaping this exception false-locked now means even that degradation
 *   failed (see the suppressed exception).
 *
 * NOT used for [KeystoreManager.KEYS_ALIAS_AUTH_GATED]: it carries BOTH
 * gates, so the same `UserNotAuthenticatedException` may equally mean
 * "auth window closed", and only that reading is fixable by prompting —
 * it keeps its prompt-and-retry contract via `BiometricGate` (see
 * [KeystoreManager.decrypt]). Nor for the `*_UNBOUND` aliases, which carry
 * neither gate, so a denial there is not a lock denial at all.
 */
class KeystoreDeviceLockedException(
    /** Keystore alias whose operation was denied (or would be, for the pre-check). */
    val alias: String,
    /** The denied operation: `"encrypt"`, `"decrypt"`, or a pre-check name. */
    val operation: String,
    /** `KeyguardManager` lock state sampled when this was thrown. */
    val lockState: DeviceLockState,
    cause: Throwable? = null,
) : GeneralSecurityException(
    "Keystore denied '$operation' on lock-bound alias '$alias' as device-locked; " +
        "KeyguardManager at throw time: isDeviceLocked=${lockState.isDeviceLocked} " +
        "isKeyguardLocked=${lockState.isKeyguardLocked}" +
        (if (lockState.isDeviceLocked) {
            " (genuinely locked — retry after unlock)"
        } else {
            " (FALSE-LOCKED: device reports unlocked but Keystore denied — " +
                "Keystore2 lock-state misreporting; retryable immediately)"
        }),
    cause,
) {
    /**
     * Whether `KeyguardManager` agreed the device was locked when this was
     * thrown. False identifies the false-locked defect class — the device
     * was demonstrably unlocked, so a short bounded retry may succeed.
     */
    val deviceReportsLocked: Boolean get() = lockState.isDeviceLocked
}
