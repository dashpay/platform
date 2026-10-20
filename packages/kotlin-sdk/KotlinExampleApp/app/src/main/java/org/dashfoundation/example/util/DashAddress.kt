package org.dashfoundation.example.util

import org.dashfoundation.dashsdk.Network

/**
 * Recipient-address family detection + Orchard display encoding — port of
 * `DashAddress.swift` (SwiftExampleApp/Core/Models). Drives the send
 * screen's flow routing (Core / Platform / Shielded recipient) and the
 * Receive sheet's shielded tab.
 *
 * Detection only: a matched address is re-validated authoritatively on the
 * Rust side when the transition is built (`PlatformAddress::from_bech32m_string`,
 * `OrchardAddress::from_raw_bytes`, Core Base58Check parsing). The Core arm
 * mirrors the send screen's original light check (Base58 payload length)
 * because the network-aware `Address.validate` FFI isn't bridged on Android.
 */
sealed interface DashAddressType {
    /** Base58Check Core L1 address (25-byte decoded payload). */
    data class Core(val address: String) : DashAddressType

    /** 21-byte bech32m Platform payload: type byte (0xb0/0x80) + 20-byte hash. */
    data class Platform(val payload21: ByteArray) : DashAddressType

    /** 43-byte raw Orchard payment address (11-byte diversifier + 32-byte pk_d). */
    data class Orchard(val raw43: ByteArray) : DashAddressType

    data object Unknown : DashAddressType
}

object DashAddress {

    /** Bech32m type byte marking an Orchard payload (← DashAddress.swift:41). */
    private const val ORCHARD_TYPE_BYTE = 0x10

    /** Bech32m type bytes marking a Platform payload (← DashAddress.swift:34). */
    private const val PLATFORM_P2PKH_TYPE_BYTE = 0xb0
    private const val PLATFORM_P2SH_TYPE_BYTE = 0x80

    /** DIP-0018 HRP: `dash` on mainnet, `tdash` everywhere else. */
    fun hrp(network: Network): String =
        if (network == Network.MAINNET) "dash" else "tdash"

    /**
     * Parse any address string and detect its family — port of
     * `DashAddress.parse(_:network:)`. Platform and Orchard share the
     * dash/tdash HRP and are distinguished by the payload's leading type
     * byte (0xb0/0x80 = platform, 0x10 = orchard).
     */
    fun parse(input: String, network: Network): DashAddressType {
        val trimmed = input.trim()
        if (trimmed.isEmpty()) return DashAddressType.Unknown

        // 1. bech32m (Platform / Orchard).
        val decoded = Bech32m.decode(trimmed)
        if (decoded != null && decoded.hrp == hrp(network)) {
            val data = decoded.data
            if (data.size == 21) {
                val typeByte = data[0].toInt() and 0xff
                if (typeByte == PLATFORM_P2PKH_TYPE_BYTE || typeByte == PLATFORM_P2SH_TYPE_BYTE) {
                    return DashAddressType.Platform(data)
                }
            }
            if (data.size == 44 && (data[0].toInt() and 0xff) == ORCHARD_TYPE_BYTE) {
                return DashAddressType.Orchard(data.copyOfRange(1, 44))
            }
            return DashAddressType.Unknown
        }

        // 2. Core Base58Check — light check (version + hash160 + checksum
        //    = 25 bytes), same validation SendTransactionScreen shipped with.
        if (Base58.decode(trimmed)?.size == 25) {
            return DashAddressType.Core(trimmed)
        }

        return DashAddressType.Unknown
    }

    /**
     * Encode a raw 43-byte Orchard address as its bech32m display string —
     * port of `DashAddress.encodeOrchard(rawBytes:network:)`: prepend the
     * 0x10 type byte, then bech32m-encode under the dash/tdash HRP.
     * Returns null for a wrong-length input.
     */
    fun encodeOrchard(rawBytes: ByteArray, network: Network): String? {
        if (rawBytes.size != 43) return null
        return Bech32m.encode(hrp(network), byteArrayOf(ORCHARD_TYPE_BYTE.toByte()) + rawBytes)
    }
}
