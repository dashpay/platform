package org.dashfoundation.example.services.faucet

import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.async
import kotlinx.coroutines.awaitAll
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeout
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

/**
 * **Dev/QA tool only.** App-level client for the testnet DASH faucet at
 * `faucet.thepasta.org` — port of `TestnetFaucetService.swift` (issue #3897).
 *
 * A plain `HttpURLConnection` client plus a pure-Kotlin `cap.js`
 * proof-of-work solver. It deliberately does **not** touch the wallet /
 * Platform FFI, `platform-wallet`, or any SDK business logic, so it lives in
 * the app layer, not the SDK. It lets QA top up a fresh/drained testnet
 * wallet with one tap instead of round-tripping through the web faucet.
 *
 * The faucet enforces a `cap.js` captcha; the soft challenge is a small
 * proof-of-work solved on-device off the main thread. The hard challenge
 * (returned on rate limit) is too expensive to brute-force here, so on rate
 * limit or any failure the caller falls back to the web faucet.
 */

/** cap.js proof-of-work solver (bit-identical to `@cap.js/widget`). */
internal object CapSolver {
    /**
     * cap.js seeded PRNG: FNV-1a-style seed + xorshift32. Emits [length]
     * lowercase hex chars. All arithmetic is unsigned 32-bit ([UInt] wraps).
     */
    fun prng(seed: String, length: Int): String {
        var state = 2166136261u // 0x811C9DC5
        for (ch in seed) { // seeds are ASCII (hex token + digits [+ "d"])
            state = state xor ch.code.toUInt()
            val sh = (state shl 1) + (state shl 4) + (state shl 7) + (state shl 8) + (state shl 24)
            state += sh
        }
        val out = StringBuilder()
        while (out.length < length) {
            state = state xor (state shl 13)
            state = state xor (state shr 17)
            state = state xor (state shl 5)
            // Zero-extended unsigned value → 8 lowercase hex chars.
            out.append(state.toLong().toString(16).padStart(8, '0'))
        }
        return out.substring(0, length)
    }

    /**
     * Brute-force the smallest non-negative `nonce` such that
     * lowercase-hex(`SHA256(salt + nonce)`) starts with [target]. Compares
     * against the raw digest bytes rather than materializing the hex string.
     * Suspend: checks cancellation every 0x10000 attempts so the caller's
     * `withTimeout` can actually preempt a slow challenge (← Swift
     * `CapSolver.solve`'s `Task.checkCancellation()` cadence — a worst-case
     * solve must not pin a core after the sheet already gave up).
     */
    suspend fun solve(salt: String, target: String): Long {
        val fullBytes = target.length / 2
        val hasNibble = target.length % 2 == 1
        val prefix = ByteArray(fullBytes) { i ->
            ((hexNibble(target[i * 2]) shl 4) or hexNibble(target[i * 2 + 1])).toByte()
        }
        val nibble = if (hasNibble) hexNibble(target[fullBytes * 2]) else 0

        val saltBytes = salt.toByteArray(Charsets.UTF_8)
        val md = MessageDigest.getInstance("SHA-256")
        var nonce = 0L
        while (true) {
            md.reset()
            md.update(saltBytes)
            md.update(nonce.toString().toByteArray(Charsets.UTF_8))
            val digest = md.digest()
            if (digestHasPrefix(digest, prefix, nibble, hasNibble)) return nonce
            nonce++
            if (nonce and 0xFFFF == 0L) {
                kotlin.coroutines.coroutineContext.ensureActive()
            }
        }
    }

    private fun digestHasPrefix(
        digest: ByteArray,
        prefix: ByteArray,
        nibble: Int,
        hasNibble: Boolean,
    ): Boolean {
        for (i in prefix.indices) {
            if (digest[i] != prefix[i]) return false
        }
        if (hasNibble) {
            return ((digest[prefix.size].toInt() and 0xFF) ushr 4) == nibble
        }
        return true
    }

