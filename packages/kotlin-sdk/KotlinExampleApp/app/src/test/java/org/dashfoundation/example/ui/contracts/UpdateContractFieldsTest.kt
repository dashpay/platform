package org.dashfoundation.example.ui.contracts

import org.dashfoundation.dashsdk.identity.DataContractUpdatePreview
import org.dashfoundation.dashsdk.identity.DataContractUpdateSource
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The text conversions and the paid no-op guard of the reviewed Update Contract screen: the
 * Kotlin side of Swift's `UpdateContractImportTests`. The review's content is Rust's; these
 * pin only how the screen turns its fields into wallet arguments and when it offers the
 * broadcast.
 */
class UpdateContractFieldsTest {

    private fun preview(hasChanges: Boolean = true, configChanged: Boolean = false) =
        DataContractUpdatePreview(
            currentVersion = 3,
            newVersion = 4,
            hasChanges = hasChanges,
            documentTypes = DataContractUpdatePreview.SectionChanges(emptyList(), emptyList()),
            tokens = DataContractUpdatePreview.SectionChanges(emptyList(), emptyList()),
            groups = DataContractUpdatePreview.SectionChanges(emptyList(), emptyList()),
            keywordsChanged = false,
            descriptionChanged = false,
            configChanged = configChanged,
            configChangedFields = emptyList(),
            moderatorsAdded = emptyList(),
            moderatorsRemoved = emptyList(),
        )

    private val target = UpdateContractTarget(contractIdHex = "aa".repeat(32), ownerIdHex = "bb".repeat(32))

    private fun review(fields: UpdateContractFields, preview: DataContractUpdatePreview) =
        UpdateContractViewModel.Review(fields, target, fields.walletArguments(), preview)

    @Test
    fun shouldTurnEmptyFieldsIntoKeepTheLiveValues() {
        val arguments = UpdateContractFields(documents = "  ").walletArguments()
        assertEquals("{}", arguments.documents)
        assertNull(arguments.tokens)
        assertNull(arguments.groups)
        assertNull(arguments.keywords)
        assertNull(arguments.description)
        assertNull(arguments.config)
    }

    @Test
    fun shouldSendTheDescriptionAsTypedAndTrimJsonSections() {
        val arguments = UpdateContractFields(
            documents = " {\"a\":{}} ",
            tokens = "\n{\"0\":{}}\n",
            description = "  spaced  ",
            config = " {\"readonly\":false} ",
        ).walletArguments()
        assertEquals("{\"a\":{}}", arguments.documents)
        assertEquals("{\"0\":{}}", arguments.tokens)
        assertEquals("  spaced  ", arguments.description)
        assertEquals("{\"readonly\":false}", arguments.config)
    }

    @Test
    fun shouldReadKeywordsCommaSeparatedOrAsAJsonArray() {
        assertEquals("[\"a\",\"b c\"]", UpdateContractFields.keywordsJson(" a , b c ,"))
        assertNull(UpdateContractFields.keywordsJson(" , ,"))
        assertEquals("  [\"alpha,beta\"]", UpdateContractFields.keywordsJson("  [\"alpha,beta\"]"))
        assertEquals("[\"say \\\"hi\\\"\"]", UpdateContractFields.keywordsJson("say \"hi\""))
    }

    @Test
    fun shouldFillEveryFieldFromASplitSourceAndResetTheOthers() {
        val fields = UpdateContractFields.from(
            DataContractUpdateSource(
                documents = "{\"note\":{}}",
                tokens = null,
                groups = null,
                keywords = "[\"k\"]",
                description = null,
                config = null,
                carriesSharedDefinitions = false,
            ),
        )
        assertEquals(UpdateContractFields(documents = "{\"note\":{}}", keywords = "[\"k\"]"), fields)
    }

    /** The update is offered only for a review of the fields on screen that reports a change. */
    @Test
    fun shouldOfferTheUpdateOnlyForAReviewOfTheFieldsShownThatChangesSomething() {
        val fields = UpdateContractFields(documents = "{\"a\":{}}")
        assertFalse(canSubmitUpdate(null, fields, target, false, false, false))
        assertTrue(canSubmitUpdate(review(fields, preview()), fields, target, false, false, false))
        // An edit after the review
        assertFalse(canSubmitUpdate(review(fields, preview()), fields.copy(tokens = "{}"), target, false, false, false))
        // Rust says nothing changes
        assertFalse(canSubmitUpdate(review(fields, preview(hasChanges = false)), fields, target, false, false, false))
    }

    /** A review stands only for the contract and owner it was made for. */
    @Test
    fun shouldNotOfferTheUpdateOnceTheContractOrOwnerChanged() {
        val fields = UpdateContractFields(documents = "{\"a\":{}}")
        val reviewed = review(fields, preview())
        assertTrue(canSubmitUpdate(reviewed, fields, target, false, false, false))
        assertFalse(canSubmitUpdate(reviewed, fields, target.copy(contractIdHex = "cc".repeat(32)), false, false, false))
        assertFalse(canSubmitUpdate(reviewed, fields, target.copy(ownerIdHex = "dd".repeat(32)), false, false, false))
        assertFalse(canSubmitUpdate(reviewed, fields, null, false, false, false))
    }

    @Test
    fun shouldAskForTheConfigurationAcknowledgmentAndHoldAfterAnUnconfirmedUpdate() {
        val fields = UpdateContractFields(config = "{\"readonly\":true}")
        val configReview = review(fields, preview(configChanged = true))
        assertFalse(canSubmitUpdate(configReview, fields, target, false, false, false))
        assertTrue(canSubmitUpdate(configReview, fields, target, true, false, false))
        assertFalse(canSubmitUpdate(configReview, fields, target, true, true, false))
        assertFalse(canSubmitUpdate(configReview, fields, target, true, false, true))
    }
}
