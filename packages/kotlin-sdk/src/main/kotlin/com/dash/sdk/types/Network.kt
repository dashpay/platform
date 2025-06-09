package com.dash.sdk.types

/**
 * Represents the different Dash networks
 */
enum class Network(val value: Int) {
    /**
     * Dash mainnet
     */
    MAINNET(0),

    /**
     * Dash testnet
     */
    TESTNET(1),

    /**
     * Dash devnet
     */
    DEVNET(2),

    /**
     * Local development network
     */
    LOCAL(3);

    companion object {
        fun fromValue(value: Int): Network = values().find { it.value == value }
            ?: throw IllegalArgumentException("Unknown network value: $value")
    }
}