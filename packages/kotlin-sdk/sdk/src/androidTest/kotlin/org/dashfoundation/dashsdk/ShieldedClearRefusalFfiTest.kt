package org.dashfoundation.dashsdk

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeoutOrNull
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.persistence.entities.ShieldedNoteEntity
import org.dashfoundation.dashsdk.security.WalletStorage
import org.dashfoundation.dashsdk.services.ShieldedService
import org.dashfoundation.dashsdk.wallet.PlatformWalletManager
import org.junit.After
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * A shielded Clear through [ShieldedService.clearLocalState], the call the
 * Clear button makes, while a bind holds the service's ledger: the Clear is
 * refused at once and does not run later either. JVM tests cannot reach
 * this, because a service is bound only through the native library.
 *
 * The Clear is refused before it asks Rust, and nothing starts the shielded
 * sync loop.
 */
@RunWith(AndroidJUnit4::class)
class ShieldedClearRefusalFfiTest {

    // BIP39 English test vector (all-zero entropy).
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    /** A refusal takes no wait; the bound only ends a Clear that waits. */
    private val refusalWaitMs = 10_000L

    private lateinit var db: DashDatabase
    private lateinit var walletStorage: WalletStorage
    private lateinit var sdk: Sdk

    @Before
    fun setUp() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        db = DashDatabase.createInMemory(context)
        walletStorage = WalletStorage(context)
        // Testnet with no overrides, as in WalletManagerRoundTripTest.
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
    }

    @After
    fun tearDown() {
        runCatching { db.close() }
        runCatching { sdk.close() }
    }

    @Test
    fun clearIsRefusedWhileABindHoldsTheLedgerAndDoesNotRunAfterIt() = runBlocking {
        assumeTrue("the native library has no shielded support", Sdk.hasShielded())
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val storeDir = File(context.cacheDir, "shielded-clear-refusal-${System.nanoTime()}")
        assertTrue(storeDir.mkdirs())
        val storePath = File(storeDir, "shielded.sqlite").absolutePath
        val service = ShieldedService(db)
        try {
            PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
                manager.configureShielded(storePath)
                val walletId = manager.createWallet(
                    mnemonic = testMnemonic,
                    name = "shielded-clear-refusal",
                    createDefaultAccounts = true,
                ).walletId
                service.bind(manager, walletId, storePath)
                assertTrue("the wallet is bound", service.isBound.value)
                // A row the Clear's wipe would delete.
                db.shieldedDao().upsertNote(
                    ShieldedNoteEntity(
                        nullifier = ByteArray(32) { 1 },
                        walletId = walletId,
                        accountIndex = 0,
                        position = 1,
                        cmx = ByteArray(32) { 2 },
                        blockHeight = 100,
                        isSpent = false,
                        value = 1_000,
                        noteData = ByteArray(115),
                    ),
                )

                // Stands in for a bind inside its native calls: it holds
                // the ledger the way one does.
                val bindRunning = CompletableDeferred<Unit>()
                val bindMayFinish = CompletableDeferred<Unit>()
                val bind = launch {
                    service.withLedgerLifecycle {
                        bindRunning.complete(Unit)
                        bindMayFinish.await()
                    }
                }
                bindRunning.await()

                // The Clear button's call returns the typed refusal while
                // the bind is still running. A Clear that queued behind the
                // bind would park here, so the wait is bounded: it then ends
                // with no refusal and the assertion below fails. The bind is
                // released either way.
                val refused = try {
                    withTimeoutOrNull(refusalWaitMs) {
                        try {
                            service.clearLocalState()
                            null
                        } catch (e: DashSdkError) {
                            e
                        }
                    }
                } finally {
                    bindMayFinish.complete(Unit)
                }
                assertTrue(
                    "expected the typed refusal while the bind runs, got $refused " +
                        "(null: the Clear went through, or waited for the bind)",
                    refused is DashSdkError.PlatformWallet.ShieldedOperationInProgress,
                )

                // The bind finishing does not run the refused Clear: the row
                // is still there and the service is still bound.
                bind.join()
                assertEquals(1, db.shieldedDao().getAllNotes().size)
                assertTrue("nothing was reset", service.isBound.value)
            }
        } finally {
            service.close()
            storeDir.deleteRecursively()
        }
    }
}
