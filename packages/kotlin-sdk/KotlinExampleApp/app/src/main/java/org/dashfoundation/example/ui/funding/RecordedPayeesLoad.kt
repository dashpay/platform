package org.dashfoundation.example.ui.funding

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.wallet.ManagedPlatformWallet

/**
 * Where the read of the payees an external asset lock recorded stands, kept
 * apart so a failed read is shown neither as a lock without recipients nor as
 * a load that never ends. "Complete External Payment" is enabled only in
 * [Loaded]. ← Swift `RecordedPayeesLoad`.
 */
internal sealed interface RecordedPayeesLoad {
    /** The wallet is not loaded in the manager yet; the read starts when it is. */
    data object WaitingForWallet : RecordedPayeesLoad

    data object Loading : RecordedPayeesLoad

    data class Loaded(val payees: List<ManagedPlatformWallet.FundRecipient>) : RecordedPayeesLoad

    /** The read succeeded and the lock records no recipients. */
    data object NotFound : RecordedPayeesLoad

    /** The read failed; the form offers a retry. */
    data class Failed(val message: String) : RecordedPayeesLoad

    /** The payees once loaded, null in every other state: submission is enabled only with them. */
    val loadedPayees: List<ManagedPlatformWallet.FundRecipient>?
        get() = (this as? Loaded)?.payees
}

/**
 * Read the payees the lock at [outPointHex] recorded with [read] (the
 * wallet's [ManagedPlatformWallet.assetLockPlatformAddressRecipients]), or
 * [RecordedPayeesLoad.WaitingForWallet] while [read] is null because the
 * wallet is not loaded. An outpoint that does not parse, or a read that
 * throws, is [RecordedPayeesLoad.Failed]; a read that finds none is
 * [RecordedPayeesLoad.NotFound]. Cancellation propagates.
 */
internal suspend fun readRecordedPayees(
    outPointHex: String,
    read: (suspend (txid: ByteArray, vout: Int) -> List<ManagedPlatformWallet.FundRecipient>?)?,
): RecordedPayeesLoad {
    if (read == null) return RecordedPayeesLoad.WaitingForWallet
    val (txid, vout) = parseOutPoint(outPointHex)
        ?: return RecordedPayeesLoad.Failed("could not parse the asset lock outpoint $outPointHex")
    val payees = try {
        read(txid, vout)
    } catch (e: CancellationException) {
        throw e
    } catch (e: Exception) {
        return RecordedPayeesLoad.Failed(e.message ?: e.javaClass.simpleName)
    }
    return if (payees.isNullOrEmpty()) {
        RecordedPayeesLoad.NotFound
    } else {
        RecordedPayeesLoad.Loaded(payees)
    }
}
