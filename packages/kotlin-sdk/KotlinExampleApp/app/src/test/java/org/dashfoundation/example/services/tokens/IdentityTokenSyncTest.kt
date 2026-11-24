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
 * Wiring of [IdentityTokenSync]: a screen registers the identity it shows
 * with the canonical ids of every local token, registers the wallet's other
 * identities with none, and runs one sync pass, the way
 * `IdentityDetailView.reloadTokenBalances` does for its identity in
 * SwiftExampleApp. Token ids come from the injected native calculation; the
 * test only checks they are passed through.
 */
class IdentityTokenSyncTest {

    private val identityId = ByteArray(32) { 7 }
    private val otherIdentityId = ByteArray(32) { 8 }

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
        val registered = mutableListOf<Pair<ByteArray, List<ByteArray>>>()
        val register: suspend (ByteArray, List<ByteArray>) -> Unit = { id, tokens ->
            calls += "register"
            registered += id to tokens
        }
        val syncNow: suspend () -> Unit = { calls += "syncNow" }

        fun tokensFor(id: ByteArray): List<ByteArray> =
            registered.last { it.first.contentEquals(id) }.second
    }

    @Test
    fun shouldWatchEveryLocalTokenForTheIdentityOnScreenThenSyncOnce() = runTest {
        val tokens = listOf(token(1, 0), token(1, 1), token(2, 0))
        val recorder = Recorder()

        val done = IdentityTokenSync.refresh(
            identityId, listOf(identityId), tokens, calculate, recorder.register, recorder.syncNow,
        )

        assertTrue(done)
        assertEquals(listOf("register", "syncNow"), recorder.calls)
        val expected = tokens.map { fakeTokenId(Base58.encode(it.contractId), it.position) }
        val watched = recorder.tokensFor(identityId)
        assertEquals(expected.size, watched.size)
        expected.zip(watched).forEach { (want, got) -> assertArrayEquals(want, got) }
    }

    @Test
    fun shouldClearTheOtherIdentitiesSoAPassReadsOnlyTheIdentityOnScreen() = runTest {
        val recorder = Recorder()

        IdentityTokenSync.refresh(
            identityId, listOf(otherIdentityId, identityId), listOf(token(1, 0)), calculate,
            recorder.register, recorder.syncNow,
        )

        assertEquals(listOf("register", "register", "syncNow"), recorder.calls)
        assertTrue("other identity watches nothing", recorder.tokensFor(otherIdentityId).isEmpty())
        assertEquals(1, recorder.tokensFor(identityId).size)
    }

    @Test
    fun shouldStopTheWatchAndSkipThePassWhenNoTokenIsSavedLocally() = runTest {
        val recorder = Recorder()

        val done = IdentityTokenSync.refresh(
            identityId, listOf(identityId), emptyList(), calculate,
            recorder.register, recorder.syncNow,
        )

        assertFalse(done)
        assertEquals("registered with no tokens, no pass", listOf("register"), recorder.calls)
        assertTrue(recorder.tokensFor(identityId).isEmpty())
    }

    @Test
    fun shouldClearEveryIdentityWithoutASyncPassBeforeAContractIsDeleted() = runTest {
        val recorder = Recorder()

        val cleared = IdentityTokenSync.clear(listOf(identityId, otherIdentityId), recorder.register)

        assertTrue(cleared)
        assertEquals(listOf("register", "register"), recorder.calls)
        assertTrue(recorder.registered.all { it.second.isEmpty() })
    }

    @Test
    fun shouldSkipTokensWhoseIdCannotBeCalculated() = runTest {
        val good = token(1, 0)
        val failing = token(2, 0)
        val failingContract = Base58.encode(failing.contractId)
        val recorder = Recorder()

        IdentityTokenSync.refresh(
            identityId,
            listOf(identityId),
            listOf(good, failing),
            { contract, position ->
                if (contract == failingContract) error("native failure") else calculate(contract, position)
            },
            recorder.register,
            recorder.syncNow,
        )

        assertArrayEquals(
            fakeTokenId(Base58.encode(good.contractId), 0),
            recorder.tokensFor(identityId).single(),
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
            listOf(identityId),
            listOf(token(1, 0)),
            calculate,
            { _, _ -> error("dead manager") },
            { synced = true },
        )

        assertFalse(done)
        assertFalse("no sync pass after a failed registration", synced)
    }
}
