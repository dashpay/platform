import SwiftUI
import SwiftData
import SwiftDashSDK

/// Embeddable 5-step progress section. Use inside any parent
/// `Form` so the progress UI doesn't nest a second `Form`, which
/// SwiftUI doesn't render cleanly. For the standalone navigation
/// destination see `RegistrationProgressView` below.
///
/// Step mapping:
///
///   1. Building asset-lock tx        → activeLock `statusRaw == 0`
///   2. Broadcasting                  → activeLock `statusRaw == 1` and
///                                      < `broadcastingWindow` since
///                                      the row's `updatedAt`
///   3. Waiting for InstantSend proof → activeLock `statusRaw == 1` and
///                                      between `broadcastingWindow`
///                                      and `instantLockTimeout`
///   4. Waiting for ChainLock proof   → activeLock `statusRaw == 1` and
///                                      >= `instantLockTimeout` (the
///                                      Rust side has fallen back to
///                                      ChainLock); also done when
///                                      `statusRaw == 3` because that
///                                      proof type finalised the lock
///   5. Registering identity          → activeLock `statusRaw == 2 or 3`
///                                      AND controller still `.inFlight`
///
/// `.completed` is the *terminal* state and is not a separate step;
/// `RegistrationProgressView` renders the "Identity created" banner
/// + "View Identity" navigation below this section in its own
/// terminalSection. `.failed` marks the current step with the error
/// icon + message. Step 4 is shown as `.skipped` (faded checkmark)
/// when the IS branch came back fast so the user can see step 4 was
/// passed through without engaging the CL fallback.
struct RegistrationProgressSection: View {
    @ObservedObject var controller: IdentityRegistrationController

    /// Asset-lock rows for this slot, queried live so step 2/3/4/5
    /// transitions are reactive to status changes without polling.
    @Query private var activeLocks: [PersistentAssetLock]

    /// Cutoff (seconds since the row transitioned to `Broadcast`)
    /// between the visually-brief "Broadcasting" step (3) and the
    /// "Waiting for InstantSend proof" step (4). Tuned short so
    /// step 3 doesn't visually linger — by the time the user sees
    /// the page, the broadcast is already on the wire.
    private static let broadcastingWindow: TimeInterval = 2.0

    /// Cutoff (seconds since `Broadcast`) where the Rust side falls
    /// back from InstantSend to ChainLock. Mirrors
    /// `AssetLockManager`'s 300 s IS wait. If `statusRaw == 1` is
    /// still the state after this window, the wallet is in the CL
    /// fallback window (180 s); we mark step 4 done and step 5
    /// active to communicate the shift.
    private static let instantLockTimeout: TimeInterval = 300.0

