package org.dashfoundation.dashsdk.documents

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Input validation for [DocumentTransactions.createEncryptedDocument].
 *
 * Two independent guards are exercised here, both of which run BEFORE any
 * native call (`TransactionsNative`), so the REJECTION paths are testable on the
 * JVM without the JNI library loaded:
 *
 * 1. Version byte (dashpay/platform#4091): only 0 (CBOR) and 1 (protobuf) are
 *    wire-meaningful — `seal_tx_metadata` writes the byte verbatim and the
 *    legacy dashj `decryptTxMetadata` switches on exactly those two values, so
 *    an out-of-range byte would silently seal a document the legacy stack can't
 *    decode.
 * 2. `encryptionKeyIndex` (dashpay/platform#4186 follow-up): `null` is the
 *    preferred path (Rust allocates the index from Platform state); an explicit
 *    value, when supplied, must be non-negative.
 *
 * Paths that PASS validation proceed into native and can't be fully unit-tested
 * here (no JNI library); [noIndexPathPassesValidation] asserts only that the
 * `null` index is accepted by the guard, not rejected as an argument error.
 */
class DocumentTransactionsVersionValidationTest {

    private val id32 = ByteArray(32)
    private val payload = ByteArray(4) { it.toByte() }

    private fun createWithVersion(version: Int) = runBlocking {
        DocumentTransactions().createEncryptedDocument(
            walletHandle = 0L,
            mnemonicResolverHandle = 0L,
            ownerId = id32,
            contractId = id32,
            documentType = "txMetadata",
            version = version,
            payload = payload,
            signerHandle = 0L,
            encryptionKeyIndex = 0,
        )
    }

    /** Bytes 2..255 (previously accepted by the `0..255` range) are now rejected. */
    @Test
    fun rejectsVersionBytesTheLegacyStackCannotDecode() {
        for (version in intArrayOf(2, 3, 127, 255)) {
            val e = assertThrows(
                "version=$version must be rejected",
                IllegalArgumentException::class.java,
            ) { createWithVersion(version) }
            assertTrue(
                "message should name the wire-meaningful versions, got: ${e.message}",
                e.message!!.contains("0 (CBOR) or 1 (protobuf)"),
            )
        }
    }

    /** A negative version byte is likewise rejected. */
    @Test
    fun rejectsNegativeVersion() {
        assertThrows(IllegalArgumentException::class.java) { createWithVersion(-1) }
    }

    /**
     * An explicit NEGATIVE index (the migration/test-only path) is rejected by
     * the `require`. `null` (the allocate-in-Rust path) is the only way to omit
     * an index; a negative explicit value is a caller error.
     */
    @Test
    fun rejectsExplicitNegativeIndex() {
        val e = assertThrows(IllegalArgumentException::class.java) {
            runBlocking {
                DocumentTransactions().createEncryptedDocument(
                    walletHandle = 0L,
                    mnemonicResolverHandle = 0L,
                    ownerId = id32,
                    contractId = id32,
                    documentType = "txMetadata",
                    version = 1,
                    payload = payload,
                    signerHandle = 0L,
                    encryptionKeyIndex = -5,
                )
            }
        }
        assertTrue(
            "message should name encryptionKeyIndex, got: ${e.message}",
            e.message!!.contains("encryptionKeyIndex"),
        )
    }

    /**
     * The no-index path (`encryptionKeyIndex` omitted → `null`, the default and
     * preferred allocate-in-Rust route) must PASS the argument guards. With all
     * other inputs valid, the only failure that can surface is the native call
     * itself (no JNI library in a JVM unit test), NOT an
     * [IllegalArgumentException] from our `require`s — proving `null` is a valid
     * argument rather than a rejected one.
     */
    @Test
    fun noIndexPathPassesValidation() {
        val t = runCatching {
            runBlocking {
                DocumentTransactions().createEncryptedDocument(
                    walletHandle = 0L,
                    mnemonicResolverHandle = 0L,
                    ownerId = id32,
                    contractId = id32,
                    documentType = "txMetadata",
                    version = 1,
                    payload = payload,
                    signerHandle = 0L,
                    // encryptionKeyIndex omitted → null → allocate in Rust.
                )
            }
        }.exceptionOrNull()
        assertFalse(
            "the null-index path must not be rejected as an argument error, got: $t",
            t is IllegalArgumentException,
        )
    }
}
