package org.dashfoundation.dashsdk.wallet

import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertThrows
import org.junit.Test

class PlatformWalletManagerInitializationTest {

    @Test
    fun shouldReturnValidatedNativeStateWithoutRunningCleanup() {
        val cleanup = mutableListOf<String>()

        val initialized = initializePlatformWalletNativeManager(
            nativeCreate = { 11L },
            nativeManagerHandle = { bundle ->
                assertEquals(11L, bundle)
                22L
            },
            nativePersistenceCapabilitiesVersion = { 1 },
            nativePersistenceCapabilitiesBits = { 0xbdL },
            nativeDestroy = { cleanup += "destroy" },
            cancelScope = { cleanup += "scope" },
            closeMnemonicResolver = { cleanup += "resolver" },
            closeSigner = { cleanup += "signer" },
            closePersistenceHandler = { cleanup += "persistence" },
        )

        assertEquals(11L, initialized.bundle)
        assertEquals(22L, initialized.managerHandle)
        assertEquals(1, initialized.persistenceCapabilities.version)
        assertEquals(0xbdL, initialized.persistenceCapabilities.bits)
        assertEquals(emptyList<String>(), cleanup)
    }

    @Test
    fun shouldReleaseEveryHostResourceWhenNativeCreateThrowsOrReturnsZero() {
        listOf<() -> Long>(
            { throw TestInitializationException("create") },
            { 0L },
        ).forEach { create ->
            val cleanup = mutableListOf<String>()

            assertThrows(Throwable::class.java) {
                initializePlatformWalletNativeManager(
                    nativeCreate = create,
                    nativeManagerHandle = { error("must not run") },
                    nativePersistenceCapabilitiesVersion = { error("must not run") },
                    nativePersistenceCapabilitiesBits = { error("must not run") },
                    nativeDestroy = { cleanup += "destroy" },
                    cancelScope = { cleanup += "scope" },
                    closeMnemonicResolver = { cleanup += "resolver" },
                    closeSigner = { cleanup += "signer" },
                    closePersistenceHandler = { cleanup += "persistence" },
                )
            }

            assertEquals(listOf("scope", "resolver", "signer", "persistence"), cleanup)
        }
    }

    @Test
    fun shouldDestroyBundleExactlyOnceWhenLaterInitializationFailsOrReturnsZero() {
        listOf<(Long) -> Long>(
            { 0L },
            { throw TestInitializationException("manager") },
        ).forEach { managerHandle ->
            val cleanup = mutableListOf<String>()

            assertThrows(Throwable::class.java) {
                initializePlatformWalletNativeManager(
                    nativeCreate = { 11L },
                    nativeManagerHandle = managerHandle,
                    nativePersistenceCapabilitiesVersion = { error("must not run") },
                    nativePersistenceCapabilitiesBits = { error("must not run") },
                    nativeDestroy = { cleanup += "destroy:$it" },
                    cancelScope = { cleanup += "scope" },
                    closeMnemonicResolver = { cleanup += "resolver" },
                    closeSigner = { cleanup += "signer" },
                    closePersistenceHandler = { cleanup += "persistence" },
                )
            }

            assertEquals(
                listOf("scope", "destroy:11", "resolver", "signer", "persistence"),
                cleanup,
            )
        }
    }

    @Test
    fun shouldContinueCleanupAndSuppressCleanupFailuresAfterCapabilityQueryFailure() {
        val initializationFailure = TestInitializationException("capabilities")
        val destroyFailure = TestInitializationException("destroy")
        val cleanup = mutableListOf<String>()

        val thrown = assertThrows(TestInitializationException::class.java) {
            initializePlatformWalletNativeManager(
                nativeCreate = { 11L },
                nativeManagerHandle = { 22L },
                nativePersistenceCapabilitiesVersion = { 1 },
                nativePersistenceCapabilitiesBits = { throw initializationFailure },
                nativeDestroy = {
                    cleanup += "destroy"
                    throw destroyFailure
                },
                cancelScope = { cleanup += "scope" },
                closeMnemonicResolver = { cleanup += "resolver" },
                closeSigner = { cleanup += "signer" },
                closePersistenceHandler = { cleanup += "persistence" },
            )
        }

        assertSame(initializationFailure, thrown)
        assertEquals(listOf(destroyFailure), thrown.suppressed.toList())
        assertEquals(listOf("scope", "destroy", "resolver", "signer", "persistence"), cleanup)
    }

    private class TestInitializationException(message: String) : RuntimeException(message)
}
