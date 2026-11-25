package org.dashfoundation.example.ui.contracts

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.verticalScroll
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotDisplayed
import androidx.compose.ui.test.getUnclippedBoundsInRoot
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.wallet.PrivateDocumentEntryCreated
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner

/**
 * What the Create Entry screen leaves in view after a tap (TEST_PLAN DOC-19). The button is the
 * last thing in a scrolled form, so it is tapped at the bottom edge of the screen: the outcome of
 * the create, or why nothing was sent, appears under it and has to be brought into view. Left out
 * of sight, a create that went through looks like a tap that did nothing, with the same entry one
 * tap away from being paid for again.
 *
 * The form above the button is a spacer taller than the screen; the area under test is the
 * screen's own.
 */
@RunWith(RobolectricTestRunner::class)
class PrivateEntryCreateSubmitAreaTest {

    @get:Rule
    val compose = createComposeRule()

    private var isSubmitting by mutableStateOf(false)
    private var localRefusal by mutableStateOf<String?>(null)
    private var outcome by mutableStateOf<PrivateEntryCreateOutcome?>(null)
    private var outcomeSinceOpened by mutableStateOf(true)
    private var onSubmit: () -> Unit = {}
    private val scroll = ScrollState(0)

    /** The SDK builds this only from the bytes Rust returns; its constructor is internal. */
    private fun created(): PrivateEntryCreateOutcome.Created {
        val constructor = PrivateDocumentEntryCreated::class.java
            .getDeclaredConstructor(ByteArray::class.java, Long::class.javaPrimitiveType)
        return PrivateEntryCreateOutcome.Created(constructor.newInstance(ByteArray(32) { 7 }, 4L))
    }

    /**
     * The form as the screen lays it out: one scrolling column that ends with the area. [height]
     * makes the window a short one (landscape, split screen).
     */
    private fun showForm(height: Dp? = null) {
        compose.setContent {
            val window = height?.let { Modifier.fillMaxWidth().height(it) } ?: Modifier.fillMaxSize()
            Column(
                modifier = window.testTag("form").verticalScroll(scroll).padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Spacer(Modifier.height(1200.dp))
                PrivateEntryCreateSubmitArea(
                    isSubmitting = isSubmitting,
                    canSubmit = !isSubmitting,
                    isHeld = false,
                    localRefusal = localRefusal,
                    outcome = outcome,
                    outcomeSinceOpened = outcomeSinceOpened,
                    onSubmit = { onSubmit() },
                    onReadEntries = {},
                )
            }
        }
    }

    /** Where the sender is when tapping: at the end of the form, the button at the bottom edge. */
    private fun scrollToTheButton() {
        compose.runOnIdle { runBlocking { scroll.scrollTo(scroll.maxValue) } }
        compose.onNodeWithTag("privateEntryCreate.createButton").assertIsDisplayed()
    }

    @Test
    fun `should bring into view the outcome of a create that finishes while the form is shown`() {
        showForm()
        scrollToTheButton()

        isSubmitting = true
        compose.waitForIdle()
        isSubmitting = false
        outcome = created()

        compose.onNodeWithTag("privateEntryCreate.success").assertIsDisplayed()
        compose.onNodeWithTag("privateEntryCreate.readEntriesLink").assertIsDisplayed()
        compose.onNodeWithTag("privateEntryCreate.createButton").assertIsDisplayed()
    }

    @Test
    fun `should bring into view a create that may have gone through`() {
        showForm()
        scrollToTheButton()

        isSubmitting = true
        compose.waitForIdle()
        isSubmitting = false
        outcome = PrivateEntryCreateOutcome.Unconfirmed(PrivateEntryCreateOutcome.UNCONFIRMED_MESSAGE)

        compose.onNodeWithTag("privateEntryCreate.unconfirmed").assertIsDisplayed()
    }

    @Test
    fun `should bring into view why a tap sent nothing, each time it is tapped`() {
        onSubmit = { localRefusal = "The recipient changed: load its keys again." }
        showForm()
        scrollToTheButton()
        val buttonAtTheEdge = compose.runOnIdle { scroll.value }

        compose.onNodeWithTag("privateEntryCreate.createButton").performClick()
        compose.onNodeWithTag("privateEntryCreate.refusal").assertIsDisplayed()

        // Scrolled back so the button is at the bottom edge again, and tapped for the same answer
        compose.runOnIdle { runBlocking { scroll.scrollTo(buttonAtTheEdge) } }
        compose.onNodeWithTag("privateEntryCreate.refusal").assertIsNotDisplayed()
        compose.onNodeWithTag("privateEntryCreate.createButton").performClick()

        compose.onNodeWithTag("privateEntryCreate.refusal").assertIsDisplayed()
    }

    @Test
    fun `should leave a reopened form at its top when what it shows is an earlier create's outcome`() {
        outcomeSinceOpened = false
        showForm()
        compose.waitForIdle()

        // The wallet is picked a moment after the screen opens, and with it its last outcome
        outcome = created()
        compose.waitForIdle()

        assertEquals(0, scroll.value)
        compose.onNodeWithTag("privateEntryCreate.success").assertIsNotDisplayed()
    }

    @Test
    fun `should not scroll again to an answer it already showed when another wallet is chosen and this one again`() {
        onSubmit = { localRefusal = "Could not encode the properties: not a number" }
        showForm()
        scrollToTheButton()
        val finished = created()
        outcome = finished
        compose.onNodeWithTag("privateEntryCreate.success").assertIsDisplayed()
        compose.onNodeWithTag("privateEntryCreate.createButton").performClick()
        compose.onNodeWithTag("privateEntryCreate.refusal").assertIsDisplayed()

        // Back at the top of the form, at the wallet picker: the other wallet has no outcome here
        compose.runOnIdle { runBlocking { scroll.scrollTo(0) } }
        outcome = null
        compose.waitForIdle()
        outcome = finished
        compose.waitForIdle()

        assertEquals(0, scroll.value)
    }

    @Test
    fun `should show the whole outcome in a window too short for the button and the outcome together`() {
        showForm(height = 230.dp)
        scrollToTheButton()

        outcome = created()
        compose.waitForIdle()

        val form = compose.onNodeWithTag("form").getUnclippedBoundsInRoot()
        val button = compose.onNodeWithTag("privateEntryCreate.createButton").getUnclippedBoundsInRoot()
        val entryCreated = compose.onNodeWithTag("privateEntryCreate.success").getUnclippedBoundsInRoot()
        val readEntries = compose.onNodeWithTag("privateEntryCreate.readEntriesLink").getUnclippedBoundsInRoot()
        // The case under test: the outcome fits the window, the button with it does not
        assertTrue(readEntries.bottom - entryCreated.top < form.bottom - form.top)
        assertTrue(readEntries.bottom - button.top > form.bottom - form.top)
        assertTrue("Entry created is cut off at the top", entryCreated.top >= form.top)
        assertTrue("Read Entries is cut off at the bottom", readEntries.bottom <= form.bottom)
    }
}
