package org.dashfoundation.dashsdk.wallet

/**
 * The private key of one of a wallet's core (Layer-1) addresses, in the two
 * forms a developer screen shows: lowercase hex of the 32-byte secp256k1
 * secret (64 characters) and compressed, network-aware WIF. Port of Swift's
 * `ManagedPlatformWallet.CoreAddressPrivateKey`
 * (`ManagedPlatformWallet.swift`), returned by
 * [ManagedPlatformWallet.coreAddressPrivateKey].
 *
 * Held as [CharArray]s so the holder can wipe them with [clear] when it is
 * done. A screen that renders them as `String`s creates copies the JVM cannot
 * wipe, so keep the object only while the key is on screen. Not a data class:
 * [toString] never prints the key, and equality is identity.
 */
class CoreAddressPrivateKey internal constructor(
    /** Lowercase hex of the raw 32-byte secret. */
    val hex: CharArray,
    /** Compressed, network-aware WIF. */
    val wif: CharArray,
) {
    /** Overwrite both forms with NUL characters. Idempotent. */
    fun clear() {
        hex.fill('\u0000')
        wif.fill('\u0000')
    }

    override fun toString(): String = "CoreAddressPrivateKey(<redacted>)"

    internal companion object {
        /**
         * Decode the `u8 hexLen ‖ hex ‖ u8 wifLen ‖ wif` blob of
         * `AddressPrivateKeyNative.revealCoreAddressPrivateKey`, then zero
         * [blob] whether or not it decoded.
         *
         * @throws IllegalArgumentException for a truncated or overlong blob.
         */
        fun decodeAndWipe(blob: ByteArray): CoreAddressPrivateKey {
            try {
                var offset = 0
                fun field(name: String): CharArray {
                    require(offset < blob.size) { "address private key blob ends before $name" }
                    val length = blob[offset].toInt() and 0xFF
                    offset += 1
                    require(offset + length <= blob.size) {
                        "address private key blob is shorter than its $name length"
                    }
                    // ASCII: one byte per char, no intermediate String.
                    val chars = CharArray(length) { (blob[offset + it].toInt() and 0xFF).toChar() }
                    offset += length
                    return chars
                }
                val hex = field("hex")
                val wif = try {
                    field("WIF")
                } catch (e: IllegalArgumentException) {
                    hex.fill('\u0000')
                    throw e
                }
                if (offset != blob.size) {
                    hex.fill('\u0000')
                    wif.fill('\u0000')
                    throw IllegalArgumentException("address private key blob has trailing bytes")
                }
                return CoreAddressPrivateKey(hex, wif)
            } finally {
                blob.fill(0)
            }
        }
    }
}
