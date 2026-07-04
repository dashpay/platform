package org.dashfoundation.dashsdk.security

/**
 * Authentication gate for sensitive operations (mnemonic reveal, private
 * key access outside the Keystore auth window, send confirmation).
 *
 * The SDK defines the interface; the app supplies the Activity-bound
 * `BiometricPrompt` implementation (`AuthPrompt`), mirroring how the iOS
 * SDK leaves `LAContext` presentation to the app layer.
 */
interface BiometricGate {

    /** Mirror of the iOS `AuthOutcome` enum used by the recovery flows. */
    enum class AuthOutcome { AUTHORIZED, DENIED, UNAVAILABLE, FAILED }

    /**
     * Prompt the user to authenticate (biometric or device credential).
     * [title]/[subtitle] name the operation being authorized.
     */
    suspend fun authenticate(title: String, subtitle: String? = null): AuthOutcome
}
