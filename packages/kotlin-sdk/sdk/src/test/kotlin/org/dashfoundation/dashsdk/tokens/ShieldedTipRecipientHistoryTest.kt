package org.dashfoundation.dashsdk.tokens

import android.content.Context
import androidx.test.core.app.ApplicationProvider
import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

@RunWith(RobolectricTestRunner::class)
class ShieldedTipRecipientHistoryTest {
    private val preferences = ApplicationProvider.getApplicationContext<Context>()
        .getSharedPreferences("tip-recipient-history-test", Context.MODE_PRIVATE)
    private val walletId = ByteArray(32) { 1 }
    private val recipient = ShieldedTipRecipient(ByteArray(32) { 2 }, ByteArray(43) { 3 })
    private val history get() = ShieldedTipRecipientHistory(preferences)

    @Before
    fun reset() {
        preferences.edit().clear().commit()
    }

    @Test
    fun confirmationSurvivesReopeningAndCanonicalSpellings() = runTest {
        assertFalse(history.hasChanged(Network.TESTNET, walletId, "Alice", recipient))
        history.confirm(Network.TESTNET, walletId, " Alice.DASH ", recipient)
        assertFalse(history.hasChanged(Network.TESTNET, walletId, "a11ce", recipient))
        val changed = ShieldedTipRecipient(recipient.identityId, ByteArray(43) { 4 })
        assertTrue(history.hasChanged(Network.TESTNET, walletId, "a11ce.dash", changed))
    }

    @Test
    fun detectsBothIdentityAndAddressReplacementWithoutChangingThePin() = runTest {
        history.confirm(Network.TESTNET, walletId, "Alice", recipient)
        val newIdentity = ShieldedTipRecipient(ByteArray(32) { 4 }, recipient.address)
        val newAddress = ShieldedTipRecipient(recipient.identityId, ByteArray(43) { 5 })
        assertTrue(history.hasChanged(Network.TESTNET, walletId, "Alice", newIdentity))
        assertTrue(history.hasChanged(Network.TESTNET, walletId, "Alice", newAddress))
        assertFalse(history.hasChanged(Network.TESTNET, walletId, "Alice", recipient))
        history.confirm(Network.TESTNET, walletId, "Alice", newAddress)
        assertFalse(history.hasChanged(Network.TESTNET, walletId, "Alice", newAddress))
        assertTrue(history.hasChanged(Network.TESTNET, walletId, "Alice", recipient))
    }

    @Test
    fun isolatesNetworksWalletsAndUsernames() = runTest {
        history.confirm(Network.TESTNET, walletId, "Alice", recipient)
        val changed = ShieldedTipRecipient(ByteArray(32) { 4 }, recipient.address)
        assertFalse(history.hasChanged(Network.MAINNET, walletId, "Alice", changed))
        assertFalse(history.hasChanged(Network.TESTNET, ByteArray(32) { 6 }, "Alice", changed))
        assertFalse(history.hasChanged(Network.TESTNET, walletId, "Bob", changed))
        assertTrue(history.hasChanged(Network.TESTNET, walletId, "Alice", changed))
    }
}
