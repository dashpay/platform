package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The quorum source and mode an SDK reports through `SdkNative`, and the SPV
 * slot a wallet manager claims on it — the Kotlin side of
 * `SDKQuorumSourceTests.swift`. Regtest, so the SDKs only build against
 * loopback endpoints: the trusted provider's quorum prefetch fails in the
 * background without failing the build, and nothing connects.
 */
@RunWith(AndroidJUnit4::class)
class QuorumSourceFfiTest {

    private val regtest = SdkConfig(network = Network.REGTEST)

    /**
     * `Sdk.create` builds the adaptive provider in Auto: with no wallet
     * manager serving it SPV it asks the trusted service, and it takes the
     * mode changes a trusted SDK has none of. Each mode's source reads back
     * through the JNI exports, so their descriptors and the mode numbering
     * are exercised both ways.
     */
    @Test
    fun defaultSdkIsAdaptiveInAutoAndSwitchesModes() = runBlocking {
        val sdk = Sdk.create(regtest)
        try {
            assertEquals(Sdk.QuorumSource.TRUSTED, sdk.quorumSource())
            sdk.setQuorumMode(Sdk.QuorumMode.SPV)
            assertEquals(Sdk.QuorumSource.SPV, sdk.quorumSource())
            sdk.setQuorumMode(Sdk.QuorumMode.TRUSTED)
            assertEquals(Sdk.QuorumSource.TRUSTED, sdk.quorumSource())
            sdk.setQuorumMode(Sdk.QuorumMode.AUTO)
            assertEquals(
                "Auto without SPV asks the trusted service",
                Sdk.QuorumSource.TRUSTED,
                sdk.quorumSource(),
            )
        } finally {
            sdk.closeSuspending()
        }
    }

    /** Opting out builds the trusted provider, which has one source and no mode. */
    @Test
    fun trustedSdkRefusesAModeChange() = runBlocking {
        val sdk = Sdk.create(regtest, Sdk.ContextProviderKind.TRUSTED)
        try {
            assertEquals(Sdk.QuorumSource.TRUSTED, sdk.quorumSource())
            val refusal = runCatching { sdk.setQuorumMode(Sdk.QuorumMode.SPV) }.exceptionOrNull()
            assertTrue("expected a native refusal, got $refusal", refusal is DashSdkError)
        } finally {
            sdk.closeSuspending()
        }
    }

    /**
     * A wallet manager serves its SDK the quorum keys of its own SPV client,
     * and only one manager can: a second on the same adaptive SDK is refused
     * while the first is open, or two SPV clients would answer for one SDK.
     */
    @Test
    fun secondManagerOnOneAdaptiveSdkIsRefused() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val walletStorage = WalletStorage(context)
        val sdk = Sdk.create(regtest)
        try {
            PlatformWalletManager(sdk, Network.REGTEST, db, walletStorage).use {
                val second = runCatching {
                    PlatformWalletManager(sdk, Network.REGTEST, db, walletStorage)
                }
                second.getOrNull()?.close()
                val refusal = second.exceptionOrNull()
                assertTrue(
                    "expected the SDK's SPV slot to be taken, got $refusal",
                    refusal?.message?.contains("another SPV client") == true,
                )
            }
        } finally {
            sdk.closeSuspending()
            db.close()
        }
    }
}
