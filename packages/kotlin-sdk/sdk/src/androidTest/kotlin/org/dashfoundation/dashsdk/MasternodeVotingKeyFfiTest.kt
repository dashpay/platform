package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.DecisionProposalsNative
import org.dashfoundation.dashsdk.ffi.MasternodeVotingKeyNative
import org.dashfoundation.dashsdk.ffi.MultipleChoiceProposalsNative
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.SdkNative
import org.dashfoundation.dashsdk.ffi.TransactionsNative
import org.dashfoundation.dashsdk.persistence.toBase58String
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
 * Round trips of the JNI exports that vote with a stored masternode voting key: the two key
 * helpers (`MasternodeVotingKeyNative`) and the three `_with_signer` votes. The helpers run
 * fully on device against published vectors; the votes are refused before anything reaches
 * the network (a voting address of the wrong length in the JNI layer, a null signer in
 * rs-sdk-ffi), which proves the symbols bind and the arguments arrive in order.
 */
@RunWith(AndroidJUnit4::class)
class MasternodeVotingKeyFfiTest {

    /**
     * The secret 1 and its vectors, pinned against rs-sdk-ffi by
     * `masternode_voting_key::tests` in rs-unified-sdk-jni.
     */
    private val keyOneHex = "0000000000000000000000000000000000000000000000000000000000000001"
    private val keyOneVotingAddressHex = "751e76e8199196d454941c45d1b3a323f1433bd6"
    private val keyOneTestnetWif = "cMahea7zqjxrtgAbB7LSGbcQUr1uX1ojuat9jZodMN87JcbXMTcA"
    private val keyOne = ByteArray(32).also { it[31] = 1 }

    private val proposalId = ByteArray(32) { 0x11 }.toBase58String()
    private val proTxHash = ByteArray(32) { 1 }
    private val votingAddress = ByteArray(20) { 2 }

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
    fun shouldFormTheVotingAddressOfAPrivateKey() {
        assertArrayEquals(
            hex(keyOneVotingAddressHex),
            MasternodeVotingKeyNative.votingAddressFromPrivateKey(keyOne),
        )
        assertCode(1) { MasternodeVotingKeyNative.votingAddressFromPrivateKey(ByteArray(31)) }
        // Zero is not a secp256k1 secret key.
        assertCode(1) { MasternodeVotingKeyNative.votingAddressFromPrivateKey(ByteArray(32)) }
    }

    @Test
    fun shouldReadAPrivateKeyTypedAsHexOrWif() {
        assertArrayEquals(
            keyOne,
            MasternodeVotingKeyNative.privateKeyBytesFromText(keyOneHex.toByteArray()),
        )
        assertArrayEquals(
            keyOne,
            MasternodeVotingKeyNative.privateKeyBytesFromText(keyOneTestnetWif.toByteArray()),
        )
        assertArrayEquals(
            keyOne,
            MasternodeVotingKeyNative.privateKeyBytesFromText("  $keyOneHex\n".toByteArray()),
        )
    }

    /** A refusal carries Rust's reason, never the text. */
    @Test
    fun shouldRefuseTextThatIsNotAKeyWithoutEchoingIt() {
        val typed = "notakey" + keyOneHex.drop(10)
        val error = assertCode(1) {
            MasternodeVotingKeyNative.privateKeyBytesFromText(typed.toByteArray())
        }
        assertFalse(error.message, error.message.orEmpty().contains(typed))
        val nul = assertCode(1) {
            MasternodeVotingKeyNative.privateKeyBytesFromText((keyOneHex + "\u0000").toByteArray())
        }
        assertTrue(nul.message, nul.message.orEmpty().contains("NUL"))
    }

    /** Each vote reads exactly 20 bytes behind the voting address. */
    @Test
    fun shouldRefuseAVotingAddressThatIsNot20Bytes() {
        for (address in listOf(ByteArray(19), ByteArray(21), ByteArray(32))) {
            assertCode(1) { contestedVote(address, signerHandle = 1L) }
            assertCode(1) {
                DecisionProposalsNative.castVoteWithSigner(handle, proposalId, 0, proTxHash, address, 1L)
            }
            assertCode(1) {
                MultipleChoiceProposalsNative.castVoteWithSigner(
                    handle, proposalId, 1, null, proTxHash, address, 1L,
                )
            }
        }
    }

    /**
     * rs-sdk-ffi reads the poll and the choice, then the masternode, and refuses a null
     * signer there, before anything reaches the network.
     */
    @Test
    fun shouldReachRsSdkFfiWhichRefusesANullSigner() {
        for (refused in listOf(
            assertCode(1) { contestedVote(votingAddress, signerHandle = 0L) },
            assertCode(1) {
                DecisionProposalsNative.castVoteWithSigner(
                    handle, proposalId, 0, proTxHash, votingAddress, 0L,
                )
            },
            assertCode(1) {
                MultipleChoiceProposalsNative.castVoteWithSigner(
                    handle, proposalId, 1, null, proTxHash, votingAddress, 0L,
                )
            },
        )) {
            assertTrue(refused.message, refused.message.orEmpty().contains("signer_handle"))
        }
    }

    /** A Lock vote on the DPNS poll for "dash/alice"; no contender id is needed. */
    private fun contestedVote(address: ByteArray, signerHandle: Long) {
        TransactionsNative.castContestedResourceVoteWithSigner(
            handle,
            "GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",
            "domain",
            "parentNameAndLabel",
            "[\"dash\",\"alice\"]",
            2,
            null,
            proTxHash,
            address,
            signerHandle,
        )
    }

    private fun hex(value: String): ByteArray =
        ByteArray(value.length / 2) { value.substring(it * 2, it * 2 + 2).toInt(16).toByte() }

    private fun assertCode(code: Int, call: () -> Unit): DashSDKException {
        val error = assertThrows(DashSDKException::class.java) { call() }
        assertEquals(error.message, code, error.code)
        return error
    }
}
