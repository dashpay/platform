package org.dashfoundation.dashsdk.voting

import org.junit.Assert.assertEquals
import org.junit.Test

/**
 * Encoding of contested-resource index values into the strings rs-sdk-ffi's
 * contested-resource calls read: a leading backslash marks text, else `0x`
 * marks hex bytes, else the value is text. Pure JVM: no native library is
 * loaded.
 */
class ContestedIndexValueTest {

    @Test
    fun shouldPrefixABackslashToTextStartingWith0x() {
        // `oxford` normalizes to `0xf0rd`; sent raw, the FFI would read it as hex.
        assertEquals("\\0xf0rd", ContestedIndexValue.text("0xf0rd"))
        assertEquals("\\0x", ContestedIndexValue.text("0x"))
    }

    @Test
    fun shouldPrefixABackslashToTextStartingWithABackslash() {
        assertEquals("\\\\a11ce", ContestedIndexValue.text("\\a11ce"))
        assertEquals("\\\\0xf0rd", ContestedIndexValue.text("\\0xf0rd"))
    }

    @Test
    fun shouldLeaveOtherTextUnchanged() {
        assertEquals("a11ce", ContestedIndexValue.text("a11ce"))
        assertEquals("dash", ContestedIndexValue.text("dash"))
        assertEquals("", ContestedIndexValue.text(""))
        // Only the exact lowercase `0x` prefix is hex to the FFI.
        assertEquals("0Xf0rd", ContestedIndexValue.text("0Xf0rd"))
        assertEquals("a\\b", ContestedIndexValue.text("a\\b"))
    }

    @Test
    fun shouldEncodeBytesAsLowercaseHexWith0xPrefix() {
        assertEquals(
            "0xface0001",
            ContestedIndexValue.bytes(byteArrayOf(0xFA.toByte(), 0xCE.toByte(), 0x00, 0x01)),
        )
        assertEquals("0x", ContestedIndexValue.bytes(ByteArray(0)))
    }
}
