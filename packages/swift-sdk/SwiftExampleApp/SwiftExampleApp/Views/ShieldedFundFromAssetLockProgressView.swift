import SwiftUI
import SwiftData
import SwiftDashSDK

/// Embeddable 5-step progress section for a shielded
/// fund-from-asset-lock flow. Mirrors
/// [`AddressFundFromAssetLockProgressSection`] but for Type 18
/// (`ShieldFromAssetLockTransition`).
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
///                                      >= `instantLockTimeout` (Rust
///                                      side has fallen back to CL);
///                                      also done when
///                                      `statusRaw == 3` (CL-locked).
///   5. Shielding (Halo 2 + submit)   → activeLock `statusRaw ∈ {2, 3}`
///                                      AND controller still `.inFlight`
///
/// Step 5 is meaningfully longer than its address-funding sibling
/// because Type 18 builds a ~30s Halo 2 proof inside the FFI call.
/// The footer explicitly calls that out so users don't think the
/// app is hung.
///
/// Exactly one of steps 3/4 is `.skipped` on a successful
/// resolution — same finality-lane semantics as the address-funding
/// progress section.
struct ShieldedFundFromAssetLockProgressSection: View {
    @ObservedObject var controller: ShieldedFundFromAssetLockController

    /// Asset-lock rows for this wallet, filtered to the
    /// AssetLockShieldedAddressTopUp variant (discriminant `5`).
    /// Queried live so step 2/3/4 transitions are reactive to
    /// status changes without polling.
    @Query private var activeLocks: [PersistentAssetLock]

    /// Cutoff (seconds since the row transitioned to `Broadcast`)
    /// between the visually-brief "Broadcasting" step (2) and the
    /// "Waiting for InstantSend proof" step (3). Same value as the
    /// address-funding progress section.
    private static let broadcastingWindow: TimeInterval = 2.0

    /// Cutoff (seconds since `Broadcast`) where the Rust side falls
    /// back from InstantSend to ChainLock. Mirrors
    /// `AssetLockManager`'s 300 s IS wait.
    private static let instantLockTimeout: TimeInterval = 300.0

    init(controller: ShieldedFundFromAssetLockController) {
        self.controller = controller
        let walletId = controller.walletId
        // `fundingTypeRaw == 5` is
        // `AssetLockFundingType::AssetLockShieldedAddressTopUp` per
        // the discriminant comment on `PersistentAssetLock`. We
        // filter on it here so an interleaved Type 14 address-
        // funding asset lock can't be picked up by mistake — both
        // flows produce per-wallet asset-lock rows but only one
        // funding type matches this controller's domain.
        _activeLocks = Query(
            filter: #Predicate<PersistentAssetLock> { entry in
                entry.walletId == walletId && entry.fundingTypeRaw == 5
            },
            sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)]
        )
    }

    var body: some View {
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
                Text("Shield Progress")
            } footer: {
                Text(footerText(step: step, isFailed: isFailed))
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Step computation

    private func currentStep(now: Date) -> Int {
        switch controller.phase {
        case .idle:
            return 1
        case .completed:
            // No visible "shielded" step — the parent
            // `ShieldedFundFromAssetLockProgressView` carries that
            // state. Return 6 so every step row (1...5) is marked
            // `.done`.
            return 6
        case .failed:
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
            guard let lock = activeLocks.first else { return 1 }
            switch lock.statusRaw {
            case 0:
                return 1
            case 1:
                return broadcastSubStep(for: lock, now: now)
            case 2:
                // InstantSend-locked. Never went through step 4
                // (CL fallback); it stays as `.skipped`.
                return 5
            case 3:
                // ChainLock-locked. Step 3 (IS) is skipped (no IS
                // proof was observed — either IS timed out and CL
                // fallback ran, or `metadata.last_applied_chain_lock`
                // built a CL proof directly).
                return 5
            default:
                return 1
            }
        }
    }

    private func broadcastSubStep(for lock: PersistentAssetLock, now: Date) -> Int {
        let elapsed = now.timeIntervalSince(lock.updatedAt)
        if elapsed < Self.broadcastingWindow { return 2 }
        if elapsed < Self.instantLockTimeout { return 3 }
        return 4
    }

    /// True when step 4 should appear "skipped" — the lock came back
    /// InstantSend-locked (statusRaw == 2) so the CL fallback was
    /// never needed.
    private var step4WasSkipped: Bool {
        guard let lock = activeLocks.first else { return false }
        return lock.statusRaw == 2
    }

    /// True when step 3 should appear "skipped" — symmetric to
    /// `step4WasSkipped`. Same semantics as the address-funding
    /// progress section.
    private var step3WasSkipped: Bool {
        guard let lock = activeLocks.first else { return false }
        return lock.statusRaw != 2
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
        case 5: return "Building Orchard proof and shielding"
        default: return ""
        }
    }

    enum StepState { case done, active, pending, skipped, failed }

    private func stepState(_ idx: Int, currentStep: Int, isFailed: Bool) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        if idx < currentStep {
            if idx == 3 && step3WasSkipped {
                return .skipped
            }
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
            return "Tap Dismiss to clear this entry."
        }
        switch step {
        case 1: return "Building a Core asset-lock transaction from wallet funds."
        case 2: return "Sending the asset-lock transaction to peers."
        case 3: return "Waiting for the InstantSend lock so the asset-lock proof is final."
        case 4: return "InstantSend timed out; falling back to ChainLock finality (~2 min)."
        case 5: return "Building the Halo 2 proof (~30s) and submitting the ShieldFromAssetLock state transition to Platform. The shielded note appears in your balance after the next sync pass."
        default: return ""
        }
    }
}

/// Standalone navigation destination for a shielded funding in
/// flight, completed, or failed. Pushed from
/// `ShieldedFundFromAssetLockView` on submit.
struct ShieldedFundFromAssetLockProgressView: View {
    @ObservedObject var controller: ShieldedFundFromAssetLockController
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    init(controller: ShieldedFundFromAssetLockController) {
        self.controller = controller
    }

    var body: some View {
        Form {
            ShieldedFundFromAssetLockProgressSection(controller: controller)
            terminalSection
        }
        .navigationTitle("Shield from Asset Lock")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var terminalSection: some View {
        switch controller.phase {
        case .completed:
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Shielded", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    Text(
                        "The shielded note will appear in your balance after the "
                            + "next sync pass — broadcast already succeeded; the note is "
                            + "decrypted asynchronously by the shielded sync coordinator."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                    Button {
                        walletManager.shieldedFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            recipientRaw43: controller.recipientRaw43
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
                    Label("Shield failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    Button {
                        walletManager.shieldedFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            recipientRaw43: controller.recipientRaw43
                        )
                        dismiss()
                    } label: {
                        Text("Dismiss")
                            .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                    .padding(.top, 4)
                }
            }
        default:
            EmptyView()
        }
    }

}
