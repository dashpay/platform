package org.dashfoundation.dashsdk.identity

import kotlinx.coroutines.test.runTest
import org.junit.Assert.assertArrayEquals
import org.junit.Test

/**
 * Wrapper-seam tests for [IdentityUpdates.update]: the external key ids reach
 * the native call as given, a negative one included (the JNI marshaller
 * refuses it).
 */
class IdentityUpdatesTest {

    private val browserKey = IdentityPubkey(
        keyId = 3,
        keyType = KeyType.ECDSA_HASH160,
        purpose = KeyPurpose.AUTHENTICATION,
        securityLevel = SecurityLevel.HIGH,
        pubkeyBytes = ByteArray(20) { 7 },
    )

    @Test
    fun `update forwards the external key ids to the native call`() = runTest {
        var capturedExternal: IntArray? = null
        var capturedDisable: IntArray? = null
        val updates = IdentityUpdates(
            updateNative = UpdateIdentityNativeCall { _, _, _, disable, _, external ->
                capturedDisable = disable.copyOf()
                capturedExternal = external.copyOf()
            },
        )

        updates.update(
            walletHandle = 1,
            identityId = ByteArray(32) { 5 },
            addPublicKeys = listOf(browserKey),
            disablePublicKeyIds = listOf(1),
            signerHandle = 2,
            externalKeyIds = listOf(3),
        )

        assertArrayEquals(intArrayOf(3), capturedExternal)
        assertArrayEquals(intArrayOf(1), capturedDisable)
    }

    @Test
    fun `update passes no external key ids by default`() = runTest {
        var capturedExternal: IntArray? = null
        val updates = IdentityUpdates(
            updateNative = UpdateIdentityNativeCall { _, _, _, _, _, external ->
                capturedExternal = external.copyOf()
            },
        )

        updates.addKey(
            walletHandle = 1,
            identityId = ByteArray(32) { 5 },
            key = browserKey,
            signerHandle = 2,
        )

        assertArrayEquals(intArrayOf(), capturedExternal)
    }

    @Test
    fun `update leaves a negative external key id to the native marshaller`() = runTest {
        var capturedExternal: IntArray? = null
        val updates = IdentityUpdates(
            updateNative = UpdateIdentityNativeCall { _, _, _, _, _, external ->
                capturedExternal = external.copyOf()
            },
        )

        updates.update(
            walletHandle = 1,
            identityId = ByteArray(32) { 5 },
            addPublicKeys = listOf(browserKey),
            signerHandle = 2,
            externalKeyIds = listOf(-1),
        )

        // The JNI marshaller refuses it (`key_ids_from_ints`), mapped to an SDK error.
        assertArrayEquals(intArrayOf(-1), capturedExternal)
    }
}
