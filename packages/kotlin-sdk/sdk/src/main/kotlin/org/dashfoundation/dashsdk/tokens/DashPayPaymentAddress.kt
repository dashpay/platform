package org.dashfoundation.dashsdk.tokens

import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.DashpayNative
import org.dashfoundation.dashsdk.ffi.NativeLoader

/**
 * Which of the three public payment addresses of a DashPay profile a value
 * belongs to. [ffiValue] is the FFI's `DASHPAY_PROFILE_PAYMENT_ADDRESS_*`
 * kind. Port of Swift `DashPayPaymentAddressKind`
 * (`SwiftDashSDK/PlatformWallet/DashPayPaymentAddress.swift`).
 */
enum class DashPayPaymentAddressKind(val ffiValue: Int) {
    /** `corePaymentAddress`, shown as a Base58Check Core address. */
    CORE(0),

    /** `platformPaymentAddress`, shown as a bech32m Platform address. */
    PLATFORM(1),

    /** `shieldedAddress`, shown as a bech32m Orchard address. */
    SHIELDED(2),
}

/**
 * Which payment addresses a profile write may set on the connected network,
 * as `platform-wallet` reports it. Removing an address needs no support.
 * Port of Swift `DashPayPaymentAddressSupport`.
 */
data class DashPayPaymentAddressSupport(
    val core: Boolean,
    val platform: Boolean,
    val shielded: Boolean,
) {
    /** Whether an address of [kind] can be set. */
    fun supports(kind: DashPayPaymentAddressKind): Boolean = when (kind) {
        DashPayPaymentAddressKind.CORE -> core
        DashPayPaymentAddressKind.PLATFORM -> platform
        DashPayPaymentAddressKind.SHIELDED -> shielded
    }

    internal companion object {
        /** The JNI's `[core, platform, shielded]` flags. */
        fun fromFlags(flags: BooleanArray): DashPayPaymentAddressSupport {
            check(flags.size == 3) { "expected 3 support flags, got ${flags.size}" }
            return DashPayPaymentAddressSupport(flags[0], flags[1], flags[2])
        }
    }
}

/**
 * What a profile write does with one payment address: keep the published
 * value, set it to stored bytes, or remove it. The wallet checks the bytes of
 * every [Set] for its field. Port of Swift `DashPayPaymentAddressUpdate`.
 */
sealed class DashPayPaymentAddressUpdate {
    /** Leave the published address as it is. */
    data object Keep : DashPayPaymentAddressUpdate()

    /** Remove the published address. */
    data object Remove : DashPayPaymentAddressUpdate()

    /** Publish [bytes], the stored form from [DashPayPaymentAddresses.bytes]. */
    class Set(val bytes: ByteArray) : DashPayPaymentAddressUpdate() {
        override fun equals(other: Any?): Boolean = other is Set && bytes.contentEquals(other.bytes)

        override fun hashCode(): Int = bytes.contentHashCode()
    }

    /** `PaymentAddressUpdateFFI.action`: 0 keep, 1 set, 2 remove. */
    internal val ffiAction: Int
        get() = when (this) {
            Keep -> 0
            is Set -> 1
            Remove -> 2
        }

    internal val ffiBytes: ByteArray?
        get() = (this as? Set)?.bytes
}

/**
 * The conversion between the address strings people read and paste and the
 * bytes a profile stores, done by `platform-wallet`. Port of Swift
 * `DashPayProfile.paymentAddressBytes(kind:from:network:)` and
 * `paymentAddressString(kind:bytes:network:)`.
 */
object DashPayPaymentAddresses {
    /**
     * The bytes a profile stores for the address [text] of [kind] on
     * [network]. The wallet parses and checks the address, and refuses one of
     * another network or kind.
     *
     * @throws org.dashfoundation.dashsdk.errors.DashSdkError carrying the
     *   wallet's readable reason.
     */
    fun bytes(kind: DashPayPaymentAddressKind, text: String, network: Network): ByteArray {
        NativeLoader.ensureLoaded()
        val bytes = mapNativeErrors {
            DashpayNative.paymentAddressFromString(kind.ffiValue, text, network.ffiValue)
        }
        return checkNotNull(bytes) { "native paymentAddressFromString returned no bytes" }
    }

    /**
     * The address string of [network] for the stored [bytes] of a [kind]
     * address, or null when the wallet does not read them as an address of
     * that kind.
     */
    fun string(kind: DashPayPaymentAddressKind, bytes: ByteArray, network: Network): String? {
        NativeLoader.ensureLoaded()
        return mapNativeErrors {
            DashpayNative.paymentAddressToString(kind.ffiValue, bytes, network.ffiValue)
        }
    }
}
