package org.dashfoundation.example.services.auth

import android.app.Activity
import android.app.KeyguardManager
import android.content.Intent
import android.os.Build
import android.os.SystemClock
import androidx.activity.result.ActivityResultLauncher
import androidx.activity.result.contract.ActivityResultContracts
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext
import org.dashfoundation.dashsdk.security.BiometricGate
import kotlin.coroutines.resume

/**
 * Activity-bound BiometricPrompt implementation of the SDK's
 * [BiometricGate] — the Android counterpart of the `LAContext`
 * `evaluatePolicy(.deviceOwnerAuthentication)` flows in
 * `WalletDetailView.authorizeAndRevealMnemonic` and
 * `ContentView.runAuthPrompt` (orphan recovery).
 *
 * Requests a STRONG (class-3) biometric OR device credential — matching the
 * identity-key alias policy (`KEYS_ALIAS` uses
 * `AUTH_BIOMETRIC_STRONG | AUTH_DEVICE_CREDENTIAL`), so a successful prompt
 * actually refreshes the key's auth window (= iOS `.deviceOwnerAuthentication`,
 * which also falls back to passcode).
 * Outcome mapping mirrors the Swift `AuthOutcome` enum: user cancel →
 * [BiometricGate.AuthOutcome.DENIED], missing hardware / enrollment →
 * [BiometricGate.AuthOutcome.UNAVAILABLE], everything else →
 * [BiometricGate.AuthOutcome.FAILED].
 *
 * Android 10 (API 29, the app's minimum) does not support a BiometricPrompt
 * that allows both a strong biometric and the device credential
 * (`canAuthenticate` reports it unsupported). There the gate confirms the
 * device credential instead (`KeyguardManager.createConfirmDeviceCredentialIntent`),
 * which also refreshes the auth window of the identity-key alias, so a device
 * with only a PIN can still authenticate and no weak biometric is accepted.
 */
class AuthPrompt(private val activity: FragmentActivity) : BiometricGate {

    override suspend fun authenticate(
        title: String,
        subtitle: String?,
    ): BiometricGate.AuthOutcome {
        // STRONG (class-3), not WEAK: the identity-key alias is generated for
        // AUTH_BIOMETRIC_STRONG | AUTH_DEVICE_CREDENTIAL, so a weak (class-2)
        // biometric would report success here yet leave the subsequent
        // private-key decrypt unauthorized (the key's window never refreshes).
        val authenticators = BiometricManager.Authenticators.BIOMETRIC_STRONG or
            BiometricManager.Authenticators.DEVICE_CREDENTIAL

        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.R) {
            return confirmDeviceCredential(title, subtitle)
        }

        when (BiometricManager.from(activity).canAuthenticate(authenticators)) {
            BiometricManager.BIOMETRIC_SUCCESS -> Unit
            else -> return BiometricGate.AuthOutcome.UNAVAILABLE
        }

        // BiometricPrompt must be driven from the main thread.
        return withContext(Dispatchers.Main.immediate) {
            suspendCancellableCoroutine { continuation ->
                val prompt = BiometricPrompt(
                    activity,
                    ContextCompat.getMainExecutor(activity),
                    object : BiometricPrompt.AuthenticationCallback() {
                        override fun onAuthenticationSucceeded(
                            result: BiometricPrompt.AuthenticationResult,
                        ) {
                            if (continuation.isActive) {
                                continuation.resume(BiometricGate.AuthOutcome.AUTHORIZED)
                            }
                        }

                        override fun onAuthenticationError(
                            errorCode: Int,
                            errString: CharSequence,
                        ) {
                            val outcome = when (errorCode) {
                                BiometricPrompt.ERROR_NEGATIVE_BUTTON,
                                BiometricPrompt.ERROR_USER_CANCELED,
                                BiometricPrompt.ERROR_CANCELED,
                                -> BiometricGate.AuthOutcome.DENIED

                                BiometricPrompt.ERROR_HW_NOT_PRESENT,
                                BiometricPrompt.ERROR_HW_UNAVAILABLE,
                                BiometricPrompt.ERROR_NO_BIOMETRICS,
                                BiometricPrompt.ERROR_NO_DEVICE_CREDENTIAL,
                                -> BiometricGate.AuthOutcome.UNAVAILABLE

                                else -> BiometricGate.AuthOutcome.FAILED
                            }
                            if (continuation.isActive) continuation.resume(outcome)
                        }

                        // onAuthenticationFailed is a transient "try again"
                        // (bad fingerprint read); the prompt stays up, so
                        // don't resume here.
                    },
                )

                val info = BiometricPrompt.PromptInfo.Builder()
                    .setTitle(title)
                    .apply { subtitle?.let { setSubtitle(it) } }
                    .setAllowedAuthenticators(authenticators)
                    .build()

                prompt.authenticate(info)
                continuation.invokeOnCancellation { prompt.cancelAuthentication() }
            }
        }
    }

    /**
     * API 29 path: the system's confirm-credential screen (PIN, pattern or
     * password). Cancelling it is [BiometricGate.AuthOutcome.DENIED]; a device
     * without a secure lock screen is [BiometricGate.AuthOutcome.UNAVAILABLE].
     */
    private suspend fun confirmDeviceCredential(
        title: String,
        subtitle: String?,
    ): BiometricGate.AuthOutcome {
        val keyguard = activity.getSystemService(KeyguardManager::class.java)
        if (keyguard?.isDeviceSecure != true) return BiometricGate.AuthOutcome.UNAVAILABLE
        @Suppress("DEPRECATION")
        val intent = keyguard.createConfirmDeviceCredentialIntent(title, subtitle)
            ?: return BiometricGate.AuthOutcome.UNAVAILABLE
        return withContext(Dispatchers.Main.immediate) {
            suspendCancellableCoroutine { continuation ->
                lateinit var launcher: ActivityResultLauncher<Intent>
                launcher = activity.activityResultRegistry.register(
                    "dash.confirmDeviceCredential.${SystemClock.elapsedRealtimeNanos()}",
                    ActivityResultContracts.StartActivityForResult(),
                ) { result ->
                    launcher.unregister()
                    if (continuation.isActive) {
                        continuation.resume(
                            if (result.resultCode == Activity.RESULT_OK) {
                                BiometricGate.AuthOutcome.AUTHORIZED
                            } else {
                                BiometricGate.AuthOutcome.DENIED
                            },
                        )
                    }
                }
                continuation.invokeOnCancellation { launcher.unregister() }
                launcher.launch(intent)
            }
        }
    }
}

/**
 * Settable indirection so the [org.dashfoundation.example.di.AppContainer]
 * (Application-scoped, constructed before any Activity exists) can hand a
 * stable [BiometricGate] to `WalletManagerStore` / `KeystoreSigner` at
 * construction, with MainActivity binding the real [AuthPrompt] once it is
 * created — the injection point the AppContainer KDoc reserved for B-M2.
 */
class DelegatingBiometricGate : BiometricGate {

    @Volatile
    var delegate: BiometricGate? = null

    override suspend fun authenticate(
        title: String,
        subtitle: String?,
    ): BiometricGate.AuthOutcome =
        delegate?.authenticate(title, subtitle)
            ?: BiometricGate.AuthOutcome.UNAVAILABLE
}
