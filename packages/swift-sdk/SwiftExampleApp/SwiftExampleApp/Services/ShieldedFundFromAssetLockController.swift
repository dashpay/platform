import Foundation
import SwiftDashSDK

/// Per-slot state owned by a single shielded fund-from-asset-lock
/// attempt.
///
/// Mirrors [`AddressFundFromAssetLockController`] for the Type 18
/// (`ShieldFromAssetLockTransition`) flow. One controller is created
/// per `(walletId, recipientRaw43)` slot when the user submits
/// `ShieldedFundFromAssetLockView`. The controller owns the in-flight
/// `Task`, exposes its current `phase` via `@Published`, and survives
/// view dismissal via `ShieldedFundFromAssetLockCoordinator` on
/// `PlatformWalletManager`.
///
/// Unlike address-funding, shielded doesn't carry a
/// `platformAccountIndex` in the key — recipients are external
/// Orchard payment addresses (43 raw bytes) that aren't allocated
/// from a wallet account. Single-flighting is keyed on the recipient
/// alone: a wallet can fund many distinct shielded recipients
/// concurrently, but the same recipient can only have one in-flight
/// shield at a time.
@MainActor
final class ShieldedFundFromAssetLockController: ObservableObject {
    enum Phase: Equatable {
        /// Pre-submit. The controller exists but `submit` hasn't
        /// fired yet. Not surfaced by the progress view (the view
        /// only opens after a submit).
        case idle
        /// Steps 1-3 inclusive: the FFI shield call is in flight.
        /// Stage within this phase is read from the matching
        /// `PersistentAssetLock.statusRaw` row.
        case inFlight
        /// Step 4 / terminal: the shield ST has been accepted by
        /// Platform. The Orchard note arrives via the next shielded
        /// sync — the FFI call returns `Void`, so unlike the
        /// address-funding sibling there's no proof-attested
        /// balance to surface here.
        case completed
        /// Failure terminal state. Message is shown inline in the
        /// progress view's step 4; the row stays in the
        /// coordinator's map until the user dismisses it manually.
        case failed(String)

        /// Whether the controller is currently holding its slot.
        /// Used by the Resumable Funding surface to hide orphan
        /// asset locks whose slot is mid-flight — otherwise the
        /// same lock could appear in both Pending and Resumable
        /// lists during the broadcast-to-success window, letting
        /// the user race a duplicate Resume tap against the
        /// original FFI call.
        var isActive: Bool {
            switch self {
            case .inFlight:
                return true
            case .idle, .completed, .failed:
                return false
            }
        }
    }

    /// Current phase. Updates flow:
    /// `.idle` → `.inFlight` (submit) →
    /// `.completed | .failed(message)`.
    @Published private(set) var phase: Phase = .idle

    /// Wallet this controller is bound to. Stored so the coordinator
    /// and the progress view can filter `PersistentAssetLock` rows
    /// by `(walletId, fundingTypeRaw == AssetLockShieldedAddressTopUp)`.
    let walletId: Data

    /// 43-byte raw Orchard payment address (11-byte diversifier +
    /// 32-byte pk_d). Composite-key component so two concurrent
    /// shields to different recipients on the same wallet don't
    /// collide.
    let recipientRaw43: Data

    /// Timestamp of the most recent `submit` call. Used by the
    /// coordinator's TTL-based retention policy (`.completed` rows
    /// purge ~30s after the success transition).
    private(set) var lastSubmittedAt: Date?

    /// Identity of the operation currently occupying this slot — the
    /// fresh-shield marker or the resumed lock's outpoint. Set by every
    /// accepted `submit`. The coordinator compares it to tell a re-tap
    /// of the SAME operation (reuse the controller) from a DIFFERENT
    /// operation on the same `(walletId, recipient)` slot, whose body
    /// `submit` would otherwise silently drop (two resumable locks
    /// normally share the wallet's default shielded recipient).
    private(set) var operationId: String?

    /// Active funding task. Holds a reference so the coordinator's
    /// stash retains the work until completion; cancellation isn't
    /// wired today (the FFI call doesn't yet support clean abort).
    private var task: Task<Void, Never>?

    init(walletId: Data, recipientRaw43: Data) {
        self.walletId = walletId
        self.recipientRaw43 = recipientRaw43
    }

    /// Submit the funding. Defensively rejects any phase that
    /// shouldn't fire a fresh FFI call:
    ///   - `.inFlight`: a second FFI call would race the first
    ///     (and burn the same asset lock twice).
    ///   - `.completed`: re-submitting after success would flip the
    ///     UI from "Done" back to a spinner before failing on the
    ///     consumed lock.
    /// `.idle` and `.failed` are allowed — the coordinator drives
    /// the legitimate-restart flow through them (a user retries a
    /// failure via `failed → submit`).
    ///
    /// `operationId` names the operation the body performs (see
    /// `operationId`). `body` performs the actual FFI call. It runs
    /// detached on a background priority. Unlike the address-funding
    /// sibling, the FFI returns `Void` — the shielded note arrives via
    /// the next sync, not from the broadcast call — so the controller
    /// flips `phase` to `.completed` (no balance payload) / `.failed`
    /// accordingly.
    func submit(operationId: String, body: @escaping () async throws -> Void) {
        switch phase {
        case .idle, .failed:
            break
        case .inFlight, .completed:
            return
        }
        self.operationId = operationId
        phase = .inFlight
        lastSubmittedAt = Date()
        task = Task { [weak self] in
            do {
                try await body()
                await MainActor.run {
                    self?.phase = .completed
                }
            } catch {
                await MainActor.run {
                    self?.phase = .failed(error.localizedDescription)
                }
            }
        }
    }
}
