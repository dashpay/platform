package org.dashfoundation.dashsdk

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.IOException
import java.net.InetAddress
import java.net.ServerSocket
import java.net.URI
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicBoolean
import kotlin.concurrent.thread

/**
 * Mirrors the decoding tolerance of `SDK.discoverActiveMasternodes` in
 * `SwiftDashSDK/SDK.swift`: ENABLED + versionCheck=success filter, default
 * DAPI port 443, lenient optional fields.
 */
class MasternodeDiscoveryTest {

    @Test
    fun oneArgumentDiscoveryOverloadRemainsCallableAtItsOriginalJvmSignature() {
        assertNull(Sdk.discoverActiveMasternodes("://not-a-valid-url"))
        assertNotNull(
            Sdk.Companion::class.java.getDeclaredMethod(
                "discoverActiveMasternodes",
                String::class.java,
            ),
        )
    }

    @Test
    fun shouldReadABodyWithinTheSizeAndTimeBounds() {
        val body = "{\"success\": true, \"data\": []}"
        assertEquals(
            body,
            Sdk.readBounded(ByteArrayInputStream(body.toByteArray()), body.length, Long.MAX_VALUE),
        )
    }

    @Test
    fun shouldRefuseABodyLargerThanTheCap() {
        val body = ByteArray(20_000) { 'a'.code.toByte() }
        assertNull(Sdk.readBounded(ByteArrayInputStream(body), 19_999, Long.MAX_VALUE))
    }

    @Test
    fun shouldGiveUpOnAServerThatTricklesItsHeadersPastTheDeadline() {
        // Bind and connect to the same IPv4 loopback address: a server on
        // ::1 would refuse the client and pass the test without a deadline.
        ServerSocket(0, 1, InetAddress.getByName("127.0.0.1")).use { server ->
            val accepted = AtomicBoolean(false)
            val trickler = thread(isDaemon = true) {
                try {
                    server.accept().use { socket ->
                        accepted.set(true)
                        val out = socket.getOutputStream()
                        out.write("HTTP/1.1 200 OK\r\n".toByteArray())
                        // One header byte every 200 ms: each read finishes
                        // well inside the read timeout, and the headers do
                        // not end before the asserted limit. The server
                        // gives up after 10 s, so a broken deadline fails
                        // the assertion instead of stalling the test job.
                        repeat(50) {
                            out.write('x'.code)
                            out.flush()
                            Thread.sleep(200)
                        }
                    }
                } catch (_: Exception) {
                    // The client disconnected.
                }
            }
            try {
                val url = URI("http://127.0.0.1:${server.localPort}/masternodes").toURL()
                val started = System.nanoTime()
                assertNull(Sdk.fetchBounded(url, deadlineMs = 1_000))
                val elapsedMs = (System.nanoTime() - started) / 1_000_000
                assertTrue("took $elapsedMs ms", elapsedMs < 4_000)
                assertTrue("the client never connected", accepted.get())
            } finally {
                trickler.interrupt()
            }
        }
    }

    @Test
    fun shouldReturnAtTheDeadlineWhenTheRequestCannotBeAborted() {
        // A host name lookup has no socket to close, and disconnecting can
        // wait for a read in progress: the caller must stop waiting at the
        // deadline even though the abort neither works nor returns.
        val aborted = CountDownLatch(1)
        val blocked = CountDownLatch(1)
        val started = System.nanoTime()
        try {
            assertThrows(IOException::class.java) {
                Sdk.withDeadline(
                    200,
                    abort = {
                        aborted.countDown()
                        blocked.await()
                    },
                ) { blocked.await() }
            }
            val elapsedMs = (System.nanoTime() - started) / 1_000_000
            assertTrue("took $elapsedMs ms", elapsedMs < 2_000)
            assertTrue("the request was not told to stop", aborted.await(2, TimeUnit.SECONDS))
        } finally {
            blocked.countDown()
        }
    }

