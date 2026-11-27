package org.dashfoundation.dashsdk.wallet

import kotlinx.coroutines.CancellationException
import org.dashfoundation.dashsdk.errors.DashSdkError
import org.dashfoundation.dashsdk.errors.mapNativeErrors
import org.dashfoundation.dashsdk.ffi.WalletDrainNative
import java.util.UUID
import java.util.concurrent.ConcurrentHashMap

// Core to shielded "Max" (a wallet drain): preview, execution and status by a
// durable operation id. Port of
// `packages/swift-sdk/Sources/SwiftDashSDK/PlatformWallet/PlatformWalletManagerWalletDrain.swift`,
// kept in its own file as there. Manager-handle calls like the other shielded
// funding wrappers; only meaningful on a shielded build ([org.dashfoundation.dashsdk.Sdk.hasShielded]).

/**
 * Drain calls of this process that have not returned. Such a call may not
 * have reached Rust's operation registry yet (it sits between this wrapper
 * and that registry while it waits for a thread), which is a fact only the
 * host knows; what a status read reports in that window is Rust's rule
 * ([WalletDrainRules.statusDuringHostCall]). Counted, so one returning call
 * cannot clear another caller's protection. Process-wide, as the Swift
 * wrapper's `walletDrainCalls` is.
 */
internal object WalletDrainActiveCalls {
    private data class Key(val managerHandle: Long, val walletIdHex: String, val operationId: UUID)

    private val counts = ConcurrentHashMap<Key, Int>()

    private fun key(managerHandle: Long, walletId: ByteArray, operationId: UUID) =
        Key(managerHandle, walletId.joinToString("") { "%02x".format(it) }, operationId)

    fun enter(managerHandle: Long, walletId: ByteArray, operationId: UUID) {
        counts.merge(key(managerHandle, walletId, operationId), 1, Int::plus)
    }

    fun leave(managerHandle: Long, walletId: ByteArray, operationId: UUID) {
        counts.computeIfPresent(key(managerHandle, walletId, operationId)) { _, count ->
            if (count > 1) count - 1 else null
        }
    }

    fun isActive(managerHandle: Long, walletId: ByteArray, operationId: UUID): Boolean =
        counts.containsKey(key(managerHandle, walletId, operationId))
}

/**
 * Estimate a Max of [fundingAccountIndex] without reserving inputs, deriving
 * addresses, signing or broadcasting. Port of Swift's
 * `shieldedWalletDrainPreview`. Show the amounts as estimates.
 */
suspend fun PlatformWalletManager.shieldedWalletDrainPreview(
    walletId: ByteArray,
    fundingAccountIndex: Int = 0,
): WalletDrainPreview = teardownGate.op {
    require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
    require(fundingAccountIndex >= 0) {
        "fundingAccountIndex must be non-negative, got $fundingAccountIndex"
    }
    mapNativeErrors {
        WalletDrainPreview.decode(
            WalletDrainNative.preview(managerHandle, walletId, fundingAccountIndex),
        )
    }
}

/**
 * Drain the permitted transparent accounts at [fundingAccountIndex] into one
 * Orchard recipient. Port of Swift's `shieldedFundFromWalletDrain`.
 *
 * This does not ask [WalletDrainRules.startOffered]: a caller that starts a
 * NEW drain asks it first and does not call this where the answer is false
 * (mainnet, until dashpay/platform#5580). The example app does.
 *
 * Store [operationId] durably BEFORE calling. A repeated id names the same
 * operation; never replace it with a new one after an unknown result. Rust
 * picks the inputs and the amount, and associates the asset lock with the id
 * before it is sent.
 *
 * Never throws for a failed drain: the returned outcome carries the state,
 * the asset lock if one exists and the error together, and a failure that
 * happens before Rust is reached is returned as pending, because it cannot
 * show that an earlier call under the same id sent nothing. A native library
 * that lacks this entry point is a packaging error, not a failed drain: the
 * JVM's `UnsatisfiedLinkError` is not caught, and nothing has run by then.
 * Blocks for the asset-lock proof wait and the Halo 2 proof.
 */
suspend fun PlatformWalletManager.shieldedFundFromWalletDrain(
    walletId: ByteArray,
    operationId: UUID,
    fundingAccountIndex: Int = 0,
    recipientRaw43: ByteArray,
): WalletDrainOutcome {
    WalletDrainActiveCalls.enter(managerHandle, walletId, operationId)
    try {
        return teardownGate.op {
            require(walletId.size == 32) {
                "walletId must be exactly 32 bytes, got ${walletId.size}"
            }
            require(fundingAccountIndex >= 0) {
                "fundingAccountIndex must be non-negative, got $fundingAccountIndex"
            }
            require(recipientRaw43.size == 43) {
                "recipientRaw43 must be exactly 43 bytes, got ${recipientRaw43.size}"
            }
            WalletDrainOutcome.decode(
                mapNativeErrors {
                    WalletDrainNative.fundFromWalletDrainV2(
                        managerHandle,
                        walletId,
                        operationId.toWalletDrainOperationIdBytes(),
                        fundingAccountIndex,
                        recipientRaw43,
                        mnemonicResolverHandle,
                    )
                },
            )
        }
    } catch (e: CancellationException) {
        throw e
    } catch (e: DashSdkError) {
        return WalletDrainOutcome.unresolved(e)
    } catch (e: Exception) {
        // A closed manager or a rejected argument.
        return WalletDrainOutcome.unresolved(
            DashSdkError.InvalidState(e.message ?: e.toString(), e),
        )
    } finally {
        WalletDrainActiveCalls.leave(managerHandle, walletId, operationId)
    }
}

/**
 * Read one operation by its id, including after a restart. Port of Swift's
 * `shieldedWalletDrainStatus`. Never touches the network and sends nothing.
 *
 * Throws when the read fails. Neither a failed read nor a pending result is
 * evidence that the operation was not sent.
 */
suspend fun PlatformWalletManager.shieldedWalletDrainStatus(
    walletId: ByteArray,
    operationId: UUID,
): WalletDrainOutcome {
    // Read before the native call: a drain that starts during it has entered
    // Rust's registry by the time it returns, or is covered by the caller's
    // own overlap accounting.
    val callActive = WalletDrainActiveCalls.isActive(managerHandle, walletId, operationId)
    val native = teardownGate.op {
        require(walletId.size == 32) { "walletId must be exactly 32 bytes, got ${walletId.size}" }
        val outcome = WalletDrainOutcome.decode(
            mapNativeErrors {
                WalletDrainNative.statusV2(
                    managerHandle,
                    walletId,
                    operationId.toWalletDrainOperationIdBytes(),
                )
            },
        )
        // The status call returns no error beside its outcome, so one here
        // means the outcome could not be decoded.
        outcome.error?.let { throw it }
        outcome
    }
    // What to report while this host's own call is active is Rust's rule;
    // this wrapper only supplies the fact that one is.
    val state = WalletDrainRules.statusDuringHostCall(native.state, hostCallActive = callActive)
    return if (state == native.state) native else WalletDrainOutcome(state, native.outpoint)
}
