package org.dashfoundation.example.ui.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class RevealSessionTest {

    @Test
    fun `should show a key derived after a PIN round trip`() {
        val session = RevealSession()
        val token = started(session.begin())
        // The device-credential screen stops the activity while the prompt is up.
        session.onStop()
        assertTrue(session.authorized(token))
        assertTrue(session.finish(token))
        assertEquals(RevealSession.Phase.IDLE, session.phase)
    }

    @Test
    fun `should refuse a key derived after the app stopped`() {
        val session = RevealSession()
        val token = started(session.begin())
        assertTrue(session.authorized(token))
        session.onStop()
        assertFalse(session.finish(token))
        assertEquals(RevealSession.Phase.IDLE, session.phase)
    }

    @Test
    fun `should refuse a result once the screen cleared the reveal`() {
        val session = RevealSession()
        val token = started(session.begin())
        session.invalidate()
        assertFalse(session.authorized(token))
        assertFalse(session.finish(token))
    }

    @Test
    fun `should allow one reveal at a time and a new one after a cancelled prompt`() {
        val session = RevealSession()
        val first = started(session.begin())
        assertNull(session.begin())
        session.abandon(first)
        val second = started(session.begin())
        assertFalse(session.authorized(first))
        assertTrue(session.authorized(second))
    }

    private fun started(token: Long?): Long {
        assertNotNull("a reveal should start", token)
        return token!!
    }
}
