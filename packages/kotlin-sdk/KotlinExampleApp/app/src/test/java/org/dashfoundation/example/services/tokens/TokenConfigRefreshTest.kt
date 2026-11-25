package org.dashfoundation.example.services.tokens

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertSame
import org.junit.Test

class TokenConfigRefreshTest {

    private val pauses = mutableListOf<Long>()
    private val floors = mutableListOf<Long>()

    @Test
    fun `should refresh with the version above the stored one as the floor`() = runBlocking {
        val failure = TokenConfigRefresh.afterConfirmedUpdate(
            storedVersion = 1,
            pause = { pauses += it },
        ) { floors += it }

        assertNull(failure)
        // The update raised the contract from version 1 to 2: an answer still
        // at version 1 (the old max supply) must be refused.
        assertEquals(listOf(2L), floors)
        assertEquals(emptyList<Long>(), pauses)
    }

    @Test
    fun `should try again when a node has not seen the update yet`() = runBlocking {
        var calls = 0
        val failure = TokenConfigRefresh.afterConfirmedUpdate(
            storedVersion = 4,
            pause = { pauses += it },
        ) { floor ->
            floors += floor
            calls++
            if (calls < 3) throw Exception("The node returned version 4, older than version 5")
        }

        assertNull(failure)
        assertEquals(listOf(5L, 5L, 5L), floors)
        assertEquals(List(2) { TokenConfigRefresh.RETRY_DELAY_MS }, pauses)
    }

    @Test
    fun `should give up after the last attempt and return its failure`() = runBlocking {
        val last = Exception("third")
        var calls = 0
        val failure = TokenConfigRefresh.afterConfirmedUpdate(
            storedVersion = 2,
            pause = { pauses += it },
        ) {
            calls++
            throw if (calls == TokenConfigRefresh.ATTEMPTS) last else Exception("lagging")
        }

        assertSame(last, failure)
        assertEquals(TokenConfigRefresh.ATTEMPTS, calls)
    }

    @Test
    fun `should not refresh without a stored contract`() = runBlocking {
        val failure = TokenConfigRefresh.afterConfirmedUpdate(storedVersion = null) { floors += it }

        assertNull(failure)
        assertEquals(emptyList<Long>(), floors)
    }
}
