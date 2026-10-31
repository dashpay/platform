package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.DecisionProposalsNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.dashfoundation.dashsdk.persistence.toBase58String
import org.dashfoundation.dashsdk.voting.YesNoAbstainVoteChoice
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Round trips of the decision proposal JNI exports through `rs-sdk-ffi` on a device: the
 * symbol names, the argument marshalling and the error codes Kotlin maps.
 * Every call here is refused before anything reaches the network (rs-sdk-ffi reads every
 * argument first), so no network or protocol version 15 is needed: `createTrusted` only
 * builds the client. The flows that reach Platform (build, fetch, vote) need a network at
 * protocol version 15 and are exercised through KotlinExampleApp.
 */
@RunWith(AndroidJUnit4::class)
class DecisionProposalsFfiTest {

    /** A well-formed 32-byte id that names no proposal. */
    private val proposalId = ByteArray(32) { 0x11 }.toBase58String()

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

    @Test
    fun shouldReadTheContractIdInBothEncodingsAndTheTypeName() {
        val bytes = DecisionProposalsNative.contractIdBytes()
        assertEquals(32, bytes.size)
        assertEquals(bytes.toBase58String(), DecisionProposalsNative.contractId())
        assertTrue(DecisionProposalsNative.documentTypeName().isNotEmpty())
    }

    @Test
    fun shouldFreeANullDocumentHandleAsANoOp() {
        DecisionProposalsNative.documentFree(0L)
    }

    /** rs-sdk-ffi refuses null document handles and bad ids with InvalidParameter (1). */
    @Test
    fun shouldRefuseBadArgumentsBeforeTheNetwork() {
        assertCode(1) { DecisionProposalsNative.propertiesJson(0L) }
        assertCode(1) {
            DecisionProposalsNative.buildEdit(
                0L,
                DecisionProposalsNative.contractId(),
                DecisionProposalsNative.documentTypeName(),
                "new title",
                null,
                null,
                false,
            )
        }
        assertCode(1) { DecisionProposalsNative.fetch(handle, "0OIl") }
        assertCode(1) { DecisionProposalsNative.fetchDocument(handle, "0OIl") }
        assertCode(1) { DecisionProposalsNative.getVotePollId(handle, "0OIl") }
        assertCode(1) { DecisionProposalsNative.getVotePollState(handle, "0OIl") }
        assertCode(1) { DecisionProposalsNative.listByOwner(handle, "0OIl", 0, null) }
        assertCode(1) { DecisionProposalsNative.listByCreatedAt(handle, 0, "0OIl") }
        assertCode(1) {
            DecisionProposalsNative.build(handle, "0OIl", "title", "description", null, 60, 10, 1L)
        }
    }

    /** The JNI layer refuses a value the C type cannot hold instead of wrapping it. */
    @Test
    fun shouldRefuseNumbersOutsideTheCTypes() {
        assertCode(1) {
            DecisionProposalsNative.build(handle, proposalId, "title", "description", null, 300, 10, 1L)
        }
        assertCode(1) {
            DecisionProposalsNative.build(handle, proposalId, "title", "description", null, 60, -1, 1L)
        }
        assertCode(1) {
            DecisionProposalsNative.build(handle, proposalId, "title", "description", null, 60, 10, -1L)
        }
        assertCode(1) { DecisionProposalsNative.listByEndDate(handle, 0L, 0L, -1, null) }
        assertCode(1) { DecisionProposalsNative.listByEndDate(handle, -1L, 0L, 0, null) }
    }

    /** The C functions read exactly 32 bytes behind these pointers. */
    @Test
    fun shouldRefuseAProTxHashOrVotingKeyThatIsNot32Bytes() {
        assertCode(1) { DecisionProposalsNative.getMasternodeVote(handle, ByteArray(31), proposalId) }
        assertCode(1) {
            DecisionProposalsNative.castVote(handle, proposalId, 0, ByteArray(31), ByteArray(32), 1)
        }
        assertCode(1) {
            DecisionProposalsNative.castVote(handle, proposalId, 0, ByteArray(32), ByteArray(33), 1)
        }
    }

    /**
     * rs-sdk-ffi reads the vote choice before the voting key, so every Kotlin choice gets
     * past the choice and is refused on the all-zero key instead, while a byte outside the
     * three is refused as a choice. Nothing reaches the network either way.
     */
    @Test
    fun shouldAcceptExactlyTheThreeVoteChoices() {
        for (choice in YesNoAbstainVoteChoice.entries) {
            val error = assertCode(1) {
                DecisionProposalsNative.castVote(
                    handle, proposalId, choice.discriminant, ByteArray(32) { 1 }, ByteArray(32), 1,
                )
            }
            assertFalse(error.message, error.message.orEmpty().contains("vote_choice"))
        }
        val refused = assertCode(1) {
            DecisionProposalsNative.castVote(handle, proposalId, 3, ByteArray(32) { 1 }, ByteArray(32), 1)
        }
        assertTrue(refused.message, refused.message.orEmpty().contains("vote_choice"))
        assertCode(1) {
            DecisionProposalsNative.castVote(handle, proposalId, 256, ByteArray(32) { 1 }, ByteArray(32), 1)
        }
    }

    private fun assertCode(code: Int, call: () -> Unit): DashSDKException {
        val error = assertThrows(DashSDKException::class.java) { call() }
        assertEquals(error.message, code, error.code)
        return error
    }
}
