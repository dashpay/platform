package org.dashfoundation.example

import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.test.ext.junit.runners.AndroidJUnit4
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * App-shell smoke test — the plan's B-M0 Compose gate: the five tabs render
 * and switch (testTags reuse the iOS accessibility identifiers, e.g.
 * `rootTab.wallets`). Runs against the real bootstrap (native library +
 * testnet SDK build), so it also proves end-to-end app startup on-device.
 */
@RunWith(AndroidJUnit4::class)
class AppSmokeTest {

    @get:Rule
    val composeRule = createAndroidComposeRule<MainActivity>()

    @Test
    fun tabsRenderAndSwitch() {
        // Bootstrap gate → main scaffold. SDK creation reaches out to
        // testnet config only (no blocking network round-trip), but allow
        // a generous window for the native init on cold emulators.
        composeRule.waitUntil(timeoutMillis = 60_000) {
            composeRule.onAllNodes(androidx.compose.ui.test.hasTestTag("rootTab.wallets"))
                .fetchSemanticsNodes().isNotEmpty()
        }

        listOf(
            "rootTab.wallets",
            "rootTab.identities",
            "rootTab.dashpay",
            "rootTab.settings",
            "rootTab.sync",
        ).forEach { tag ->
            composeRule.onNodeWithTag(tag).assertIsDisplayed().performClick()
            composeRule.waitForIdle()
        }
    }
}
