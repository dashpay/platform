package org.dashfoundation.example.ui.wallet

import org.junit.Assert.assertEquals
import org.junit.Test

class WalletKeyHealthStatusTest {

    @Test
    fun `should leave a hecate signer key without a private key or wallet slot unchecked`() {
        // An externally generated Hecate signer key: the wallet stores no slot
        // for a key type it doesn't derive, and no private key was persisted.
        assertEquals(
            WalletKeyHealthStatus.NotChecked,
            walletKeyHealthStatus(
                isRecoverable = false,
                derivationIdentityIndex = null,
                derivationKeyIndex = null,
            ),
        )
    }

    @Test
    fun `should offer a repair only for a key with its whole wallet slot recorded`() {
        assertEquals(
            WalletKeyHealthStatus.Repairable,
            walletKeyHealthStatus(
                isRecoverable = false,
                derivationIdentityIndex = 1,
                derivationKeyIndex = 2,
            ),
        )
        // The repair needs both indices, so half a slot can't be repaired.
        assertEquals(
            WalletKeyHealthStatus.NotChecked,
            walletKeyHealthStatus(
                isRecoverable = false,
                derivationIdentityIndex = 1,
                derivationKeyIndex = null,
            ),
        )
        assertEquals(
            WalletKeyHealthStatus.NotChecked,
            walletKeyHealthStatus(
                isRecoverable = false,
                derivationIdentityIndex = null,
                derivationKeyIndex = 2,
            ),
        )
    }

    @Test
    fun `should call a key whose private key opens recoverable with or without a slot`() {
        assertEquals(
            WalletKeyHealthStatus.Recoverable,
            walletKeyHealthStatus(
                isRecoverable = true,
                derivationIdentityIndex = null,
                derivationKeyIndex = null,
            ),
        )
        assertEquals(
            WalletKeyHealthStatus.Recoverable,
            walletKeyHealthStatus(
                isRecoverable = true,
                derivationIdentityIndex = 1,
                derivationKeyIndex = 2,
            ),
        )
    }
}
