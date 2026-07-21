package org.dashfoundation.dashsdk.security

/**
 * Security policy for the Keystore alias that wraps **identity private
 * keys** ([WalletStorage.storePrivateKey] / [WalletStorage.retrievePrivateKey]).
 *
 * The SDK's default alias gates every identity-key decrypt behind Android
 * user authentication (biometric / device credential) with a short
 * post-unlock validity window — the right model when the SDK owns the
 * auth UX. Host apps that already gate wallet access behind their own
 * auth model (e.g. an app-level PIN that decrypts the wallet) end up with
 * a *second*, redundant auth prompt at signing time — and signing fails
 * outside the ~30 s window entirely when no [BiometricGate] is wired.
 * [DEVICE_BOUND] lets such hosts opt into a non-gated (but still
 * hardware-backed, non-exportable) wrapping key instead.
 *
 * ## Semantics
 *
 * - [AUTH_GATED] — the default; behavior matches the historical one. New
 *   identity keys are wrapped under [KeystoreManager.KEYS_ALIAS_AUTH_GATED]
 *   (the legacy [KeystoreManager.KEYS_ALIAS] is kept read-only so pre-RSA
 *   blobs migrate rather than strand): encrypt (store) never prompts, decrypt
 *   (sign) requires user authentication within
 *   [KeystoreManager.AUTH_VALIDITY_SECONDS] of a biometric / device-credential
 *   auth, re-promptable through a [BiometricGate].
 * - [DEVICE_BOUND] — identity keys are wrapped under the separate
 *   [KeystoreManager.KEYS_ALIAS_DEVICE_BOUND] alias: the same
 *   StrongBox-preferring, non-exportable RSA wrapping pair, but with **no**
 *   `setUserAuthenticationRequired` gate, so decrypts never throw
 *   `UserNotAuthenticatedException` and never need a [BiometricGate].
 *   Keys remain bound to this device's Keystore (`setUnlockedDeviceRequired`
 *   still applies) — the host app is responsible for gating *access* to
 *   signing flows (PIN, biometrics, session policy) itself.
 *
 * ## Choosing and switching
 *
 * The two policies use **distinct Keystore aliases**, and a blob written
 * under one alias can only be decrypted by that alias's private key. Pick
 * the policy once per install and construct [WalletStorage] /
 * [KeystoreManager] with it consistently: switching an existing install to
 * the other policy leaves previously stored identity keys undecryptable
 * (they surface through the key-health / re-derive path, e.g.
 * `PlatformWalletManager.repairIdentityKey`, which re-encrypts under the
 * current policy's alias). Mnemonics ([KeystoreManager.MASTER_ALIAS]) are
 * unaffected — this policy governs identity keys only.
 *
 * ## Lockless-device degradation (dashpay/platform#4060)
 *
 * [AUTH_GATED]'s authentication gate requires a secure lock screen to exist
 * — Android KeyMint rejects generating the gated key otherwise. The wallet
 * must still work without a screen lock (product decision), so on a
 * lockless device the SDK does NOT silently generate a gate-less key under
 * the auth-gated alias; new identity keys are written under the
 * [DEVICE_BOUND] alias instead, and the degradation is surfaced honestly
 * via [KeystoreManager.effectiveKeySecurityPolicy] /
 * [WalletStorage.effectiveKeySecurityPolicy]. Each blob records the alias
 * that produced it, so keys written during a lockless period remain
 * readable after a lock screen is later enrolled (new writes then move to
 * the gated alias). Hosts that must never degrade can construct
 * `KeystoreManager(requireAuthGated = true)`, which throws
 * [KeySecurityPolicyUnavailableException] instead of degrading.
 */
enum class KeySecurityPolicy {
    /**
     * Identity-key decrypts require Android user authentication within
     * [KeystoreManager.AUTH_VALIDITY_SECONDS] (the historical default).
     */
    AUTH_GATED,

    /**
     * Identity-key decrypts are hardware-backed but not auth-gated; the
     * host app supplies its own authentication model.
     */
    DEVICE_BOUND,
}
