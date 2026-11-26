package org.dashfoundation.dashsdk.wallet

import java.io.File
import org.dashfoundation.dashsdk.ffi.ShieldedTokenPoolBalanceData
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * The JNI transport rows of the token shielded pool calls become the public
 * types unchanged: unsigned values read back from their bit patterns, and a
 * cleared `has*` flag becomes null rather than the zero the native row holds.
 */
class ShieldedTokenPoolBalanceTest {

    private fun row(scanned: Boolean, lastError: String? = null) = ShieldedTokenPoolBalanceData(
        tokenId = ByteArray(32) { 3 },
        dataContractId = ByteArray(32) { 4 },
        tokenContractPosition = 65_535,
        accountIndex = 2,
        balance = -1L,
        unspentNoteCount = -1,
        reservedBalance = 250,
        hasScannedNoteCount = scanned,
        scannedNoteCount = if (scanned) 2_048 else 0,
        hasPoolNoteCount = scanned,
        poolNoteCount = if (scanned) 4_096 else 0,
        hasScannedHeight = scanned,
        scannedHeight = if (scanned) 123_456 else 0,
        lastError = lastError,
    )

    @Test
    fun shouldReadEveryUnsignedValueFromItsBitPattern() {
        val balance = ShieldedTokenPoolBalance.fromNative(row(scanned = true, lastError = "chunk read failed"))

        assertArrayEquals(ByteArray(32) { 3 }, balance.tokenId)
        assertArrayEquals(ByteArray(32) { 4 }, balance.dataContractId)
        assertEquals(65_535, balance.tokenContractPosition)
        assertEquals(2, balance.accountIndex)
        assertEquals(ULong.MAX_VALUE, balance.balance)
        assertEquals(UInt.MAX_VALUE, balance.unspentNoteCount)
        assertEquals(250uL, balance.reservedBalance)
        assertEquals(2_048uL, balance.scannedNoteCount)
        assertEquals(4_096uL, balance.poolNoteCount)
        assertEquals(123_456uL, balance.scannedHeight)
        assertEquals("chunk read failed", balance.lastError)
    }

    @Test
    fun shouldReadAClearedFlagAsNullNotZero() {
        val balance = ShieldedTokenPoolBalance.fromNative(row(scanned = false))

        assertNull(balance.scannedNoteCount)
        assertNull(balance.poolNoteCount)
        assertNull(balance.scannedHeight)
        assertNull(balance.lastError)
    }

    @Test
    fun shouldCompareRowsByTheContentOfTheirIds() {
        val first = ShieldedTokenPoolBalance.fromNative(row(scanned = true))
        val second = ShieldedTokenPoolBalance.fromNative(row(scanned = true))

        assertEquals(first, second)
        assertEquals(first.hashCode(), second.hashCode())
        assertNotEquals(first, first.copy(tokenId = ByteArray(32) { 9 }))
    }

    @Test
    fun shouldReadThePreviewFeeAndNoteCounts() {
        val preview = ShieldedTokenSpendPreview.fromNative(longArrayOf(-1L, 2L, 4_294_967_295L))

        assertEquals(ULong.MAX_VALUE, preview.fee)
        assertEquals(2u, preview.tokenNotes)
        assertEquals(UInt.MAX_VALUE, preview.feeNotes)
    }

    @Test(expected = IllegalStateException::class)
    fun shouldRefuseAPreviewOfTheWrongShape() {
        ShieldedTokenSpendPreview.fromNative(longArrayOf(1L, 2L))
    }

    /**
     * The JNI library builds each row with a hand-written constructor
     * descriptor (`BALANCE_ROW_CTOR` in
     * `rs-unified-sdk-jni/src/shielded_token_pools.rs`). JVM tests never load
     * that library and the instrumented tests have no watched pool to list,
     * so a mismatch would first show as a crash on a device: derive the
     * descriptor from the Kotlin class and compare.
     */
    @Test
    fun shouldMatchTheConstructorDescriptorTheJniLibraryUses() {
        val constructor = ShieldedTokenPoolBalanceData::class.java.declaredConstructors.single()
        val descriptor = constructor.parameterTypes.joinToString("", "(", ")V") { jniType(it) }

        val rust = locateJniSource("shielded_token_pools.rs").readText()
        val declared = Regex("""const BALANCE_ROW_CTOR: &str = "([^"]+)";""").find(rust)
            ?.groupValues?.get(1)
        assertEquals(descriptor, declared)
        assertTrue(
            rust.contains("org/dashfoundation/dashsdk/ffi/ShieldedTokenPoolBalanceData"),
        )
    }

    private fun jniType(type: Class<*>): String = when (type) {
        Int::class.javaPrimitiveType -> "I"
        Long::class.javaPrimitiveType -> "J"
        Boolean::class.javaPrimitiveType -> "Z"
        Byte::class.javaPrimitiveType -> "B"
        ByteArray::class.java -> "[B"
        else -> "L" + type.name.replace('.', '/') + ";"
    }

    private fun locateJniSource(name: String): File {
        var dir: File? = File(System.getProperty("user.dir") ?: ".").absoluteFile
        while (dir != null) {
            for (candidate in listOf(
                File(dir, "rs-unified-sdk-jni/src/$name"),
                File(dir, "packages/rs-unified-sdk-jni/src/$name"),
            )) {
                if (candidate.isFile) return candidate
            }
            dir = dir.parentFile
        }
        error("could not locate rs-unified-sdk-jni/src/$name from ${System.getProperty("user.dir")}")
    }

    @Test
    fun shouldSendTheSpendKindBytesTheFfiNames() {
        // `platform_wallet_manager_shielded_token_spend_preview`: 0 shielded
        // transfer (type 26), 1 unshield (type 27).
        assertEquals(0.toByte(), ShieldedTokenPoolSpendKind.ShieldedTransfer.raw)
        assertEquals(1.toByte(), ShieldedTokenPoolSpendKind.Unshield.raw)
    }
}
