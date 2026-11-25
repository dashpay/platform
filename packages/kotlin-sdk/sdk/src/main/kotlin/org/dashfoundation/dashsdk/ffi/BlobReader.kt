package org.dashfoundation.dashsdk.ffi

import org.dashfoundation.dashsdk.errors.DashSdkError
import java.nio.BufferUnderflowException
import java.nio.ByteBuffer

/**
 * Reads a big-endian BLOB a JNI export wrote, refusing anything malformed with
 * [DashSdkError.SerializationError]: a truncated BLOB, a length past what is left (so a
 * corrupt length never allocates), and bytes left over after [finish]. `bytes` is a `u32`
 * length and that many bytes, `str` the same as UTF-8.
 */
internal class BlobReader(blob: ByteArray, private val what: String) {
    private val buffer = ByteBuffer.wrap(blob)

    fun u8(): Int = guard { buffer.get().toInt() and 0xFF }

    fun u32(): Long = guard { buffer.int.toLong() and 0xFFFF_FFFFL }

    /** A `u64` that fits a [Long]; Platform credits and heights always do. */
    fun u64(): Long = guard {
        val value = buffer.long
        if (value < 0) throw DashSdkError.SerializationError("$what: u64 $value does not fit a Long")
        value
    }

    /** A `u32` count, bounded by the bytes left so a corrupt count cannot allocate. */
    fun count(): Int {
        val count = u32()
        if (count > buffer.remaining()) {
            throw DashSdkError.SerializationError("$what: count $count exceeds the ${buffer.remaining()} bytes left")
        }
        return count.toInt()
    }

    fun fixed(length: Int): ByteArray = guard { ByteArray(length).also { buffer.get(it) } }

    fun bytes(): ByteArray {
        val length = u32()
        if (length > buffer.remaining()) {
            throw DashSdkError.SerializationError("$what: length $length exceeds the ${buffer.remaining()} bytes left")
        }
        return fixed(length.toInt())
    }

    fun str(): String = String(bytes(), Charsets.UTF_8)

    /** Refuse bytes left after the last field. */
    fun finish() {
        if (buffer.hasRemaining()) {
            throw DashSdkError.SerializationError("$what: ${buffer.remaining()} bytes left over")
        }
    }

    private inline fun <T> guard(read: () -> T): T = try {
        read()
    } catch (e: BufferUnderflowException) {
        throw DashSdkError.SerializationError("$what: truncated", e)
    }
}
