import Foundation
import SwiftDashSDK

/// Application-owned tip submissions. They outlive the sheet that started
/// them, so a dismissed sheet never implies the native send stopped and
/// reopening from either entry point (the DashPay tab or the profile) finds the
/// same state. Restart safety is the wallet's: it records each tip as
/// unresolved before broadcast and refuses another until it settles.
@MainActor
final class ShieldedTipSubmissions: ObservableObject {
    private var wallets: [String: ShieldedTipSubmission] = [:]

    /// One submission per network and wallet, across all of its identities.
    func forWallet(network: Network, walletId: Data) -> ShieldedTipSubmission {
        let key = "\(network.rawValue):\(walletId.hexString)"
        if let existing = wallets[key] { return existing }
        let created = ShieldedTipSubmission()
        wallets[key] = created
        return created
    }
}

/// Main-actor state of one wallet's shielded tip submission. It renders the
/// wallet's decisions; it makes none.
@MainActor
final class ShieldedTipSubmission: ObservableObject {
    enum Status: Equatable { case ready, sending, sent, unresolved }

    @Published private(set) var status: Status = .ready
    @Published private(set) var message: String?

    var busy: Bool { status == .sending }
    var submitted: Bool { status != .ready }

    /// Locks synchronously, before the task starts, so two UI events cannot
    /// submit twice. Returns nil when a submission already holds the lock; the
    /// task is returned so tests can await the outcome.
    @discardableResult
    func submit(_ send: @escaping @MainActor () async throws -> Void) -> Task<Void, Never>? {
        guard !submitted else { return nil }
        status = .sending
        message = nil
        return Task { @MainActor in
            do {
                try await send()
                status = .sent
                message = "Shielded tip sent."
            } catch {
                if Self.wasNotSent(error) {
                    status = .ready
                    message = error.localizedDescription
                } else {
                    status = .unresolved
                    message = "The tip may have been sent. Shielded sync settles it; the wallet "
                        + "refuses another tip to this recipient until then. \(error.localizedDescription)"
                }
            }
        }
    }

    /// Starting another payment is an explicit action after a finished send.
    /// After an unresolved one, the wallet still refuses a tip to that
    /// recipient until shielded sync settles it; the review shows that.
    func startNewTip() {
        guard status == .sent || status == .unresolved else { return }
        status = .ready
        message = nil
    }

    /// The wallet's disposition decides; the SDK reports every send failure as
    /// a `ShieldedTipFailure`. Anything else (cancellation) is unknown.
    static func wasNotSent(_ error: Error) -> Bool {
        (error as? ShieldedTipFailure)?.disposition == .notSent
    }
}
