package org.dashfoundation.dashsdk.identity

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.io.ByteArrayOutputStream
import java.io.DataOutputStream

class IdentityKeyPreviewTest {

    @Test
    fun `malformed preview blob is scrubbed after a partial decode failure`() {
        val complete = previewBlob(rowCount = 2)
        val blob = complete.copyOf(complete.size - 1)

        val failure = runCatching { IdentityKeyPreview.decodeAll(blob) }

        assertTrue(failure.isFailure)
        assertArrayEquals(
            "the JNI preview blob contains private scalars and must be wiped even when parsing fails",
            ByteArray(blob.size),
            blob,
        )
    }

    private fun previewBlob(rowCount: Int): ByteArray {
        val out = ByteArrayOutputStream()
        DataOutputStream(out).use { dos ->
            dos.writeInt(rowCount)
            repeat(rowCount) { keyId ->
                dos.writeInt(7)
                val path = "m/9'/7'/$keyId'".toByteArray()
                dos.writeShort(path.size)
                dos.write(path)
                dos.write(ByteArray(33) { (keyId + 2).toByte() })
                dos.write(ByteArray(32) { (keyId + 1).toByte() })
            }
        }
        return out.toByteArray()
    }
}
