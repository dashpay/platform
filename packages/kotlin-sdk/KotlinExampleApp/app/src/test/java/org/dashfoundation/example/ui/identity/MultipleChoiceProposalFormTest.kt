package org.dashfoundation.example.ui.identity

import org.dashfoundation.dashsdk.voting.DescriptionDocumentParams
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposal
import org.dashfoundation.dashsdk.voting.MultipleChoiceProposalEdit
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * How the multiple choice proposal forms turn their text fields into what the wallet calls
 * take: the description document's ids as bytes, and an edit that sends only what the user
 * changed. The forms judge nothing else; Platform refuses what it does not accept.
 */
class MultipleChoiceProposalFormTest {

    private val describedIdBytes = ByteArray(32) { 5 }
    private val describedContractIdBytes = ByteArray(32) { 6 }
    private val describedId = Base58.encode(describedIdBytes)
    private val describedContractId = Base58.encode(describedContractIdBytes)

    private val proposal = MultipleChoiceProposal(
        id = Base58.encode(ByteArray(32) { 1 }),
        ownerId = Base58.encode(ByteArray(32) { 2 }),
        revision = 1,
        createdAtMs = 1_700_000_000_000,
        updatedAtMs = 1_700_000_000_000,
        title = "Which block size?",
        descriptionDocument = MultipleChoiceProposal.DescriptionDocument(describedId, describedContractId, "post"),
        url = "https://forum.dash.org/t/block-size",
        open = true,
    )

    /** The edit the form makes when its fields hold what is given, the rest untouched. */
    private fun edit(
        proposal: MultipleChoiceProposal = this.proposal,
        title: String = proposal.title,
        url: String = proposal.url.orEmpty(),
        describedId: String = proposal.descriptionDocument?.id.orEmpty(),
        describedContractId: String = proposal.descriptionDocument?.contractId.orEmpty(),
        describedType: String = proposal.descriptionDocument?.documentType.orEmpty(),
    ): BuiltMultipleChoiceEdit =
        multipleChoiceProposalEdit(proposal, title, url, describedId, describedContractId, describedType)

    private fun changes(edit: MultipleChoiceProposalEdit) = BuiltMultipleChoiceEdit.Edit(edit)

    @Test
    fun shouldReadThreeEmptyFieldsAsNoDescriptionDocument() {
        assertEquals(ParsedDescriptionDocument.None, parseDescriptionDocument("", " ", ""))
    }

    @Test
    fun shouldReadTheDescriptionDocumentIdsAsTheirBytes() {
        assertEquals(
            ParsedDescriptionDocument.Document(
                DescriptionDocumentParams(describedIdBytes, describedContractIdBytes, "post"),
            ),
            parseDescriptionDocument(" $describedId ", describedContractId, " post "),
        )
    }

    @Test
    fun shouldReportAnIdFieldThatDoesNotHold32Bytes() {
        val noId = parseDescriptionDocument("", describedContractId, "post")
        assertTrue("$noId", noId is ParsedDescriptionDocument.Unreadable)
        val shortContractId = parseDescriptionDocument(describedId, "abc", "post")
        assertTrue("$shortContractId", shortContractId is ParsedDescriptionDocument.Unreadable)
    }

    /** A document type left empty still crosses: the wallet is the one to refuse it. */
    @Test
    fun shouldLeaveAnEmptyDocumentTypeToTheWallet() {
        assertEquals(
            ParsedDescriptionDocument.Document(
                DescriptionDocumentParams(describedIdBytes, describedContractIdBytes, ""),
            ),
            parseDescriptionDocument(describedId, describedContractId, ""),
        )
    }

    @Test
    fun shouldSendNothingForAnUntouchedForm() {
        assertEquals(changes(MultipleChoiceProposalEdit()), edit())
    }

    @Test
    fun shouldSendOnlyTheFieldTheUserChanged() {
        assertEquals(
            changes(MultipleChoiceProposalEdit(title = "Which block size next year?")),
            edit(title = "Which block size next year?"),
        )
        assertEquals(
            changes(
                MultipleChoiceProposalEdit(
                    url = MultipleChoiceProposalEdit.Change.Set("https://example.org/sizes"),
                ),
            ),
            edit(url = " https://example.org/sizes "),
        )
        val otherId = Base58.encode(ByteArray(32) { 7 })
        assertEquals(
            changes(
                MultipleChoiceProposalEdit(
                    descriptionDocument = MultipleChoiceProposalEdit.Change.Set(
                        DescriptionDocumentParams(ByteArray(32) { 7 }, describedContractIdBytes, "post"),
                    ),
                ),
            ),
            edit(describedId = otherId),
        )
    }

    @Test
    fun shouldRemoveAnEmptiedUrlOrDescriptionDocument() {
        assertEquals(
            changes(MultipleChoiceProposalEdit(url = MultipleChoiceProposalEdit.Change.Remove)),
            edit(url = ""),
        )
        assertEquals(
            changes(
                MultipleChoiceProposalEdit(
                    descriptionDocument = MultipleChoiceProposalEdit.Change.Remove,
                ),
            ),
            edit(describedId = "", describedContractId = "", describedType = ""),
        )
    }

    /** Fields that were empty and stay empty are untouched, not a removal. */
    @Test
    fun shouldKeepWhatAProposalDoesNotHave() {
        val bare = proposal.copy(descriptionDocument = null, url = null)
        assertEquals(changes(MultipleChoiceProposalEdit()), edit(proposal = bare))
        assertEquals(
            changes(
                MultipleChoiceProposalEdit(
                    descriptionDocument = MultipleChoiceProposalEdit.Change.Set(
                        DescriptionDocumentParams(describedIdBytes, describedContractIdBytes, "post"),
                    ),
                ),
            ),
            edit(
                proposal = bare,
                describedId = describedId,
                describedContractId = describedContractId,
                describedType = "post",
            ),
        )
    }

    @Test
    fun shouldReportAnEditedDescriptionDocumentIdThatDoesNotHold32Bytes() {
        val built = edit(describedId = "abc")
        assertTrue("$built", built is BuiltMultipleChoiceEdit.Unreadable)
    }

    @Test
    fun shouldSayWhoEntersChoices() {
        assertEquals("anyone enters choices", proposal.whoEntersChoices())
        assertEquals("only its owner enters choices", proposal.copy(open = false).whoEntersChoices())
    }
}
