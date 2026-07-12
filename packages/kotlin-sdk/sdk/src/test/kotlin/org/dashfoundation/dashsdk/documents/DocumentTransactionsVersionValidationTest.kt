package org.dashfoundation.dashsdk.documents

import kotlinx.coroutines.runBlocking
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * Version-byte validation for [DocumentTransactions.createEncryptedDocument]
 * (dashpay/platform#4091). Only 0 (CBOR) and 1 (protobuf) are wire-meaningful —
 * `seal_tx_metadata` writes the byte verbatim and the legacy dashj
 * `decryptTxMetadata` switches on exactly those two values, so an out-of-range
 * byte would silently seal a document the legacy stack can't decode.
 *
 * The `require` runs before any native call (`TransactionsNative`), so the
 * REJECTION paths are exercised on the JVM without the JNI library loaded. The
 * accepted values 0/1 would proceed into native and can't be unit-tested here.
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
            encryptionKeyIndex = 0,
            version = version,
            payload = payload,
            signerHandle = 0L,
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
}
