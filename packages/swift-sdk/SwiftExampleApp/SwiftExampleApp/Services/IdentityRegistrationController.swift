import Foundation
import SwiftDashSDK

/// Per-slot state owned by a single identity registration attempt.
///
/// One controller is created per `(walletId, identityIndex)` slot
/// when the user submits `CreateIdentityView`. The controller owns
/// the in-flight `Task`, exposes its current `phase` via
/// `@Published`, and survives view dismissal via
/// `RegistrationCoordinator` on `PlatformWalletManager`. Multiple
/// controllers can be active simultaneously (one per slot); each
/// runs on `@MainActor` so SwiftUI observers see consistent
/// transitions.
///
/// The 5-step progress bar in `RegistrationProgressView` derives its
/// step from a combination of `phase` (Step 1, 4, 5) and the live
/// `PersistentAssetLock` row queried via `@Query` filtered by
/// `(walletId, identityIndex)` (Step 2/3, driven by `statusRaw`).
@MainActor
final class IdentityRegistrationController: ObservableObject {
    enum Phase: Equatable {
        /// Pre-submit. The controller exists but `submit` hasn't
        /// fired yet. Not surfaced by `RegistrationProgressView`
        /// (the view only opens after a submit).
        case idle
        /// Step 1: pre-deriving + Keychain-persisting the identity
        /// keys via `prePersistIdentityKeysForRegistration`. The
        /// caller drives this transition before invoking `submit`.
        case preparingKeys
        /// Steps 2–4 inclusive: the FFI registration call is in
        /// flight. Stage within this phase is read from the
        /// matching `PersistentAssetLock.statusRaw` row.
        case inFlight
        /// Step 5: identity is registered. `identityId` is the
        /// 32-byte identifier the caller should persist /
        /// navigate to.
        case completed(identityId: Data)
        /// Failure terminal state. The message is shown inline in
        /// `RegistrationProgressView`'s step 5; the row stays in
        /// the coordinator's map until the user dismisses it
        /// manually.
        case failed(String)

        /// Whether the controller is currently holding its slot
        /// against a fresh registration. Used by the Resumable
        /// Registrations surface to hide orphan asset locks whose
        /// slot is mid-flight — otherwise the same lock could
        /// appear in both Pending and Resumable lists during the
        /// window between asset-lock broadcast and PersistentIdentity
        /// write, letting the user race a duplicate Resume tap
        /// against the original FFI call.
        ///
        /// `.completed` is NOT active here: by the time the
        /// controller flips to `.completed` the persister has
        /// (or is about to) write a `PersistentIdentity` row,
        /// which the identity-slot anti-join already covers.
        /// `.failed` is NOT active either: the user is expected
        /// to retry, so the lock should resurface for selection.
        var isActive: Bool {
            switch self {
            case .preparingKeys, .inFlight:
                return true
            case .idle, .completed, .failed:
                return false
            }
        }
    }

    /// Current phase. Updates flow:
    /// `.idle` → `.preparingKeys` (caller) → `.inFlight` (submit) →
    /// `.completed(id) | .failed(message)`.
    @Published private(set) var phase: Phase = .idle

    /// Slot this controller is bound to. Stored so the coordinator
    /// and the progress view can filter `PersistentAssetLock` rows
    /// by `(walletId, identityIndex)`.
    let walletId: Data
    let identityIndex: UInt32

    /// Timestamp of the most recent `submit` call. Used by the
    /// coordinator's TTL-based retention policy (`.completed` rows
    /// purge ~30s after the success transition).
    private(set) var lastSubmittedAt: Date?

    /// Active registration task. Holds a reference so the
    /// coordinator's stash retains the work until completion;
    /// cancellation isn't wired today (the FFI call doesn't yet
    /// support clean abort), but the field lets future work hang
    /// off the same shape.
    private var task: Task<Void, Never>?

    init(walletId: Data, identityIndex: UInt32) {
        self.walletId = walletId
        self.identityIndex = identityIndex
    }

    /// Transition to `.preparingKeys`. Called by the caller before
    /// `submit` while it pre-derives the identity public keys via
    /// `prePersistIdentityKeysForRegistration`.
    func enterPreparingKeys() {
        phase = .preparingKeys
    }

    /// Submit the registration. Defensively rejects any phase that
    /// shouldn't fire a fresh FFI call:
    ///   - `.inFlight`: a second FFI call would race the first,
    ///     ending with a misleading "asset lock consumed" failure.
    ///   - `.completed`: re-submitting after success would flip the
    ///     UI from "Done" back to a spinner before failing on the
    ///     consumed lock.
    /// `.idle`, `.preparingKeys`, and `.failed` are allowed — the
    /// coordinator drives the legitimate-restart flow through them
    /// (callers must call `enterPreparingKeys()` before `submit()`,
    /// `failed → preparingKeys → submit` is how a user retries).
    ///
    /// `body` performs the actual FFI call. It runs detached on a
    /// background priority and reports the identity id on success
    /// or rethrows on failure. The controller flips `phase` to
    /// `.completed` / `.failed` accordingly.
    func submit(body: @escaping () async throws -> Data) {
        switch phase {
        case .idle, .preparingKeys, .failed:
            break
        case .inFlight, .completed:
            return
        }
        phase = .inFlight
        lastSubmittedAt = Date()
        task = Task { [weak self] in
            do {
                let identityId = try await body()
                await MainActor.run {
                    self?.phase = .completed(identityId: identityId)
                }
            } catch {
                await MainActor.run {
                    self?.phase = .failed(error.localizedDescription)
                }
            }
        }
    }
}
