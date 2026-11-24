package org.dashfoundation.example.services.tokens

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.persistence.entities.TokenEntity
import org.dashfoundation.example.util.Base58
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Wiring of [IdentityTokenSync]: the Tokens screen registers the selected
 * identity with the canonical ids of every local token and then runs one
 * sync pass, the way `IdentityDetailView.reloadTokenBalances` does in
 * SwiftExampleApp. Token ids come from the injected native calculation; the
 * test only checks they are passed through.
 */
class IdentityTokenSyncTest {

    private val identityId = ByteArray(32) { 7 }

    private fun token(contractTag: Int, position: Int) = TokenEntity(
        id = ByteArray(32) { contractTag.toByte() } + byteArrayOf(0, 0, 0, position.toByte()),
        contractId = ByteArray(32) { contractTag.toByte() },
        position = position,
        name = "Token $contractTag/$position",
        baseSupply = "1000",
    )

    /** A stand-in native id, distinct per (contract, position): the contract id with byte 0 = position. */
    private fun fakeTokenId(contractIdBase58: String, position: Int): ByteArray {
        val contract = Base58.decodeIdentifier(contractIdBase58)!!
        return contract.copyOf().also { it[0] = (position + 100).toByte() }
    }

    private val calculate: suspend (String, Int) -> String? = { contract, position ->
        Base58.encode(fakeTokenId(contract, position))
    }

    private class Recorder {
        val calls = mutableListOf<String>()
        var registeredIdentity: ByteArray? = null
        var registeredTokens: List<ByteArray> = emptyList()
        val register: suspend (ByteArray, List<ByteArray>) -> Unit = { id, tokens ->
            calls += "register"
            registeredIdentity = id
            registeredTokens = tokens
        }
        val syncNow: suspend () -> Unit = { calls += "syncNow" }
    }

    @Test
    fun shouldRegisterEveryLocalTokenThenRunOneSyncPass() = runTest {
        val tokens = listOf(token(1, 0), token(1, 1), token(2, 0))
        val recorder = Recorder()

        val done = IdentityTokenSync.refresh(
            identityId, tokens, calculate, recorder.register, recorder.syncNow,
        )

        assertTrue(done)
        assertEquals(listOf("register", "syncNow"), recorder.calls)
        assertArrayEquals(identityId, recorder.registeredIdentity)
        val expected = tokens.map { fakeTokenId(Base58.encode(it.contractId), it.position) }
        assertEquals(expected.size, recorder.registeredTokens.size)
        expected.zip(recorder.registeredTokens).forEach { (want, got) ->
            assertArrayEquals(want, got)
        }
    }

    @Test
    fun shouldRegisterNothingWhenNoTokenIsSavedLocally() = runTest {
        val recorder = Recorder()

        val done = IdentityTokenSync.refresh(
            identityId, emptyList(), calculate, recorder.register, recorder.syncNow,
        )

        assertFalse(done)
        assertTrue(recorder.calls.isEmpty())
    }

    @Test
    fun shouldSkipTokensWhoseIdCannotBeCalculated() = runTest {
        val good = token(1, 0)
        val failing = token(2, 0)
        val failingContract = Base58.encode(failing.contractId)
        val recorder = Recorder()

        IdentityTokenSync.refresh(
            identityId,
            listOf(good, failing),
            { contract, position ->
                if (contract == failingContract) error("native failure") else calculate(contract, position)
            },
            recorder.register,
            recorder.syncNow,
        )

        assertEquals(1, recorder.registeredTokens.size)
        assertArrayEquals(
            fakeTokenId(Base58.encode(good.contractId), 0),
            recorder.registeredTokens.single(),
        )
    }

    @Test
    fun shouldSkipPositionsOutsideTheU16Range() = runTest {
        val tokenIds = IdentityTokenSync.watchedTokenIds(
            listOf(token(1, -1), token(1, 65_536), token(1, 65_535)),
            calculate,
        )

        assertEquals(1, tokenIds.size)
    }

    @Test
    fun shouldWatchEachTokenIdOnce() = runTest {
        val tokenIds = IdentityTokenSync.watchedTokenIds(
            listOf(token(1, 0), token(1, 0)),
            calculate,
        )

        assertEquals(1, tokenIds.size)
    }

    @Test
    fun shouldReportFailureWithoutThrowingWhenTheNativeCallFails() = runTest {
        var synced = false

        val done = IdentityTokenSync.refresh(
            identityId,
            listOf(token(1, 0)),
            calculate,
            { _, _ -> error("dead manager") },
            { synced = true },
        )

        assertFalse(done)
        assertFalse("no sync pass after a failed registration", synced)
    }
}
