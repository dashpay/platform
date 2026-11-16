package org.dashfoundation.dashsdk.tokens

/** Explicit partial-update semantics for a public profile payment address. */
sealed class PaymentAddressUpdate(internal val action: Int, internal val bytes: ByteArray?) {
    data object Keep : PaymentAddressUpdate(0, null)
    class Set(bytes: ByteArray) : PaymentAddressUpdate(1, bytes.copyOf())
    data object Remove : PaymentAddressUpdate(2, null)
}
