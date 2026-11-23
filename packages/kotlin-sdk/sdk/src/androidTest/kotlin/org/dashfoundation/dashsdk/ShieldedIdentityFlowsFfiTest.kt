package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.funding.ShieldedProver
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assert.fail
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * The JNI surface KotlinExampleApp uses to shield from an identity's
 * balance (type 21), list and stop its durable records, and top up an
 * identity from the shielded pool (type 22), against the real library: the
 * record listing marshals through a real manager, and a shield from an
 * identity the wallet does not manage is refused locally (before any network
 * call) with no record saved. `FfiSmokeTest` checks that the four exports
 * bind and refuse arguments they cannot read.
 *
 * Not covered here: a shield or top-up Platform accepts, which needs a
 * funded identity or shielded balance (TEST_PLAN SH-21 to SH-23), and a
 * top-up's refusals, which come after the wallet asks the network whether
 * the transition is active.
 */
@RunWith(AndroidJUnit4::class)
class ShieldedIdentityFlowsFfiTest {

    private val identityId = ByteArray(32) { 2 }
    private val activityId = ByteArray(32) { 3 }

    // BIP39 English test vector (all-zero entropy).
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    @Before
    fun setUp() {
        NativeLoader.ensureLoaded()
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
    }

    /**
     * Through a real manager and a real (unfunded) wallet: both fee kinds
     * the screens show are positive, the wallet has no records to list, an
     * unknown record cannot be stopped, and a shield from an identity the
     * wallet does not manage is refused as not found, without leaving a
     * record or an activity row.
     */
    @Test
    fun shouldListNoRecordsAndRefuseAnUnmanagedIdentityWithoutARecord() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val db = DashDatabase.createInMemory(context)
        val sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
        val storeDir = File(context.cacheDir, "identity-shield-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, WalletStorage(context)).use { manager ->
                manager.configureShielded(File(storeDir, "shielded.sqlite").absolutePath)
                val id = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "identity-shield",
                    createDefaultAccounts = true,
                ).walletId
                manager.bindShielded(id, listOf(0))

                for (kind in listOf(
                    ShieldedProver.FeeKind.ShieldFromIdentity,
                    ShieldedProver.FeeKind.IdentityTopUpFromPool,
                )) {
                    val fee = manager.estimateShieldedFee(kind)
                    assertTrue("the $kind fee is a positive number of credits: $fee", fee > 0)
                }

                assertEquals(emptyList<Any>(), manager.shieldedIdentityDebitRecoveryRecords(id))

                try {
                    manager.abandonShieldedIdentityDebit(id, 0u, activityId, true)
                    fail("an unknown record cannot be stopped")
                } catch (_: DashSdkError) {
                }

                try {
                    manager.shieldedShieldFromIdentity(id, identityId, 1_000L)
                    fail("a shield from an identity the wallet does not manage must be refused")
                } catch (e: DashSdkError) {
                    assertTrue("refused as an identity the wallet does not hold: $e", e is DashSdkError.NotFound)
                }
                assertEquals(
                    "the refused shield saved no record",
                    emptyList<Any>(),
                    manager.shieldedIdentityDebitRecoveryRecords(id),
                )
                assertEquals(
                    "nothing was recorded for the refused requests",
                    0,
                    db.shieldedDao().getAllActivity().size,
                )
            }
        } finally {
            storeDir.deleteRecursively()
            runCatching { db.close() }
            runCatching { sdk.close() }
        }
    }
}