    @Test
    fun shouldRefuseANewRequestWhileTooManyAbandonedOnesAreStillRunning() {
        val blocked = CountDownLatch(1)
        try {
            repeat(Sdk.MAX_OUTSTANDING_REQUESTS) {
                assertThrows(IOException::class.java) {
                    Sdk.withDeadline(50, abort = {}) { blocked.await() }
                }
            }
            // Every slot is held by a request its caller gave up on.
            var ran = false
            assertThrows(IOException::class.java) {
                Sdk.withDeadline(5_000, abort = {}) { ran = true }
            }
            assertFalse(ran)
        } finally {
            blocked.countDown()
        }
        // The slots come back once those requests end.
        val giveUpAt = System.nanoTime() + 5_000_000_000L
        while (true) {
            try {
                assertEquals("body", Sdk.withDeadline(5_000, abort = {}) { "body" })
                break
            } catch (e: IOException) {
                if (System.nanoTime() > giveUpAt) throw e
                Thread.sleep(10)
            }
        }
    }

    @Test
    fun shouldPassOnTheResultOrTheFailureOfARequestThatFinishesInTime() {
        assertEquals("body", Sdk.withDeadline(5_000, abort = {}) { "body" })
        val failure = assertThrows(IllegalStateException::class.java) {
            Sdk.withDeadline<String>(5_000, abort = {}) { error("refused") }
        }
        assertEquals("refused", failure.message)
    }

    @Test
    fun shouldStopReadingOnceTheDeadlinePasses() {
        val body = ByteArray(20_000) { 'a'.code.toByte() }
        var now = 0L
        // Each clock read advances one second; the deadline is two seconds in.
        val clock = { now.also { now += 1_000_000_000L } }
        assertNull(Sdk.readBounded(ByteArrayInputStream(body), Int.MAX_VALUE, 2_000_000_000L, clock))
    }

    @Test
    fun filtersToEnabledAndVersionCheckedNodes() {
        val body = """
            {"success": true, "data": [
              {"address": "34.1.2.3:19999", "status": "ENABLED", "platformHTTPPort": 1443, "versionCheck": "success"},
              {"address": "34.1.2.4:19999", "status": "ENABLED", "platformHTTPPort": 1443, "versionCheck": "failed"},
              {"address": "34.1.2.5:19999", "status": "POSE_BANNED", "platformHTTPPort": 1443, "versionCheck": "success"}
            ]}
        """.trimIndent()
        val active = Sdk.parseActiveMasternodes(body)!!
        assertEquals(1, active.size)
        assertEquals("34.1.2.3:19999", active[0].spvPeer)
        assertEquals("https://34.1.2.3:1443", active[0].dapiUrl)
    }

    @Test
    fun missingPlatformPortFallsBackTo443() {
        val body = """
            {"success": true, "data": [
              {"address": "1.2.3.4:9999", "status": "ENABLED", "versionCheck": "success"}
            ]}
        """.trimIndent()
        assertEquals("https://1.2.3.4:443", Sdk.parseActiveMasternodes(body)!![0].dapiUrl)
        assertEquals("https://1.2.3.4:1443", Sdk.parseActiveMasternodes(body, 1443)!![0].dapiUrl)
    }

    @Test
    fun bracketedIpv6AddressPreservesTheWholeHost() {
        val body = """
            {"success": true, "data": [
              {"address": "[2001:db8::1]:19999", "status": "ENABLED", "platformHTTPPort": 1443, "versionCheck": "success"}
            ]}
        """.trimIndent()

        assertEquals(
            "https://[2001:db8::1]:1443",
            Sdk.parseActiveMasternodes(body)!![0].dapiUrl,
        )
    }

    @Test
    fun missingVersionCheckIsExcluded() {
        val body = """
            {"success": true, "data": [
              {"address": "1.2.3.4:9999", "status": "ENABLED"}
            ]}
        """.trimIndent()
        assertNull(Sdk.parseActiveMasternodes(body))
    }

    @Test
    fun toleratesUnknownFieldsAndFailsClosedOnBadShape() {
        val ok = """
            {"success": true, "extra": 1, "data": [
              {"address": "1.2.3.4:9999", "status": "ENABLED", "versionCheck": "success", "unknown": {"x": 1}}
            ]}
        """.trimIndent()
        assertEquals(1, Sdk.parseActiveMasternodes(ok)!!.size)

        assertNull(Sdk.parseActiveMasternodes("""{"success": false, "data": []}"""))
        assertNull(Sdk.parseActiveMasternodes("not json"))
        assertNull(Sdk.parseActiveMasternodes("""{"success": true, "data": []}"""))
    }
}
