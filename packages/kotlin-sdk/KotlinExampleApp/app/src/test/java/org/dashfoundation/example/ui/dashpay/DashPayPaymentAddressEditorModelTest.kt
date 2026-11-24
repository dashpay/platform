package org.dashfoundation.example.ui.dashpay

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressKind
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressSupport
import org.dashfoundation.dashsdk.tokens.DashPayPaymentAddressUpdate
import org.dashfoundation.example.ui.dashpay.DashPayPaymentAddressEditorModel.Mode
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The editor's state rules, ported from Swift's
 * `DashPayPaymentAddressEditorModel` (`DashPayPaymentAddressesSection.swift`).
 * The wallet's parse is stubbed: accepting or refusing an address is
 * platform-wallet's decision and is covered by its own tests and by
 * `FfiSmokeTest.profilePaymentAddressSymbolsBindAndConvert`.
 */
class DashPayPaymentAddressEditorModelTest {

    private val core = DashPayPaymentAddressKind.CORE
    private val platform = DashPayPaymentAddressKind.PLATFORM
    private val shielded = DashPayPaymentAddressKind.SHIELDED
    private val good = ByteArray(21) { 1 }

    private fun model(existing: DashPayProfile? = null) = DashPayPaymentAddressEditorModel(existing) { _, text, _ ->
        if (text == "good") good else throw IllegalArgumentException("'$text' is not an address for Testnet")
    }

    private suspend fun DashPayPaymentAddressEditorModel.startWith(support: DashPayPaymentAddressSupport) =
        start(Network.TESTNET) { support }

    @Test
    fun shouldKeepEveryAddressByDefault() = runTest {
        val m = model(DashPayProfile(null, null, null, corePaymentAddress = good))
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        for (kind in DashPayPaymentAddressKind.entries) {
            assertEquals(DashPayPaymentAddressUpdate.Keep, m.update(kind))
        }
        assertFalse(m.hasIncompleteSet)
    }

    @Test
    fun shouldOfferRemoveOnlyForAPublishedAddress() {
        val m = model(DashPayProfile(null, null, null, platformPaymentAddress = good))
        assertEquals(listOf(Mode.KEEP, Mode.SET), m.availableModes(core))
        assertEquals(listOf(Mode.KEEP, Mode.SET, Mode.REMOVE), m.availableModes(platform))
    }

