package org.dashfoundation.dashsdk.wallet

import androidx.test.ext.junit.runners.AndroidJUnit4
import org.dashfoundation.dashsdk.ffi.DashSDKException
import org.dashfoundation.dashsdk.ffi.NativeLoader
import org.dashfoundation.dashsdk.ffi.WalletManagerNative
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertThrows
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Binding-level coverage for the MAYACHAIN-deposit builder controls
 * (`add_op_return`, `preserve_output_order`, `change_to_first_input`,
 * `signed_transaction_bytes`) — the Android counterpart of the gated
 * Swift `MayaDepositVerificationIntegrationTests`, minus everything that
 * needs a funded wallet. Proves the four new JNI symbols resolve, happy-path
 * calls succeed against a live builder, and the FFI's error paths surface as
 * [DashSDKException] instead of aborting.
 *
 * No network, no wallet, no funds: a builder handle alone accepts outputs
 * and options; only funding/finalizing needs a wallet. The full
 * deposit-shape assertion (vault VOUT0 / memo VOUT1 / change VOUT2 on a
 * really-funded transaction) stays with the Swift integration suite and the
 * wallet-side testnet verification.
 */
@RunWith(AndroidJUnit4::class)
class CoreTxBuilderOpReturnBindingTest {

    // Any syntactically valid testnet P2PKH address works — the builder
    // validates encoding/network only; nothing is funded or sent. Same
    // address the FFI's own persistence tests use.
    private val testnetAddress = "yMqShkrgjTRuReBGFpQr7FozEF1QcNBBYA"

    private fun withBuilder(block: (Long) -> Unit) {
        NativeLoader.ensureLoaded()
        val builder = WalletManagerNative.coreTxBuilderNew(network = 1)
        assertNotEquals("builder handle must be live", 0L, builder)
        try {
            block(builder)
        } finally {
            WalletManagerNative.coreTxBuilderDestroy(builder)
        }
    }

    @Test
    fun mayaShapeOptionsBindAndAccept() {
        withBuilder { builder ->
            // The canonical Maya deposit sequence, sans funding: vault output,
            // memo, insertion-order + VIN0-change options.
            WalletManagerNative.coreTxBuilderAddOutput(builder, vaultAddressForTest(), 100_000)
            WalletManagerNative.coreTxBuilderAddOpReturn(
                builder,
                "=:ETH.ETH:0x1c7b17362c84287bd1184447e6dfeaf920c31bbe".toByteArray(Charsets.UTF_8),
            )
            WalletManagerNative.coreTxBuilderPreserveOutputOrder(builder)
            WalletManagerNative.coreTxBuilderChangeToFirstInput(builder)
        }
    }

    @Test
    fun opReturnAcceptsExactly80Bytes() {
        withBuilder { builder ->
            WalletManagerNative.coreTxBuilderAddOpReturn(builder, ByteArray(80))
        }
    }

    @Test
    fun opReturnRejects81BytesAndBuilderSurvives() {
        withBuilder { builder ->
            assertThrows(DashSDKException::class.java) {
                WalletManagerNative.coreTxBuilderAddOpReturn(builder, ByteArray(81))
            }
            // The FFI rejects the payload BEFORE consuming builder state, so
            // the same handle must still accept further configuration.
            WalletManagerNative.coreTxBuilderAddOutput(builder, vaultAddressForTest(), 100_000)
        }
    }

    @Test
    fun signedTransactionBytesSymbolBindsAndRejectsNullHandle() {
        NativeLoader.ensureLoaded()
        // Handle 0 can never be a finalized transaction; the call must throw
        // (not crash), which also proves the JNI symbol resolves.
        assertThrows(DashSDKException::class.java) {
            WalletManagerNative.coreSignedTransactionBytes(0L)
        }
    }

    private fun vaultAddressForTest(): String = testnetAddress
}
