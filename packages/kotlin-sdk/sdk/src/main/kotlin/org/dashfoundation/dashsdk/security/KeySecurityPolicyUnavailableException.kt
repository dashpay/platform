package org.dashfoundation.dashsdk.security

/**
 * Thrown (Kotlin-side, before any FFI call) by identity-key writes when the
 * requested [KeySecurityPolicy.AUTH_GATED] cannot be provisioned with its
 * authentication gate — the device has no secure lock screen — and the host
 * opted into strict mode (`KeystoreManager(requireAuthGated = true)`),
 * refusing the [KeySecurityPolicy.DEVICE_BOUND] fallback.
 *
 * The default (non-strict) behavior instead degrades honestly: new keys are
 * written under [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND] and
 * [KeystoreManager.effectiveKeySecurityPolicy] reports
 * [KeySecurityPolicy.DEVICE_BOUND] — the wallet must work without a screen
 * lock (product decision, dashpay/platform#4060).
 */
class KeySecurityPolicyUnavailableException(
    message: String,
    cause: Throwable? = null,
) : IllegalStateException(message, cause)