    private fun hexNibble(c: Char): Int = when (c) {
        in '0'..'9' -> c - '0'
        in 'a'..'f' -> c - 'a' + 10
        in 'A'..'F' -> c - 'A' + 10
        else -> 0
    }
}

/** Result of a faucet funding attempt. */
sealed interface TestnetFaucetOutcome {
    /** Funds sent. [txid] is the Core tx id; [amount] the tDASH amount. */
    data class Sent(val txid: String, val amount: Double) : TestnetFaucetOutcome

    /** Server rate-limited (429). */
    data class RateLimited(val message: String) : TestnetFaucetOutcome

    /** Any other failure (network, solve, non-200). */
    data class Failed(val reason: String) : TestnetFaucetOutcome
}

/** Thin suspend client for the testnet faucet. Stateless; create one per use. */
class TestnetFaucet {
    private val host = "https://faucet.thepasta.org"
    private val json = Json { ignoreUnknownKeys = true }

    @Serializable
    private data class FaucetStatus(val capEndpoint: String, val coreFaucetAmount: Double)

    @Serializable
    private data class Challenge(val c: Int, val s: Int, val d: Int)

    @Serializable
    private data class ChallengeResponse(val challenge: Challenge, val token: String)

    @Serializable
    private data class RedeemRequest(val token: String, val solutions: List<Long>)

    @Serializable
    private data class RedeemResponse(val success: Boolean, val token: String? = null)

    @Serializable
    private data class FaucetResult(val txid: String? = null)

    /**
     * Request ~1 tDASH to [address]. Solves the soft captcha on-device and
     * posts to `/api/core-faucet`. Never throws — always maps to an outcome.
     */
    suspend fun requestCoreDash(address: String): TestnetFaucetOutcome {
        val trimmed = address.trim()
        if (trimmed.isEmpty()) return TestnetFaucetOutcome.Failed("No address available")

        val status = try {
            getJson("$host/api/status", FaucetStatus.serializer())
        } catch (e: Exception) {
            return TestnetFaucetOutcome.Failed("Faucet status unavailable: ${e.message}")
        }

        // Defense-in-depth: only solve against an HTTPS endpoint on the
        // faucet's own registrable domain, so a tampered response can't
        // downgrade the exchange or repoint the address-carrying POSTs.
        val capBase = try {
            URL(status.capEndpoint)
        } catch (e: Exception) {
            return TestnetFaucetOutcome.Failed("Untrusted captcha endpoint")
        }
        if (capBase.protocol != "https" ||
            !sameRegistrableDomain(capBase.host, URL(host).host)
        ) {
            return TestnetFaucetOutcome.Failed("Untrusted captcha endpoint")
        }

        val capToken = try {
            solveCaptcha(status.capEndpoint)
        } catch (e: Exception) {
            return TestnetFaucetOutcome.Failed("Captcha solve failed: ${e.message}")
        }

        return postFaucet(trimmed, capToken, status.coreFaucetAmount)
    }

    /** cap.js handshake: challenge → brute-force the sub-challenges → redeem. */
    private suspend fun solveCaptcha(capEndpoint: String): String {
        val base = capEndpoint.trimEnd('/')
        val challenge = postJson(
            "$base/challenge",
            "{}",
            ChallengeResponse.serializer(),
        )
        val c = challenge.challenge.c
        val s = challenge.challenge.s
        val d = challenge.challenge.d
        val token = challenge.token

        // Reject pathological challenge sizes up front (a bad/future faucet
        // response could otherwise pin every core with no in-app abort).
        require(c in 1..256 && s in 1..256 && d in 1..6) {
            "Unsupported captcha challenge (c=$c, s=$s, d=$d)"
        }

        val solutions: List<Long> = withTimeout(30_000) {
            coroutineScope {
                (1..c).map { i ->
                    async(Dispatchers.Default) {
                        val salt = CapSolver.prng("$token$i", s)
                        val target = CapSolver.prng("$token${i}d", d)
                        CapSolver.solve(salt, target)
                    }
                }.awaitAll()
            }
        }

        val redeem = postJson(
            "$base/redeem",
            json.encodeToString(RedeemRequest.serializer(), RedeemRequest(token, solutions)),
            RedeemResponse.serializer(),
        )
        require(redeem.success && redeem.token != null) { "Captcha redeem rejected" }
        return redeem.token
    }

