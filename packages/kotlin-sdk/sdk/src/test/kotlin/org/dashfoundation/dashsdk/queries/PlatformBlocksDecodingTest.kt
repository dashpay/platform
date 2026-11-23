package org.dashfoundation.dashsdk.queries

import org.dashfoundation.dashsdk.errors.DashSdkError
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Decodes the JSON the block and transition status reads in rs-sdk-ffi write
 * (`system/queries/blocks.rs` and `transition_status.rs`, whose tests pin the same shapes)
 * into the [PlatformBlocks] answer types. The Kotlin side of `PlatformBlocksDecodingTests.swift`.
 */
class PlatformBlocksDecodingTest {

    private val transactionJson =
        """{"hash":"${"07".repeat(32)}","code":40100,"executed":false,"gas_used":1200,""" +
            """"result_data":"abcd","type":"IdentityCreditTransfer",""" +
            """"owner_id":"11111111111111111111111111111111"}"""

    private inline fun <reified T> decode(json: String): T =
        PlatformBlocks.decode("test", json)

    @Test
    fun shouldDecodeABlockWithItsTransactions() {
        val list = decode<PlatformBlockList>(
            """{"proved":true,"blocks":[{"height":120,"hash":"${"09".repeat(32)}",""" +
                """"time_ms":1700000000000,"core_chain_locked_height":2100,""" +
                """"app_hash":"${"01".repeat(32)}","proposer_pro_tx_hash":"${"02".repeat(32)}",""" +
                """"transactions":[$transactionJson]}]}""",
        )
        assertTrue(list.proved)
        val block = list.blocks.single()
        assertEquals(120L, block.height)
        assertEquals("09".repeat(32), block.hash)
        assertEquals(1_700_000_000_000L, block.timeMs)
        assertEquals(2_100L, block.coreChainLockedHeight)
        assertEquals("01".repeat(32), block.appHash)
        assertEquals("02".repeat(32), block.proposerProTxHash)
        val transaction = block.transactions.single()
        assertEquals("07".repeat(32), transaction.hash)
        assertEquals(40_100L, transaction.code)
        assertFalse(transaction.executed)
        assertEquals(1_200L, transaction.gasUsed)
        assertEquals("abcd", transaction.resultData)
        assertEquals("IdentityCreditTransfer", transaction.type)
        assertEquals("11111111111111111111111111111111", transaction.ownerId)
    }

    @Test
    fun shouldDecodeATransactionThatReportsNoOwner() {
        val answer = decode<OwnerTransactionsInBlock>(
            """{"proved":false,"height":5,"transactions":[""" +
                """{"hash":"${"03".repeat(32)}","code":0,"executed":true,"gas_used":0,""" +
                """"result_data":"","type":"MasternodeVote","owner_id":null}]}""",
        )
        assertFalse(answer.proved)
        assertEquals(5L, answer.height)
        assertTrue(answer.transactions.single().executed)
        assertNull(answer.transactions.single().ownerId)
    }

    @Test
    fun shouldDecodeANodeViewWithAndWithoutAnError() {
        val committed = decode<NodeTransitionStatus>(
            """{"proved":false,"status":"committed","height":120,"error":{"code":40100,""" +
                """"message":"insufficient balance","consensus_error":null},"node_height":122}""",
        )
        assertEquals(NodeTransitionStatus.Status.COMMITTED, committed.status)
        assertEquals(120L, committed.height)
        assertEquals(40_100L, committed.error?.code)
        assertEquals("insufficient balance", committed.error?.message)
        assertNull(committed.error?.consensusError)
        assertEquals(122L, committed.nodeHeight)

        val unknown = decode<NodeTransitionStatus>(
            """{"proved":false,"status":"unknown","height":null,"error":null,"node_height":null}""",
        )
        assertEquals(NodeTransitionStatus.Status.UNKNOWN, unknown.status)
        assertNull(unknown.height)
        assertNull(unknown.error)
        assertNull(unknown.nodeHeight)
    }

    @Test
    fun shouldDecodeATransitionFoundInItsBlockAsProved() {
        val status = decode<TransitionStatusByHash>(
            """{"proved":true,"status":"in_block","block_height":120,""" +
                """"block_hash":"${"09".repeat(32)}","block_time_ms":1700000000000,""" +
                """"transaction":$transactionJson}""",
        )
        assertTrue(status.proved)
        assertEquals(TransitionStatusByHash.Status.IN_BLOCK, status.status)
        assertEquals(120L, status.blockHeight)
        assertEquals("09".repeat(32), status.blockHash)
        assertEquals(1_700_000_000_000L, status.blockTimeMs)
        assertEquals(40_100L, status.transaction?.code)
        assertNull(status.nodeHeight)
    }

    @Test
    fun shouldDecodeEveryUnprovedStatusByHash() {
        val cases = mapOf(
            """{"proved":false,"status":"committed_not_proved","height":120,"error":null,"node_height":122}"""
                to TransitionStatusByHash.Status.COMMITTED_NOT_PROVED,
            """{"proved":false,"status":"committed_refuted","height":120,"node_height":122}"""
                to TransitionStatusByHash.Status.COMMITTED_REFUTED,
            """{"proved":false,"status":"pending","node_height":122}"""
                to TransitionStatusByHash.Status.PENDING,
            """{"proved":false,"status":"unknown","node_height":null}"""
                to TransitionStatusByHash.Status.UNKNOWN,
        )
        for ((json, expected) in cases) {
            val status = decode<TransitionStatusByHash>(json)
            assertFalse(json, status.proved)
            assertEquals(json, expected, status.status)
            assertNull(json, status.transaction)
        }
        val refuted = decode<TransitionStatusByHash>(cases.keys.elementAt(1))
        assertEquals(120L, refuted.height)
        assertEquals(122L, refuted.nodeHeight)
    }

    @Test
    fun shouldIgnoreKeysANewerFfiAdds() {
        val list = decode<PlatformBlockList>("""{"proved":false,"blocks":[],"new_key":1}""")
        assertFalse(list.proved)
        assertTrue(list.blocks.isEmpty())
    }

    @Test
    fun shouldReportAMissingOrUndecodableAnswerAsASerializationError() {
        assertThrows(DashSdkError.SerializationError::class.java) {
            PlatformBlocks.decode<PlatformBlockList>("test", null)
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            decode<TransitionStatusByHash>("""{"proved":false,"status":"lost"}""")
        }
        assertThrows(DashSdkError.SerializationError::class.java) {
            decode<PlatformBlockList>("not json")
        }
    }
}
