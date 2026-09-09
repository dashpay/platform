package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.ContestedDpnsNamesNativeResult
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class ContestedDpnsNamesTest {

    @Test
    fun `one shared sync returns complete cached snapshot beyond old eight label cap`() = runTest {
        val expected = Array(12) { "contested-$it" }
        var syncCalls = 0
        var cacheCalls = 0
        val destroyed = mutableListOf<Long>()
        val registration = IdentityRegistration(
            syncContestedDpnsNative = SyncContestedDpnsNativeCall { _, _ ->
                syncCalls += 1
                expected.size
            },
            managedIdentityLookupNative = ManagedIdentityLookupNativeCall { _, _ -> 44L },
            cachedContestedDpnsNative = CachedContestedDpnsNativeCall {
                cacheCalls += 1
                ContestedDpnsNamesNativeResult(expected)
            },
            destroyManagedIdentity = destroyed::add,
        )

        val snapshot = registration.contestedDpnsNames(1L, ByteArray(32))

        assertEquals(expected.toList(), snapshot.labels)
        assertEquals(12, snapshot.refreshedCount)
        assertEquals(1, syncCalls)
        assertEquals(1, cacheCalls)
        assertEquals(listOf(44L), destroyed)
    }

    @Test
    fun `managed identity handle is freed when cached array copy fails`() = runTest {
        val destroyed = mutableListOf<Long>()
        val registration = IdentityRegistration(
            syncContestedDpnsNative = SyncContestedDpnsNativeCall { _, _ -> 1 },
            managedIdentityLookupNative = ManagedIdentityLookupNativeCall { _, _ -> 55L },
            cachedContestedDpnsNative = CachedContestedDpnsNativeCall {
                error("cached read failed")
            },
            destroyManagedIdentity = destroyed::add,
        )

        assertTrue(runCatching { registration.contestedDpnsNames(1, ByteArray(32)) }.isFailure)
        assertEquals(listOf(55L), destroyed)
    }

    @Test
    fun `identity absent from wallet is typed not found`() = runTest {
        val registration = IdentityRegistration(
            syncContestedDpnsNative = SyncContestedDpnsNativeCall { _, _ -> 0 },
            managedIdentityLookupNative = ManagedIdentityLookupNativeCall { _, _ -> 0L },
        )

        val error = runCatching { registration.contestedDpnsNames(1, ByteArray(32)) }
            .exceptionOrNull()
        assertTrue(error is DashSdkError.NotFound)
    }
}
