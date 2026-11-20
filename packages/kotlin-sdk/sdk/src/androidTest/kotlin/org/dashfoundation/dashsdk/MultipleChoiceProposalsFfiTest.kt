package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.MultipleChoiceProposalsNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.voting.MultipleChoiceVoteChoice
import org.dashfoundation.dashsdk.voting.toNative
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Round trips of the multiple choice proposal JNI exports through `rs-sdk-ffi` and
 * `platform-wallet-ffi` on a device: the symbol names, the argument marshalling and the
 * error codes Kotlin maps. Every call here is refused before anything reaches the network
 * (the JNI layer and the C functions read every argument first), so no network, wallet or
 * protocol version 15 is needed: `createTrusted` only builds the client. The flows that
 * reach Platform (create, edit, enter a choice, the reads, the two amounts, vote) need a
 * network at protocol version 15 and are exercised through KotlinExampleApp.
 */
@RunWith(AndroidJUnit4::class)
class MultipleChoiceProposalsFfiTest {

    /** A well-formed 32-byte id that names no proposal. */
    private val proposalIdBytes = ByteArray(32) { 0x11 }
    private val proposalId = proposalIdBytes.toBase58String()

    /** A well-formed 32-byte identity id. */
    private val ownerId = ByteArray(32) { 0x22 }

    /** A well-formed id that names no choice, base58. */
    private val choiceId = ByteArray(32) { 0x33 }.toBase58String()

    private var handle = 0L

    @Before
    fun createSdk() {
        NativeLoader.ensureLoaded()
        handle = SdkNative.createTrusted(
            network = 1,
            dapiAddresses = null,
            quorumUrl = null,
            skipAssetLockProofVerification = false,
            requestRetryCount = 3,
            requestTimeoutMs = 30_000,
            platformVersion = 0,
        )
        assertNotEquals("createTrusted must return a live handle", 0L, handle)
    }

    @After
    fun destroySdk() {
        SdkNative.destroy(handle)
    }

    /** rs-sdk-ffi refuses ids that are not base58 with InvalidParameter (1). */
    @Test
    fun shouldRefuseBadArgumentsBeforeTheNetwork() {
        assertCode(1) { MultipleChoiceProposalsNative.fetch(handle, "0OIl") }
        assertCode(1) { MultipleChoiceProposalsNative.getVote(handle, "0OIl") }
        assertCode(1) { MultipleChoiceProposalsNative.getKeptChoice(handle, "0OIl") }
        assertCode(1) { MultipleChoiceProposalsNative.getChoiceFundToJoin(handle, "0OIl") }
        assertCode(1) { MultipleChoiceProposalsNative.listByOwner(handle, "0OIl", 0, null) }
        assertCode(1) { MultipleChoiceProposalsNative.listByOwner(handle, proposalId, 0, "0OIl") }
        assertCode(1) { MultipleChoiceProposalsNative.listByCreatedAt(handle, 0, "0OIl") }
        assertCode(1) {
            MultipleChoiceProposalsNative.getMasternodeVote(handle, ByteArray(32) { 1 }, "0OIl")
        }
    }

    /** The JNI layer refuses a value the C type cannot hold instead of wrapping it. */
    @Test
    fun shouldRefuseNumbersOutsideTheCTypes() {
        assertCode(1) { MultipleChoiceProposalsNative.listByOwner(handle, proposalId, -1, null) }
        assertCode(1) { MultipleChoiceProposalsNative.listByCreatedAt(handle, -1, null) }
    }

    /**
     * The wallet writes read exactly 32 bytes behind each id, and the two that create a
     * document check the array its id is written back to before anything is broadcast.
     */
    @Test
    fun shouldRefuseAWalletWriteIdThatIsNot32Bytes() {
        assertCode(1) { create(outProposalId = ByteArray(31)) }
        assertCode(1) { create(ownerId = ByteArray(33)) }
        assertCode(1) { create(descriptionDocumentId = ByteArray(31)) }
        assertCode(1) { create(descriptionDocumentContractId = ByteArray(33)) }
        assertCode(1) { edit(ownerId = ByteArray(31)) }
        assertCode(1) { edit(proposalId = ByteArray(33)) }
        assertCode(1) { edit(descriptionDocumentId = ByteArray(33)) }
        assertCode(1) { enterChoice(outChoiceId = ByteArray(31)) }
        assertCode(1) { enterChoice(ownerId = ByteArray(33)) }
        assertCode(1) { enterChoice(proposalId = ByteArray(31)) }
    }

    /**
     * The three wallet writes reach `platform-wallet-ffi`, which refuses a null signer
     * (`ErrorNullPointer`, 3) before it looks the wallet up; the code arrives shifted by the
     * platform wallet offset, and no id is written back.
     */
    @Test
    fun shouldReachThePlatformWalletForTheWrites() {
        val nullPointer = DashSdkError.PLATFORM_WALLET_CODE_OFFSET + 3
        val outProposalId = ByteArray(32)
        assertCode(nullPointer) { create(outProposalId = outProposalId) }
        assertArrayEquals(ByteArray(32), outProposalId)
        // A description document given in part still crosses: the wallet is the one to refuse it.
        assertCode(nullPointer) { create(descriptionDocumentId = null, descriptionDocumentType = null) }
        assertCode(nullPointer) { edit() }
        assertCode(nullPointer) { edit(url = "https://example.com", removeUrl = true) }
        assertCode(nullPointer) { edit(removeDescriptionDocument = true) }
        val outChoiceId = ByteArray(32)
        assertCode(nullPointer) { enterChoice(outChoiceId = outChoiceId) }
        assertArrayEquals(ByteArray(32), outChoiceId)
    }