    @Test
    fun shouldRefuseSetForAFieldTheNetworkDoesNotSupportButAllowRemove() = runTest {
        val m = model(DashPayProfile(null, null, null, shieldedAddress = ByteArray(43)))
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = false))

        assertNotNull(m.setUnavailableReason(shielded))
        m.setMode(Mode.SET, shielded)
        assertEquals(Mode.KEEP, m.row(shielded).mode)
        m.setMode(Mode.REMOVE, shielded)
        assertEquals(DashPayPaymentAddressUpdate.Remove, m.update(shielded))
        assertNull(m.setUnavailableReason(core))
    }

    @Test
    fun shouldDisableSetEverywhereWhenTheSupportCheckFails() = runTest {
        val m = model()
        m.start(Network.TESTNET) { throw IllegalStateException("offline") }
        assertTrue(m.supportError!!.contains("offline"))
        for (kind in DashPayPaymentAddressKind.entries) assertNotNull(m.setUnavailableReason(kind))
    }

    @Test
    fun shouldReportNoWalletWhenThereIsNone() = runTest {
        val m = model()
        m.start(network = null, loadSupport = null)
        assertEquals("No wallet available for this identity.", m.networkError)
        assertNotNull(m.setUnavailableReason(core))
    }

    @Test
    fun shouldShowTheWalletsReasonAndHoldSaveUntilTheAddressIsAccepted() = runTest {
        val m = model()
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        m.setMode(Mode.SET, core)
        assertTrue("an empty Set field holds Save", m.hasIncompleteSet)

        m.setText("bad", core)
        assertEquals("'bad' is not an address for Testnet", m.row(core).error)
        assertTrue(m.hasIncompleteSet)

        m.setText("good", core)
        assertNull(m.row(core).error)
        assertFalse(m.hasIncompleteSet)
        val update = m.update(core) as DashPayPaymentAddressUpdate.Set
        assertArrayEquals(good, update.bytes)
    }

    @Test
    fun shouldFetchTheCoreWalletAddressOncePerEditor() = runTest {
        val m = model()
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        m.setMode(Mode.SET, core)
        var calls = 0
        val fetch: suspend () -> String = { calls++; "good" }
        m.useCoreWalletAddress(fetch)
        assertTrue("the reuse note shows for a filled address", m.row(core).filledFromWallet)
        m.setText("", core)
        assertFalse("a typed edit clears the reuse note", m.row(core).filledFromWallet)
        m.useCoreWalletAddress(fetch)
        assertEquals(1, calls)
        assertEquals("good", m.row(core).text)
    }

    @Test
    fun shouldHoldSaveAndKeepANewerEditWhileTheCoreLookupRuns() = runTest {
        val m = model()
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        m.setMode(Mode.SET, core)
        m.setText("good", core)
        assertFalse("a valid previous value", m.hasIncompleteSet)
        var nested = 0
        m.useCoreWalletAddress {
            assertTrue("Save is held while the lookup runs", m.hasIncompleteSet)
            m.useCoreWalletAddress { nested++; "good" }
            m.setText("pasted", core)
            "good"
        }
        assertEquals("a second tap while the lookup runs is ignored", 0, nested)
        assertEquals("the newer paste wins", "pasted", m.row(core).text)
        assertFalse(m.row(core).filledFromWallet)
        assertFalse(m.row(core).isFetchingWalletAddress)
    }

    @Test
    fun shouldDropAPreparedShieldedAddressTheUserEditedOverMeanwhile() = runTest {
        val m = model()
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        m.setMode(Mode.SET, shielded)
        m.useShieldedWalletAddress(
            prepare = {
                assertTrue("Save is held while the fill runs", m.hasIncompleteSet)
                m.setText("typed", shielded)
                ByteArray(43)
            },
            render = { _, _ -> "good" },
        )
        assertEquals("typed", m.row(shielded).text)
        assertFalse(m.row(shielded).filledFromWallet)
        assertNull(m.row(shielded).walletAddressError)
        assertFalse(m.row(shielded).isFetchingWalletAddress)
    }

    @Test
    fun shouldFillTheShieldedFieldFromThePreparedTipAddress() = runTest {
        val m = model()
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        m.setMode(Mode.SET, shielded)
        m.useShieldedWalletAddress(prepare = { ByteArray(43) }, render = { _, _ -> "good" })
        assertEquals("good", m.row(shielded).text)
        assertTrue(m.row(shielded).filledFromWallet)
        assertFalse(m.row(shielded).isFetchingWalletAddress)

        m.useShieldedWalletAddress(prepare = { throw IllegalStateException("unbound") }, render = { _, _ -> "good" })
        assertTrue(m.row(shielded).walletAddressError!!.contains("unbound"))
        assertFalse(m.row(shielded).isFetchingWalletAddress)
    }

    @Test
    fun shouldSendOnlyTheTextFieldsAnUpdateChanged() {
        val snapshot = DashPayProfile("Alice", "old message", null)
        assertNull("an untouched field keeps the published value",
            profileTextFieldUpdate("old message", snapshot, snapshot.publicMessage))
        assertEquals("Alicia", profileTextFieldUpdate(" Alicia ", snapshot, snapshot.displayName))
        assertEquals("https://x/a.png", profileTextFieldUpdate("https://x/a.png", snapshot, snapshot.avatarUrl))
        assertEquals("a create sends every filled field", "Alice", profileTextFieldUpdate("Alice", null, null))
        assertNull(profileTextFieldUpdate("  ", null, null))
    }

    @Test
    fun shouldReportAChangeOnlyForSetOrRemove() = runTest {
        val m = model(DashPayProfile(null, null, null, corePaymentAddress = good))
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        assertFalse("all Keep changes nothing", m.hasChanges)
        m.setMode(Mode.REMOVE, core)
        assertTrue(m.hasChanges)
        m.setMode(Mode.KEEP, core)
        m.setMode(Mode.SET, platform)
        assertTrue(m.hasChanges)
    }

    @Test
    fun shouldTreatSettingThePublishedAddressAsNoChange() = runTest {
        val published = ByteArray(43) { 9 }
        val m = DashPayPaymentAddressEditorModel(DashPayProfile(null, null, null, shieldedAddress = published)) { _, _, _ ->
            ByteArray(43) { 9 } // the same address, a distinct array
        }
        m.startWith(DashPayPaymentAddressSupport(core = true, platform = true, shielded = true))
        m.setMode(Mode.SET, shielded)
        m.setText("tip address", shielded)
        assertFalse("re-filling the published address changes nothing", m.hasChanges)
        assertEquals(DashPayPaymentAddressUpdate.Keep, m.update(shielded))
    }
}
