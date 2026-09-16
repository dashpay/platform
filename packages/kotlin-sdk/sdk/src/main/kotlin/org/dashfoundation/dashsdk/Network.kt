package org.dashfoundation.dashsdk

/**
 * The Dash network the SDK is operating on.
 *
 * Kotlin mirror of `dash-network`'s `FFINetwork` C-ABI enum — the single
 * network type every Rust FFI surface takes/returns. Port of
 * `SwiftDashSDK/DashNetwork.swift`; [ffiValue] ordinals must stay in sync
 * with `dash-network/src/ffi.rs`.
 */
enum class Network(val ffiValue: Int) {
    MAINNET(0),
    TESTNET(1),
    DEVNET(2),
    REGTEST(3);

    /** User-facing name — matches the iOS `displayName` strings. */
    val displayName: String
        get() = when (this) {
            MAINNET -> "Mainnet"
            TESTNET -> "Testnet"
            DEVNET -> "Devnet"
            REGTEST -> "Local"
        }

    /** Canonical lowercase name used in persistence keys and logs. */
    val networkName: String
        get() = when (this) {
            MAINNET -> "mainnet"
            TESTNET -> "testnet"
            DEVNET -> "devnet"
            REGTEST -> "regtest"
        }

    companion object {
        val DEFAULT: Network = TESTNET

        fun fromFfiValue(value: Int): Network =
            entries.firstOrNull { it.ffiValue == value }
                ?: throw IllegalArgumentException("Invalid FFINetwork value: $value")

        fun fromNetworkName(name: String): Network? =
            entries.firstOrNull { it.networkName == name.lowercase() }
    }
}
