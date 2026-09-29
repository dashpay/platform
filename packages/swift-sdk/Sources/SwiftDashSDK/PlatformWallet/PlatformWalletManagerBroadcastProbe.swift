import Foundation
import DashSDKFFI

/// What the network said about an unconfirmed send whose broadcast outcome
/// was unknown ("Transaction status unknown").
public enum OutgoingTransactionVerdict: Sendable, Equatable {
    /// A node holds the transaction: it is in a mempool or a block.
    case accepted
    /// Two distinct nodes proved it can never be mined (for example its input
    /// no longer exists). `reason` is diagnostic, never user-facing copy.
    case dead(reason: String)
    /// No verdict yet; the SDK asks again on a later block.
    case unresolved(reason: String)
}

/// Identifies one unconfirmed send across the wallets of a manager.
public struct OutgoingTransactionKey: Hashable, Sendable {
    public let walletId: Data
    /// 32 bytes, wire (internal) byte order.
    public let txidWire: Data

    public init(walletId: Data, txidWire: Data) {
        self.walletId = walletId
        self.txidWire = txidWire
    }
}

/// One broadcast-probe verdict. Nothing in the wallet changed because of it.
public struct OutgoingTransactionProbeEvent: Sendable, Equatable {
    public let walletId: Data
    /// 32 bytes, wire (internal) byte order.
    public let txidWire: Data
    public let verdict: OutgoingTransactionVerdict

    /// The txid as explorers and the UI display it (byte-reversed hex).
    public var txidDisplayHex: String {
        txidWire.reversed().map { String(format: "%02x", $0) }.joined()
    }

    public var key: OutgoingTransactionKey {
        OutgoingTransactionKey(walletId: walletId, txidWire: txidWire)
    }

    public init(walletId: Data, txidWire: Data, verdict: OutgoingTransactionVerdict) {
        self.walletId = walletId
        self.txidWire = txidWire
        self.verdict = verdict
    }
}

/// C trampoline for
/// `EventHandlerCallbacksExtension.on_outgoing_transaction_probed_fn`. Rust
/// owns every pointer only for this call, so everything is copied before the
/// hop to the main queue. `DispatchQueue.main` is FIFO, so a verdict and a
/// later clear for the same send are applied in the order Rust sent them —
/// separate `Task`s give no such guarantee.
func outgoingTransactionProbedCallback(
    context: UnsafeMutableRawPointer?,
    walletIdPtr: UnsafePointer<UInt8>?,
    txidPtr: UnsafePointer<UInt8>?,
    verdictCode: UInt8,
    reasonPtr: UnsafePointer<CChar>?
) {
    guard let context, let walletIdPtr, let txidPtr else { return }
    let handler = Unmanaged<PlatformWalletEventHandler>
        .fromOpaque(context)
        .takeUnretainedValue()

    let walletId = Data(bytes: walletIdPtr, count: 32)
    let txidWire = Data(bytes: txidPtr, count: 32)
    if verdictCode == UInt8(OUTGOING_PROBE_VERDICT_CLEARED) {
        let key = OutgoingTransactionKey(walletId: walletId, txidWire: txidWire)
        DispatchQueue.main.async { [weak manager = handler.manager] in
            MainActor.assumeIsolated {
                manager?.handleOutgoingTransactionCleared(key)
            }
        }
        return
    }
    let reason = reasonPtr.map { String(cString: $0) } ?? ""
    let verdict: OutgoingTransactionVerdict
    switch verdictCode {
    case UInt8(OUTGOING_PROBE_VERDICT_ACCEPTED):
        verdict = .accepted
    case UInt8(OUTGOING_PROBE_VERDICT_DEAD):
        verdict = .dead(reason: reason)
    default:
        verdict = .unresolved(reason: reason)
    }
    let event = OutgoingTransactionProbeEvent(
        walletId: walletId,
        txidWire: txidWire,
        verdict: verdict
    )

    DispatchQueue.main.async { [weak manager = handler.manager] in
        MainActor.assumeIsolated {
            manager?.handleOutgoingTransactionProbed(event)
        }
    }
}

extension PlatformWalletManager {
    /// Turn automatic probing of unconfirmed sends on or off (off by default).
    ///
    /// While on, the SDK resubmits every unconfirmed send whose broadcast
    /// outcome went quiet to evonodes over DAPI — right after SPV reports it
    /// uncertain, then every block for 24 blocks and every 10 after — and
    /// publishes each *change* of
    /// verdict in `outgoingTransactionVerdicts` / `lastOutgoingTransactionProbe`.
    /// A send that settles or leaves the wallet is removed from the map.
    /// Resubmitting sends the same signed bytes: it can deliver a payment, it
    /// can never create a second one. Nothing in the wallet changes.
    public func setBroadcastProbeEnabled(_ enabled: Bool) throws {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidHandle("PlatformWalletManager not configured")
        }
        try platform_wallet_manager_set_broadcast_probe_enabled(handle, enabled).check()
    }

    @MainActor
    func handleOutgoingTransactionProbed(_ event: OutgoingTransactionProbeEvent) {
        guard !shutdownRequested, isConfigured else { return }
        outgoingTransactionVerdicts[event.key] = event
        lastOutgoingTransactionProbe = event
    }

    /// The send settled or left the wallet: its verdict no longer applies.
    @MainActor
    func handleOutgoingTransactionCleared(_ key: OutgoingTransactionKey) {
        guard !shutdownRequested, isConfigured else { return }
        outgoingTransactionVerdicts.removeValue(forKey: key)
        if lastOutgoingTransactionProbe?.key == key {
            lastOutgoingTransactionProbe = nil
        }
    }
}