    /** POSTs the funding request and maps the response to an outcome. */
    private suspend fun postFaucet(
        address: String,
        capToken: String,
        amount: Double,
    ): TestnetFaucetOutcome = withContext(Dispatchers.IO) {
        try {
            val conn = openPost("$host/api/core-faucet")
            val body = json.encodeToString(
                CoreFaucetRequest.serializer(),
                CoreFaucetRequest(address, capToken),
            )
            conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
            when (val code = conn.responseCode) {
                200 -> {
                    val txid = json.decodeFromString(
                        FaucetResult.serializer(),
                        conn.inputStream.bufferedReader().use { it.readText() },
                    ).txid
                    if (txid != null) {
                        TestnetFaucetOutcome.Sent(txid, amount)
                    } else {
                        TestnetFaucetOutcome.Failed("Unexpected success payload")
                    }
                }
                429 -> TestnetFaucetOutcome.RateLimited(
                    "Rate limited (3/hour per IP). Try the web faucet.",
                )
                else -> {
                    val detail = conn.errorStream?.bufferedReader()?.use { it.readText() }
                        ?.take(120) ?: ""
                    TestnetFaucetOutcome.Failed("Faucet error $code: $detail")
                }
            }
        } catch (e: Exception) {
            TestnetFaucetOutcome.Failed("Network error: ${e.message}")
        }
    }

    @Serializable
    private data class CoreFaucetRequest(val address: String, val capToken: String)

    // --- HTTP helpers -------------------------------------------------------

    private suspend fun <T> getJson(
        url: String,
        deserializer: kotlinx.serialization.DeserializationStrategy<T>,
    ): T = withContext(Dispatchers.IO) {
        val conn = (URL(url).openConnection() as HttpURLConnection).apply {
            requestMethod = "GET"
            connectTimeout = 30_000
            readTimeout = 30_000
        }
        if (conn.responseCode != 200) throw Exception("HTTP ${conn.responseCode}")
        json.decodeFromString(deserializer, conn.inputStream.bufferedReader().use { it.readText() })
    }

    private suspend fun <T> postJson(
        url: String,
        body: String,
        deserializer: kotlinx.serialization.DeserializationStrategy<T>,
    ): T = withContext(Dispatchers.IO) {
        val conn = openPost(url)
        conn.outputStream.use { it.write(body.toByteArray(Charsets.UTF_8)) }
        if (conn.responseCode != 200) throw Exception("HTTP ${conn.responseCode}")
        json.decodeFromString(deserializer, conn.inputStream.bufferedReader().use { it.readText() })
    }

    private fun openPost(url: String): HttpURLConnection =
        (URL(url).openConnection() as HttpURLConnection).apply {
            requestMethod = "POST"
            connectTimeout = 30_000
            readTimeout = 30_000
            doOutput = true
            setRequestProperty("Content-Type", "application/json")
        }

    companion object {
        /** Web-faucet URL, used as the fallback when the API path fails. */
        const val WEB_URL = "https://faucet.thepasta.org/"

        /**
         * True when two hosts share their registrable domain (last two DNS
         * labels), e.g. `cap.thepasta.org` and `faucet.thepasta.org`.
         */
        private fun sameRegistrableDomain(a: String, b: String): Boolean {
            fun base(h: String) = h.split(".").takeLast(2).joinToString(".")
            val ba = base(a)
            return ba.contains(".") && ba == base(b)
        }
    }
}
