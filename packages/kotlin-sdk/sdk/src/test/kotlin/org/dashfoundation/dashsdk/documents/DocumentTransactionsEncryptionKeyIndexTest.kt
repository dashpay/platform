package org.dashfoundation.dashsdk.documents

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Argument handling for [DocumentTransactions.createEncryptedDocument].
 *
 * This wrapper owns exactly one decision about the encrypted-document
 * arguments: `null` is how a caller says "no index supplied", and an explicit
 * index must therefore be non-negative, because a negative value is neither an
 * index nor the omission. Everything else — which wire versions are meaningful,
 * how large a payload may be, which indices have a derivable key — is decided by
 * the wallet core, so this layer must not reject those values on its own.
 *
 * Rejections happen before any native call, so they are testable on the JVM with
 * no JNI library loaded. Arguments that PASS proceed into native, which a JVM
 * unit test cannot complete; those cases assert only that the failure is NOT an
 * [IllegalArgumentException] from this wrapper.
 */
class DocumentTransactionsEncryptionKeyIndexTest {

    private val id32 = ByteArray(32)
    private val payload = ByteArray(4) { it.toByte() }

    /**
     * Call the wrapper and return whatever it threw, or `null` on success.
     *
     * A JVM unit test has no JNI library, so a call that gets past this
     * wrapper's guards fails inside the native layer instead. Returning the
     * throwable lets each case assert on which layer refused.
     */
    private fun createReturningFailure(
        version: Int,
        encryptionKeyIndex: Int? = null,
    ): Throwable? = runCatching {
        runBlocking {
            DocumentTransactions().createEncryptedDocument(
                walletHandle = 0L,
                mnemonicResolverHandle = 0L,
                ownerId = id32,
                contractId = id32,
                documentType = "txMetadata",
                version = version,
                payload = payload,
                signerHandle = 0L,
                encryptionKeyIndex = encryptionKeyIndex,
            )
        }
    }.exceptionOrNull()

    /**
     * The wrapper does not decide which wire versions are meaningful.
     *
     * Only the wallet core knows which version bytes the legacy stack can
     * decode, and it rejects an unsupported one before anything is sealed. A
     * guard here would be a second place where that set is written down, free to
     * drift from the core and to reject a value a later core accepts. So every
     * value a caller can pass must get past this layer — including ones the core
     * will refuse.
     */
    @Test
    fun doesNotRejectAnyVersionLocally() {
        for (version in intArrayOf(-1, 0, 1, 2, 3, 127, 255, 256, Int.MAX_VALUE, Int.MIN_VALUE)) {
            val failure = createReturningFailure(version = version, encryptionKeyIndex = 0)
            assertFalse(
                "version=$version must not be rejected by the Kotlin wrapper; " +
                    "representation narrowing and version policy belong to the " +
                    "native layers, got: $failure",
                failure is IllegalArgumentException,
            )
        }
    }

    /**
     * An explicit NEGATIVE index is a caller error this layer does own.
     *
     * `null` is how the API expresses "no index supplied", so a negative number
     * denotes neither an index nor the omission and cannot be forwarded as
     * either.
     */
    @Test
    fun rejectsAnExplicitNegativeIndex() {
        for (index in intArrayOf(-1, -5, Int.MIN_VALUE)) {
            val failure = createReturningFailure(version = 1, encryptionKeyIndex = index)
            val rejected = assertThrows(
                "encryptionKeyIndex=$index must be rejected by the wrapper",
                IllegalArgumentException::class.java,
            ) { throw failure!! }
            assertTrue(
                "the message should name the offending argument, got: ${rejected.message}",
                rejected.message!!.contains("encryptionKeyIndex"),
            )
        }
    }

    /**
     * Omitting the index is valid and must reach native.
     *
     * This is the preferred path: the SDK allocates the index from Platform
     * state. It must not be mistaken for a missing argument.
     */
    @Test
    fun acceptsAnOmittedIndex() {
        val failure = createReturningFailure(version = 1, encryptionKeyIndex = null)
        assertFalse(
            "the omitted-index path must not be rejected as an argument error, got: $failure",
            failure is IllegalArgumentException,
        )
    }

    /**
     * Zero is an ordinary explicit index, not a stand-in for absence.
     *
     * Guards the boundary between the two representations: only `null` means
     * omitted, so `0` must pass through as a real index.
     */
    @Test
    fun acceptsZeroAsAnExplicitIndex() {
        val failure = createReturningFailure(version = 1, encryptionKeyIndex = 0)
        assertFalse(
            "an explicit zero index must not be rejected as an argument error, got: $failure",
            failure is IllegalArgumentException,
        )
    }
}
