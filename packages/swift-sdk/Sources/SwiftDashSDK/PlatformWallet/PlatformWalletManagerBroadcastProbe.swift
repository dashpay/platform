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

    public init(walletId: Data, txidWire: Data, verdict: OutgoingTransactionVerdict) {
        self.walletId = walletId
        self.txidWire = txidWire
        self.verdict = verdict
    }
}

/// C trampoline for
/// `EventHandlerCallbacksExtension.on_outgoing_transaction_probed_fn`. Rust
/// owns every pointer only for this call, so everything is copied before the
/// main-actor hop.
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
        walletId: Data(bytes: walletIdPtr, count: 32),
        txidWire: Data(bytes: txidPtr, count: 32),
        verdict: verdict
    )

    Task { @MainActor [weak manager = handler.manager] in
        manager?.handleOutgoingTransactionProbed(event)
    }
}

extension PlatformWalletManager {
    /// Turn automatic probing of unconfirmed sends on or off (off by default).
    ///
    /// While on, the SDK resubmits every unconfirmed send whose broadcast
    /// outcome went quiet to evonodes over DAPI — right after SPV reports it
    /// uncertain, then once per block — and publishes each verdict in
    /// `outgoingTransactionVerdicts` / `lastOutgoingTransactionProbe`.
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
        outgoingTransactionVerdicts[event.txidWire] = event
        lastOutgoingTransactionProbe = event
    }
}
