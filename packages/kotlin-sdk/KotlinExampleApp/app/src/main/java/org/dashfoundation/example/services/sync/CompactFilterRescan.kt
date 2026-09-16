package org.dashfoundation.example.services.sync

import org.dashfoundation.dashsdk.errors.DashSdkError

data class CompactFilterRescanWallet(
    val walletIdHex: String,
    val walletId: ByteArray,
)

data class CompactFilterRescanFailure(
    val walletIdHex: String,
    val error: DashSdkError,
)

data class CompactFilterRescanResult(
    val fromHeight: Int,
    val acceptedWalletIds: List<String>,
    val failures: List<CompactFilterRescanFailure>,
)

/** Thin per-wallet fan-out; the native manager owns all rescan semantics. */
object CompactFilterRescan {
    suspend fun armAll(
        wallets: List<CompactFilterRescanWallet>,
        fromHeight: Int,
        arm: suspend (ByteArray, Int) -> Unit,
    ): CompactFilterRescanResult {
        require(fromHeight >= 0) { "fromHeight must be non-negative" }
        val accepted = ArrayList<String>()
        val failures = ArrayList<CompactFilterRescanFailure>()
        for (wallet in wallets) {
            try {
                arm(wallet.walletId, fromHeight)
                accepted += wallet.walletIdHex
            } catch (error: DashSdkError) {
                failures += CompactFilterRescanFailure(wallet.walletIdHex, error)
            }
        }
        return CompactFilterRescanResult(fromHeight, accepted, failures)
    }
}