    init(controller: IdentityRegistrationController) {
        self.controller = controller
        let walletId = controller.walletId
        let identityIndex = controller.identityIndex
        _activeLocks = Query(
            filter: PersistentAssetLock.predicate(
                walletId: walletId,
                identityIndex: identityIndex
            ),
            sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)]
        )
    }

    var body: some View {
        // `TimelineView` re-fires the body every 1 s so the
        // elapsed-time heuristic that distinguishes step 2 / 3 / 4
        // refreshes without an external timer. The lock row's
        // `updatedAt` is the anchor.
        TimelineView(.periodic(from: .now, by: 1.0)) { timeline in
            let now = timeline.date
            let step = currentStep(now: now)
            let isFailed = isFailed
            let errorMessage = failureMessage

            Section {
                ForEach(1...5, id: \.self) { idx in
                    stepRow(
                        index: idx,
                        title: stepTitle(idx),
                        state: stepState(idx, currentStep: step, isFailed: isFailed)
                    )
                    if idx == 5, let message = errorMessage {
                        Text(message)
                            .font(.caption)
                            .foregroundColor(.red)
                            .padding(.leading, 32)
                    }
                }
            } header: {
                Text("Registration Progress")
            } footer: {
                Text(footerText(step: step, isFailed: isFailed))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Step computation

    /// 1...5, current active step. On `.completed` we report 6 (one
    /// past the last visual step) so all rows render as `.done`;
    /// the terminal "Identity created" banner is rendered by the
    /// parent `RegistrationProgressView`, not by this section.
    /// `now` is the time-of-rendering, used to drive the
    /// Broadcasting → Waiting-IS → Waiting-CL transition within
    /// `statusRaw == 1`.
    private func currentStep(now: Date) -> Int {
        switch controller.phase {
        case .idle, .preparingKeys:
            return 1
        case .completed:
            // No visible "registered" step — terminalSection on
            // `RegistrationProgressView` carries that state. Return
            // 6 so every step row (1...5) is marked `.done`.
            return 6
        case .failed:
            // Pick the step at which we failed by reading the
            // latest known lock status. The terminal indicator
            // (red on the failed step) is what the user sees,
            // but the partial fill of earlier steps tells them
            // how far we got.
            if let lock = activeLocks.first {
                switch lock.statusRaw {
                case 0: return 1
                case 1: return broadcastSubStep(for: lock, now: now)
                case 2, 3: return 5
                default: return 1
                }
            }
            return 5
        case .inFlight:
            guard let lock = activeLocks.first else {
                // No lock row yet — Rust has the slot but hasn't
                // emitted the first changeset. We're still
                // logically in step 1 (building the asset-lock
                // tx).
                return 1
            }
            switch lock.statusRaw {
            case 0:
                return 1
            case 1:
                return broadcastSubStep(for: lock, now: now)
            case 2:
                // InstantSend-locked. We never went through step 4
                // (CL fallback); it stays as `.skipped`. Step 5
                // active.
                return 5
            case 3:
                // ChainLock-locked. Both step 3 and step 4 done
                // (CL fallback path). Step 5 active.
                return 5
            default:
                return 1
            }
        }
    }

    /// Resolve which of steps 2/3/4 is "active" while the lock is
    /// at `statusRaw == 1`. Uses elapsed time since the row's last
    /// update as the anchor: brief broadcasting window first, then
    /// IS wait until the Rust-side timeout, then the CL fallback.
    private func broadcastSubStep(for lock: PersistentAssetLock, now: Date) -> Int {
        let elapsed = now.timeIntervalSince(lock.updatedAt)
        if elapsed < Self.broadcastingWindow { return 2 }
        if elapsed < Self.instantLockTimeout { return 3 }
        return 4
    }

    /// True when step 4 should appear "skipped" rather than
    /// "active" — i.e. the lock came back InstantSend-locked
    /// (statusRaw == 2) so the CL fallback was never needed. Drives
    /// a distinct pending visual on step 4 in the success path
    /// without bleeding into other gray rows.
    private var step4WasSkipped: Bool {
        guard let lock = activeLocks.first else { return false }
        return lock.statusRaw == 2
    }

    private var isFailed: Bool {
        if case .failed = controller.phase { return true }
        return false
    }

    private var failureMessage: String? {
        if case .failed(let msg) = controller.phase { return msg }
        return nil
    }

    private func stepTitle(_ idx: Int) -> String {
        switch idx {
        case 1: return "Building asset-lock transaction"
        case 2: return "Broadcasting"
        case 3: return "Waiting for InstantSend proof"
        case 4: return "Waiting for ChainLock proof"
        case 5: return "Registering identity"
        default: return ""
        }
    }

    /// Step-state classification. Drives the icon + tint on the
    /// row. `.skipped` is a softer pending variant for step 5 when
    /// the IS branch returned the proof without needing ChainLock
    /// fallback — visually distinguishable so users don't think
    /// the step "didn't happen yet" once we've moved past it.
    enum StepState { case done, active, pending, skipped, failed }

    private func stepState(_ idx: Int, currentStep: Int, isFailed: Bool) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        if idx < currentStep {
            // Step 4 is the only one that can be "skipped" while
            // a later step is active — when the IS path returned
            // the proof and ChainLock fallback was never engaged.
            if idx == 4 && step4WasSkipped {
                return .skipped
            }
            return .done
        }
        if idx == currentStep {
            return .active
        }
        return .pending
    }

    // MARK: - Row UI

    @ViewBuilder
    private func stepRow(index: Int, title: String, state: StepState) -> some View {
        HStack(spacing: 12) {
            stepIcon(index: index, state: state)
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.callout)
                    .foregroundColor(stepTextColor(state))
            }
            Spacer()
        }
    }

    @ViewBuilder
    private func stepIcon(index: Int, state: StepState) -> some View {
        switch state {
        case .done:
            Image(systemName: "checkmark.circle.fill")
                .foregroundColor(.green)
                .font(.title3)
        case .active:
            ProgressView()
                .scaleEffect(0.7)
                .frame(width: 22, height: 22)
        case .pending:
            ZStack {
                Circle()
                    .stroke(Color.secondary.opacity(0.4), lineWidth: 1)
                    .frame(width: 22, height: 22)
                Text("\(index)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        case .skipped:
            // Lighter checkmark to communicate "we passed this
            // step but didn't need it" — typically step 5 when
            // the InstantSend branch returned the proof and the
            // ChainLock fallback was never engaged.
            Image(systemName: "checkmark.circle")
                .foregroundColor(.secondary)
                .font(.title3)
        case .failed:
            Image(systemName: "xmark.octagon.fill")
                .foregroundColor(.red)
                .font(.title3)
        }
    }

    private func stepTextColor(_ state: StepState) -> Color {
        switch state {
        case .done: return .primary
        case .active: return .primary
        case .pending: return .secondary
        case .skipped: return .secondary
        case .failed: return .red
        }
    }

    private func footerText(step: Int, isFailed: Bool) -> String {
        if isFailed {
            return "Tap Dismiss in Pending Registrations to clear this entry."
        }
        switch step {
        case 1: return "Building a Core asset-lock transaction from wallet funds."
        case 2: return "Sending the asset-lock transaction to peers."
        case 3: return "Waiting for the InstantSend lock so the asset-lock proof is final."
        case 4: return "InstantSend timed out; falling back to ChainLock finality (~2 min)."
        case 5: return "Submitting the IdentityCreate state transition to Platform."
        default: return ""
        }
    }
}

/// Standalone navigation destination for a registration in flight,
/// completed, or failed. Pushed from `CreateIdentityView` on submit
/// and from the "Pending Registrations" row on the identities tab.
/// Renders the 7-step progress, plus the terminal section on
/// `.completed` (success banner + "View Identity" navigation) or
/// `.failed` (inline error). Embedders that already render a
/// `Form` should use `RegistrationProgressSection` directly.
struct RegistrationProgressView: View {
    @ObservedObject var controller: IdentityRegistrationController
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    init(controller: IdentityRegistrationController) {
        self.controller = controller
    }

    var body: some View {
        Form {
            RegistrationProgressSection(controller: controller)
            terminalSection
        }
        .navigationTitle("Registration")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var terminalSection: some View {
        switch controller.phase {
        case .completed(let identityId):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Identity created", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(identityId.toBase58String())
                        .font(.system(.caption, design: .monospaced))
                        .foregroundColor(.secondary)
                        .textSelection(.enabled)
                    // Dismisses the pushed progress view AND drops
                    // the controller from the coordinator so the
                    // "Pending Registrations" row on the Identities
                    // tab clears immediately (instead of lingering
                    // ~30 s for the retention sweep).
                    Button {
                        walletManager.registrationCoordinator.dismiss(
                            walletId: controller.walletId,
                            identityIndex: controller.identityIndex
                        )
                        dismiss()
                    } label: {
                        Text("Done")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .padding(.top, 4)
                }
            }
        case .failed(let message):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Registration failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                }
            }
        default:
            EmptyView()
        }
    }
}
