package org.dashfoundation.dashsdk.security

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.fail
import org.junit.Test

/**
 * Pins the seed-hygiene invariant of [signWithScrubbedMnemonic]: the platform-
 * address signing path passes the mnemonic as a caller-owned [ByteArray] and
 * zeroes it after use, so the plaintext never lingers on the JVM heap past the
 * call (the un-scrubbable `String` path it replaces could not do this).
 *
 * Red→green: with the helper's `finally { fill(0) }` removed, the phrase buffer
 * survives the call and [scrubsMnemonicBytesAfterSigning] /
 * [scrubsMnemonicBytesEvenWhenSignThrows] both fail.
 */
class SignerMnemonicScrubTest {

    @Test
    fun scrubsMnemonicBytesAfterSigning() {
        val phrase = "abandon abandon abandon about".toByteArray(Charsets.UTF_8)
        val original = phrase.copyOf()
        var seenAtSignTime: ByteArray? = null

        val signature = signWithScrubbedMnemonic(
            mnemonicUtf8 = phrase,
            derivationPath = "m/9'/5'/3'/0/0",
            network = 1,
            data = byteArrayOf(1, 2, 3),
        ) { m, _, _, _ ->
            // The signer sees the real phrase bytes (scrub happens AFTER, not before).
            seenAtSignTime = m.copyOf()
            byteArrayOf(0xAB.toByte(), 0xCD.toByte())
        }

        assertArrayEquals("signer must receive the intact phrase", original, seenAtSignTime)
        assertArrayEquals("signature is returned unchanged", byteArrayOf(0xAB.toByte(), 0xCD.toByte()), signature)
        assertArrayEquals("caller buffer must be zeroed after the call", ByteArray(phrase.size), phrase)
    }

    @Test
    fun scrubsMnemonicBytesEvenWhenSignThrows() {
        val phrase = "zoo zoo zoo wrong".toByteArray(Charsets.UTF_8)
        try {
            signWithScrubbedMnemonic(phrase, "m/9'", 1, byteArrayOf()) { _, _, _, _ ->
                throw RuntimeException("derive-and-sign failed")
            }
            fail("expected the sign failure to propagate")
        } catch (_: RuntimeException) {
            // expected
        }
        assertArrayEquals("caller buffer must be zeroed even when signing throws", ByteArray(phrase.size), phrase)
    }
}
