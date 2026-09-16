package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.runBlocking
import org.dashfoundation.dashsdk.Network
import org.dashfoundation.dashsdk.Sdk
import org.dashfoundation.dashsdk.config.SdkConfig
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.persistence.DashDatabase
import org.dashfoundation.dashsdk.security.WalletStorage
import org.junit.After
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Consumer-surface coverage for the MAYACHAIN builder controls: drives ONLY
 * the public [ManagedPlatformWallet.buildSignedPayment] overload (the API an
 * app consuming the published `dash-sdk-android` artifact can reach), not the
 * internal [CoreTransactionBuilder] / `WalletManagerNative` surface that
 * [CoreTxBuilderOpReturnBindingTest] pins.
 *
 * Runs offline against an UNFUNDED wallet, so the deepest reachable outcome
 * is key-wallet's atomic selection failing with
 * [DashSdkError.PlatformWallet.CoreInsufficientFunds] — which is exactly the
 * point: reaching that error through the option-carrying call proves the
 * memo/order/change options were accepted and threaded into the build (an
 * option failure surfaces earlier as a different error), while an oversize
 * memo must fail BEFORE selection with something other than
 * insufficient-funds and leave the wallet usable. The full funded
 * deposit-shape assertion (vault VOUT0 / memo VOUT1 / change VOUT2) stays
 * with the gated Swift `MayaDepositVerificationIntegrationTests` and the
 * wallet-side testnet verification.
 */
@RunWith(AndroidJUnit4::class)
class BuildSignedPaymentMayaOptionsTest {

    // BIP39 English test vector (all-zero entropy) — same as
    // WalletManagerRoundTripTest; nothing is funded or broadcast.
    private val testMnemonic =
        "abandon abandon abandon abandon abandon abandon abandon abandon " +
            "abandon abandon abandon about"

    // Syntactically valid testnet P2PKH standing in for a Maya vault; the
    // builder validates encoding/network only.
    private val vaultAddress = "yMqShkrgjTRuReBGFpQr7FozEF1QcNBBYA"

    private val mayaMemo =
        "=:ETH.ETH:0x1c7b17362c84287bd1184447e6dfeaf920c31bbe".toByteArray(Charsets.UTF_8)

    private lateinit var db: DashDatabase
    private lateinit var walletStorage: WalletStorage
    private lateinit var sdk: Sdk

    @Before
    fun setUp() = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        db = DashDatabase.createInMemory(context)
        walletStorage = WalletStorage(context)
        // Testnet, no overrides → offline client build (no connection made).
        sdk = Sdk.create(SdkConfig(network = Network.TESTNET))
    }

    @After
    fun tearDown() {
        runCatching { db.close() }
        runCatching { sdk.close() }
    }

    private fun withUnfundedWallet(
        block: suspend (ManagedPlatformWallet, Long) -> Unit,
    ) = runBlocking {
        PlatformWalletManager(sdk, Network.TESTNET, db, walletStorage).use { manager ->
            val created = manager.createWallet(
                mnemonic = testMnemonic,
                name = "maya-options",
                createDefaultAccounts = true,
            )
            val wallet = manager.wallet(forWalletId = created.walletId)
            assertNotNull("created wallet is addressable", wallet)
            block(wallet!!, manager.mnemonicResolverHandle)
        }
    }

    @Test
    fun mayaOptionsThreadThroughThePublicBuildSignedPayment() = withUnfundedWallet { wallet, signer ->
        // The canonical Maya sequence through the public API. On an unfunded
        // wallet the first possible failure point past option application is
        // atomic selection — so insufficient-funds here means the memo and
        // both shape flags were accepted and threaded into the build.
        val error = runCatching {
            wallet.buildSignedPayment(
                recipients = listOf(vaultAddress to 100_000L),
                network = Network.TESTNET,
                coreSignerHandle = signer,
                opReturnData = mayaMemo,
                preserveOutputOrder = true,
                changeToFirstInput = true,
            )
        }.exceptionOrNull()

        assertTrue(
            "unfunded Maya-shaped build must fail at selection, got: $error",
            error is DashSdkError.PlatformWallet.CoreInsufficientFunds,
        )
    }

    @Test
    fun oversizeMemoFailsBeforeSelectionAndWalletSurvives() = withUnfundedWallet { wallet, signer ->
        val error = runCatching {
            wallet.buildSignedPayment(
                recipients = listOf(vaultAddress to 100_000L),
                network = Network.TESTNET,
                coreSignerHandle = signer,
                opReturnData = ByteArray(81),
                preserveOutputOrder = true,
                changeToFirstInput = true,
            )
        }.exceptionOrNull()

        assertNotNull("81-byte memo must be rejected", error)
        assertFalse(
            "oversize memo must fail before selection, got: $error",
            error is DashSdkError.PlatformWallet.CoreInsufficientFunds,
        )

        // The rejection happened before anything was reserved; the same
        // wallet must still drive a well-formed build to the selection stage.
        val retry = runCatching {
            wallet.buildSignedPayment(
                recipients = listOf(vaultAddress to 100_000L),
                network = Network.TESTNET,
                coreSignerHandle = signer,
                opReturnData = mayaMemo,
                preserveOutputOrder = true,
                changeToFirstInput = true,
            )
        }.exceptionOrNull()

        assertTrue(
            "wallet survives the rejected memo, got: $retry",
            retry is DashSdkError.PlatformWallet.CoreInsufficientFunds,
        )
    }
}
