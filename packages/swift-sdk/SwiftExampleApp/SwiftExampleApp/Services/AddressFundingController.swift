import Foundation
import SwiftDashSDK

/// Per-slot state owned by a single platform-address funding attempt.
///
/// Mirrors [`IdentityRegistrationController`] for the
/// `AddressFundingFromAssetLockTransition` flow. One controller is
/// created per `(walletId, platformAccountIndex, recipientHash)`
/// slot when the user submits `FundPlatformAddressView`. The
/// controller owns the in-flight `Task`, exposes its current `phase`
/// via `@Published`, and survives view dismissal via
/// `AddressFundingCoordinator` on `PlatformWalletManager`.
///
/// The 4-step progress in `AddressFundingProgressView` derives its
/// step from a combination of `phase` (Step 1, Step 4) and the live
/// `PersistentAssetLock` row queried via `@Query` filtered by
/// `walletId` + the asset-lock funding-type discriminant (Step 2/3,
/// driven by `statusRaw`).
///
/// Address-funded asset locks differ from identity-registration asset
/// locks in one important way: there's no per-identity-index slot. A
/// wallet can fund many addresses from the same funding-type family,
/// so the slot here keys on the recipient address hash rather than a
/// numeric index. The Rust-side asset-lock builder still pulls fresh
/// credit-output keys from the `AssetLockAddressTopUp` BIP44 family;
/// the index advances naturally per call.
@MainActor
final class AddressFundingController: ObservableObject {
    enum Phase: Equatable {
        /// Pre-submit. The controller exists but `submit` hasn't
        /// fired yet. Not surfaced by the progress view (the view
        /// only opens after a submit).
        case idle
        /// Steps 1-3 inclusive: the FFI funding call is in flight.
        /// Stage within this phase is read from the matching
        /// `PersistentAssetLock.statusRaw` row.
        case inFlight
        /// Step 4: the address has been credited. `newBalance` is
        /// the proof-attested credit balance the caller should
        /// surface in the terminal banner.
        case completed(newBalance: UInt64)
        /// Failure terminal state. Message is shown inline in
        /// `AddressFundingProgressView`'s step 4; the row stays in
        /// the coordinator's map until the user dismisses it
        /// manually.
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
    /// `.completed(balance) | .failed(message)`.
    @Published private(set) var phase: Phase = .idle

    /// Wallet this controller is bound to. Stored so the coordinator
    /// and the progress view can filter `PersistentAssetLock` rows
    /// by `(walletId, fundingTypeRaw == AssetLockAddressTopUp)`.
    let walletId: Data

    /// Platform-payment account index of the recipient. Stored for
    /// the resume surface label and the live progress query.
    let platformAccountIndex: UInt32

    /// 20-byte hash of the recipient platform address. Composite-key
    /// component so two concurrent funds to different addresses on
    /// the same account don't collide.
    let recipientHash: Data

    /// Timestamp of the most recent `submit` call. Used by the
    /// coordinator's TTL-based retention policy (`.completed` rows
    /// purge ~30s after the success transition).
    private(set) var lastSubmittedAt: Date?

    /// Active funding task. Holds a reference so the coordinator's
    /// stash retains the work until completion; cancellation isn't
    /// wired today (the FFI call doesn't yet support clean abort).
    private var task: Task<Void, Never>?

    init(walletId: Data, platformAccountIndex: UInt32, recipientHash: Data) {
        self.walletId = walletId
        self.platformAccountIndex = platformAccountIndex
        self.recipientHash = recipientHash
    }

    /// Submit the funding. Defensively rejects any phase that
    /// shouldn't fire a fresh FFI call:
    ///   - `.inFlight`: a second FFI call would race the first.
    ///   - `.completed`: re-submitting after success would flip the
    ///     UI from "Done" back to a spinner before failing on the
    ///     consumed lock.
    /// `.idle` and `.failed` are allowed — the coordinator drives
    /// the legitimate-restart flow through them (a user retries a
    /// failure via `failed → submit`).
    ///
    /// `body` performs the actual FFI call. It runs detached on a
    /// background priority and reports the new credit balance on
    /// success or rethrows on failure. The controller flips `phase`
    /// to `.completed(balance)` / `.failed(message)` accordingly.
    func submit(body: @escaping () async throws -> UInt64) {
        switch phase {
        case .idle, .failed:
            break
        case .inFlight, .completed:
            return
        }
        phase = .inFlight
        lastSubmittedAt = Date()
        task = Task { [weak self] in
            do {
                let newBalance = try await body()
                await MainActor.run {
                    self?.phase = .completed(newBalance: newBalance)
                }
            } catch {
                await MainActor.run {
                    self?.phase = .failed(error.localizedDescription)
                }
            }
        }
    }
}
