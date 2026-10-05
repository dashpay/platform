import Foundation
import DashSDKFFI

/// What the network said about an unconfirmed send whose broadcast outcome
/// was unknown ("Transaction status unknown").
public enum OutgoingTransactionVerdict: Sendable, Equatable {
    /// A node holds the transaction: it is in a mempool or a block.
    case accepted
    /// No verdict yet; the SDK asks again on a later block. A refusal by the
    /// nodes asked lands here too: no answer proves a send can never land.
    /// `reason` is diagnostic, never user-facing copy.
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
    default:
        // UNRESOLVED, and the reserved OUTGOING_PROBE_VERDICT_DEAD, which
        // this version never emits.
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
    /// While on, the SDK resubmits to evonodes over DAPI the root of every
    /// chain of unconfirmed sends — the first unsettled transaction the chain
    /// depends on, which may be an incoming payment the wallet spent — right
    /// after SPV reports a broadcast uncertain (each report), then, while the
    /// wallet follows the chain tip, every block for 24 blocks from when it
    /// first did so for that root, and every 10 blocks after — a return from
    /// the background does not restart the 24 — plus at the end of a sync
    /// that was catching up (a launch, a reconnect). A root no evonode
    /// answered (DAPI unreachable) is due again at every block, and retried a
    /// minute later if no block has come (then after 2 and 4 more minutes;
    /// at most three retries per block). Each pass probes a bounded number of
    /// scheduled roots, in the order they were last probed (or first seen),
    /// so with a large backlog a root waits its turn. The roots of sends
    /// reported uncertain go ahead of them until those sends settle, taking
    /// at most half of a pass while other roots wait. A report, or a retry
    /// of probes no evonode answered, makes a pass under way stop after its
    /// probe in flight once per block; a later one waits for that pass.
    /// It publishes each *change* of
    /// verdict for the wallet's own sends in `outgoingTransactionVerdicts` /
    /// `lastOutgoingTransactionProbe` (an accepted send whose nodes' block
    /// evidence comes or goes may be published accepted again). Every verdict
    /// is advisory: only the wallet's own sync settles a send. An entry is removed when its send
    /// settles or leaves the wallet, when its wallet is deleted, and every
    /// entry when probing is turned off; a removal does not mean the send
    /// settled.
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

    /// Drop the send's verdict: the send settled or left the wallet, its wallet
    /// was removed, or probing was turned off. Not a statement that it settled.
    @MainActor
    func handleOutgoingTransactionCleared(_ key: OutgoingTransactionKey) {
        guard !shutdownRequested, isConfigured else { return }
        outgoingTransactionVerdicts.removeValue(forKey: key)
        if lastOutgoingTransactionProbe?.key == key {
            lastOutgoingTransactionProbe = nil
        }
    }
}
