package org.dashfoundation.example

import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * DashPay tab smoke tests — port of `DashPayTabUITests.swift`, adapted for
 * the Kotlin hub's structure (nav-section rows instead of a Contacts/Requests
 * segmented control). Network-free: they assert only the local-state gating
 * the tab renders (no wallet / no identity / identity present), keyed on the
 * `dashpay.*` testTags reused verbatim from iOS. Runs against the real
 * bootstrap, so it also proves end-to-end startup + tab navigation.
 */
@RunWith(AndroidJUnit4::class)
class DashPayTabUITest {

    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    private fun anyNodeWithTag(tag: String): Boolean =
        composeRule.onAllNodes(hasTestTag(tag)).fetchSemanticsNodes().isNotEmpty()

    private fun openDashPayTab() {
        composeRule.waitUntil(timeoutMillis = 60_000) {
            anyNodeWithTag("rootTab.dashpay")
        }
        composeRule.onNodeWithTag("rootTab.dashpay").performClick()
        composeRule.waitForIdle()
    }

    /**
     * The DashPay tab must render exactly one recognized local state:
     *   1. no wallet loaded       → "Open Wallets" CTA (`dashpay.openWallets`)
     *   2. wallet, no identity    → "Open Identities" CTA (`dashpay.openIdentities`)
     *   3. ≥1 eligible identity   → the hub sections (`dashpay.openContacts`)
     * On a fresh emulator state 1 is expected, but the test accepts any of the
     * three so it stays valid with leftover local wallets — the invariant is
     * "the tab renders a recognized state".
     */
    @Test
    fun dashPayTabRendersARecognizedState() {
        openDashPayTab()

        composeRule.waitUntil(timeoutMillis = 30_000) {
            anyNodeWithTag("dashpay.openWallets") ||
                anyNodeWithTag("dashpay.openIdentities") ||
                anyNodeWithTag("dashpay.openContacts")
        }

        assertTrue(
            "DashPay tab must render one of the states: no-wallet CTA, " +
                "no-identity CTA, or the identity-present hub sections.",
            anyNodeWithTag("dashpay.openWallets") ||
                anyNodeWithTag("dashpay.openIdentities") ||
                anyNodeWithTag("dashpay.openContacts"),
        )
    }

    /**
     * When an identity is active (state 3), the hub exposes the contact
     * management + recovery entry points directly — Add Contact, Refresh,
     * Ignored and Hidden. (In iOS these were toolbar buttons / a gated link;
     * the Kotlin hub surfaces them as nav rows — the documented deviation.)
     * Skipped when no eligible identity exists on this emulator.
     */
    @Test
    fun hubExposesContactEntryPointsWhenIdentityActive() {
        openDashPayTab()

        composeRule.waitUntil(timeoutMillis = 30_000) {
            anyNodeWithTag("dashpay.openWallets") ||
                anyNodeWithTag("dashpay.openIdentities") ||
                anyNodeWithTag("dashpay.openContacts")
        }

        org.junit.Assume.assumeTrue(
            "No eligible identity on this emulator — the hub sections are " +
                "unreachable, so the entry points can't be asserted.",
            anyNodeWithTag("dashpay.openContacts"),
        )

        assertTrue("Add Contact entry must be present", anyNodeWithTag("dashpay.addContact"))
        assertTrue("Refresh must be present", anyNodeWithTag("dashpay.refresh"))
        assertTrue("Ignored recovery entry must be present", anyNodeWithTag("dashpay.openIgnored"))
        assertTrue("Hidden recovery entry must be present", anyNodeWithTag("dashpay.openHidden"))
    }
}