    /** The C functions read exactly 32 bytes behind these pointers. */
    @Test
    fun shouldRefuseAProTxHashOrVotingKeyThatIsNot32Bytes() {
        assertCode(1) {
            MultipleChoiceProposalsNative.getMasternodeVote(handle, ByteArray(31), proposalId)
        }
        assertCode(1) {
            MultipleChoiceProposalsNative.castVote(
                handle, proposalId, 1, null, ByteArray(31), ByteArray(32), 1,
            )
        }
        assertCode(1) {
            MultipleChoiceProposalsNative.castVote(
                handle, proposalId, 1, null, ByteArray(32), ByteArray(33), 1,
            )
        }
    }

    /**
     * rs-sdk-ffi reads the vote choice and the choice id before the voting key, so every
     * Kotlin choice gets past them and is refused on the all-zero key instead, while a byte
     * outside the three is refused as a choice. Nothing reaches the network either way.
     */
    @Test
    fun shouldAcceptExactlyTheThreeVoteChoices() {
        val choices = listOf(
            MultipleChoiceVoteChoice.Choice(choiceId),
            MultipleChoiceVoteChoice.NoneOfTheChoices,
            MultipleChoiceVoteChoice.Abstain,
        )
        for (choice in choices) {
            val vote = choice.toNative()
            val error = assertCode(1) { castVote(vote.discriminant, vote.choiceId) }
            assertFalse(error.message, error.message.orEmpty().contains("vote_choice"))
            assertFalse(error.message, error.message.orEmpty().contains("choice_id"))
        }
        val refused = assertCode(1) { castVote(3, null) }
        assertTrue(refused.message, refused.message.orEmpty().contains("vote_choice"))
        assertCode(1) { castVote(256, null) }
    }

    /** A vote for a choice names it, and the other answers name none. */
    @Test
    fun shouldRefuseAChoiceIdThatDoesNotFitTheVoteChoice() {
        val missing = assertCode(1) { castVote(0, null) }
        assertTrue(missing.message, missing.message.orEmpty().contains("choice_id"))
        for (voteChoice in listOf(1, 2)) {
            val superfluous = assertCode(1) { castVote(voteChoice, choiceId) }
            assertTrue(superfluous.message, superfluous.message.orEmpty().contains("choice_id"))
        }
        assertCode(1) { castVote(0, "0OIl") }
    }

    /** `castVote` for an all-ones pro_tx_hash with an all-zero voting key. */
    private fun castVote(voteChoice: Int, choiceId: String?) {
        MultipleChoiceProposalsNative.castVote(
            handle, proposalId, voteChoice, choiceId, ByteArray(32) { 1 }, ByteArray(32), 1,
        )
    }

    /** `create` on no wallet and no signer, with valid arguments unless overridden. */
    private fun create(
        ownerId: ByteArray = this.ownerId,
        descriptionDocumentId: ByteArray? = ByteArray(32) { 0x44 },
        descriptionDocumentContractId: ByteArray? = ByteArray(32) { 0x55 },
        descriptionDocumentType: String? = "post",
        outProposalId: ByteArray = ByteArray(32),
    ) {
        MultipleChoiceProposalsNative.create(
            0L,
            ownerId,
            "title",
            descriptionDocumentId,
            descriptionDocumentContractId,
            descriptionDocumentType,
            null,
            true,
            0L,
            outProposalId,
        )
    }

    /** `edit` on no wallet and no signer, with valid arguments unless overridden. */
    private fun edit(
        ownerId: ByteArray = this.ownerId,
        proposalId: ByteArray = proposalIdBytes,
        descriptionDocumentId: ByteArray? = null,
        removeDescriptionDocument: Boolean = false,
        url: String? = null,
        removeUrl: Boolean = false,
    ) {
        MultipleChoiceProposalsNative.edit(
            0L,
            ownerId,
            proposalId,
            "new title",
            descriptionDocumentId,
            null,
            null,
            removeDescriptionDocument,
            url,
            removeUrl,
            0L,
        )
    }

    /** `enterChoice` on no wallet and no signer, with valid arguments unless overridden. */
    private fun enterChoice(
        ownerId: ByteArray = this.ownerId,
        proposalId: ByteArray = proposalIdBytes,
        outChoiceId: ByteArray = ByteArray(32),
    ) {
        MultipleChoiceProposalsNative.enterChoice(
            0L, ownerId, proposalId, "title", null, 0L, outChoiceId,
        )
    }

    private fun assertCode(code: Int, call: () -> Unit): DashSDKException {
        val error = assertThrows(DashSDKException::class.java) { call() }
        assertEquals(error.message, code, error.code)
        return error
    }
}
