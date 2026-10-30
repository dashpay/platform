import Foundation
import SwiftDashSDK

/// Application-owned tip submissions. They outlive the sheet that started
/// them: a dismissed sheet must never imply the native send stopped
/// broadcasting, and reopening from either entry point (the DashPay tab or
/// the profile) has to find the same guard. Mirrors the Kotlin example app's
/// `ShieldedTipSubmissions`.
@MainActor
final class ShieldedTipSubmissions: ObservableObject {
    private var wallets: [String: ShieldedTipSubmission] = [:]

    /// One guard per network and wallet, across all of its identities.
    func forWallet(network: Network, walletId: Data) -> ShieldedTipSubmission {
        let key = "\(network.rawValue):\(walletId.hexString)"
        if let existing = wallets[key] { return existing }
        let created = ShieldedTipSubmission()
        wallets[key] = created
        return created
    }
}

/// Main-actor state of one wallet's shielded tip submission.
@MainActor
final class ShieldedTipSubmission: ObservableObject {
    enum Status: Equatable { case ready, sending, sent, uncertain }

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
                if Self.canReviewAfterFailure(error) {
                    status = .ready
                    message = error.localizedDescription
                } else {
                    status = .uncertain
                    message = "The tip may have been sent. Check shielded activity before sending "
                        + "again. \(error.localizedDescription)"
                }
            }
        }
    }

    /// Starting another payment is an explicit action after a confirmed success.
    func startNewTip() {
        guard status == .sent else { return }
        status = .ready
        message = nil
    }

    /// Only failures known not to have executed a tip permit a fresh review.
    /// On the tip path Rust maps selection, build and recipient-check failures
    /// to `walletOperation`, and broadcast ambiguity to
    /// `shieldedSpendUnconfirmed`; anything unknown, including cancellation,
    /// stays locked until shielded activity shows the outcome.
    static func canReviewAfterFailure(_ error: Error) -> Bool {
        guard let error = error as? PlatformWalletError else { return false }
        switch error {
        case .nullPointer, .invalidHandle, .invalidParameter, .invalidIdentifier, .invalidNetwork,
             .walletOperation, .identityNotFound, .contactNotFound, .utf8Conversion,
             .noSelectableInputs, .shieldedBroadcastFailed, .shieldedNoRecordedAnchor,
             .shieldedInsufficientBalance:
            return true
        default:
            return false
        }
    }
}
