import SwiftUI
import SwiftData
import SwiftDashSDK

/// Embeddable 5-step progress section for an address-funding flow.
/// Mirrors [`RegistrationProgressSection`] but for
/// `AddressFundingFromAssetLockTransition`.
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
///   5. Funding platform address      → activeLock `statusRaw ∈ {2, 3}`
///                                      AND controller still `.inFlight`
///
/// Exactly one of steps 3/4 is `.skipped` on a successful resolution:
/// step 4 is skipped when IS came back first (statusRaw == 2),
/// step 3 is skipped when CL did (statusRaw == 3, whether via
/// IS-timeout fallback or the `metadata.last_applied_chain_lock`
/// direct path). The faded checkmark distinguishes "passed through"
/// from "engaged" so users can see which finality lane resolved
/// the lock.
///
/// `.completed` is the *terminal* state and is not a separate step;
/// the parent `AddressFundFromAssetLockProgressView` renders the "Address
/// funded" banner + the new balance below this section. `.failed`
/// marks the current step with the error icon + message.
struct AddressFundFromAssetLockProgressSection: View {
    @ObservedObject var controller: AddressFundFromAssetLockController

    /// Asset-lock rows for this wallet, filtered to the
    /// AssetLockAddressTopUp variant (discriminant `4`). Queried
    /// live so step 2/3/4 transitions are reactive to status
    /// changes without polling.
    @Query private var activeLocks: [PersistentAssetLock]

    /// Cutoff (seconds since the row transitioned to `Broadcast`)
    /// between the visually-brief "Broadcasting" step (2) and the
    /// "Waiting for InstantSend proof" step (3). Same value as
    /// the identity-side progress section.
    private static let broadcastingWindow: TimeInterval = 2.0

    /// Cutoff (seconds since `Broadcast`) where the Rust side falls
    /// back from InstantSend to ChainLock. Mirrors
    /// `AssetLockManager`'s 300 s IS wait.
    private static let instantLockTimeout: TimeInterval = 300.0

    init(controller: AddressFundFromAssetLockController) {
        self.controller = controller
        let walletId = controller.walletId
        // `fundingTypeRaw == 4` is `AssetLockFundingType::AssetLockAddressTopUp`
        // per the discriminant comment on `PersistentAssetLock`. We
        // filter on it here so an interleaved identity registration's
        // asset lock can't be picked up by mistake — both flows
        // produce per-wallet asset-lock rows but only one funding
        // type matches this controller's domain.
        _activeLocks = Query(
            filter: #Predicate<PersistentAssetLock> { entry in
                entry.walletId == walletId && entry.fundingTypeRaw == 4
            },
            sort: [SortDescriptor(\PersistentAssetLock.updatedAt, order: .reverse)]
        )
    }

    var body: some View {
        // Same TimelineView pattern as RegistrationProgressSection
        // so the elapsed-time heuristic distinguishing step 2 / 3 / 4
        // refreshes without an external timer.
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
                Text("Top Up Progress")
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
    /// the terminal "Address funded" banner is rendered by the
    /// parent view, not by this section.
    private func currentStep(now: Date) -> Int {
        switch controller.phase {
        case .idle:
            return 1
        case .completed:
            // No visible "funded" step — terminalSection on
            // `AddressFundFromAssetLockProgressView` carries that state.
            // Return 6 so every step row (1...5) is marked `.done`.
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
                // built a CL proof directly). Step 4 done.
                return 5
            default:
                return 1
            }
        }
    }

    /// Resolve which of steps 2/3/4 is "active" while the lock is at
    /// `statusRaw == 1`. Uses elapsed time since the row's last
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
    /// (statusRaw == 2) so the CL fallback was never needed.
    private var step4WasSkipped: Bool {
        guard let lock = activeLocks.first else { return false }
        return lock.statusRaw == 2
    }

    /// True when step 3 ("Waiting for InstantSend proof") should
    /// appear "skipped" — i.e. no IS proof was observed during the
    /// step-3 window. Symmetric to `step4WasSkipped`:
    ///
    /// - `statusRaw == 3` — CL-locked. Either IS timed out and the
    ///   CL fallback ran, OR `wait_for_proof`'s
    ///   `metadata.last_applied_chain_lock` fallback built a Chain
    ///   proof directly without ever attempting IS.
    /// - `statusRaw == 1` + elapsed past the IS deadline. The lock
    ///   is still Broadcast but `broadcastSubStep` has advanced to
    ///   step 4 (CL wait) because IS didn't materialize within
    ///   `instantLockTimeout`. The guard on `idx < currentStep` in
    ///   `stepState` means this branch only matters when we're past
    ///   step 3 anyway, so a simple `statusRaw != 2` covers it
    ///   cleanly.
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
        case 5: return "Funding platform address"
        default: return ""
        }
    }

    /// Step-state classification. Drives the icon + tint on the
    /// row. `.skipped` is a softer pending variant for the IS or
    /// CL step the wallet didn't engage on the successful path —
    /// visually distinguishable so users don't think the step
    /// "didn't happen yet" once we've moved past it.
    enum StepState { case done, active, pending, skipped, failed }

    private func stepState(_ idx: Int, currentStep: Int, isFailed: Bool) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        if idx < currentStep {
            // Steps 3 and 4 are the IS / CL halves of the proof
            // round: exactly one of them is skipped on a successful
            // resolution. Step 4 skipped when IS came back first
            // (statusRaw == 2); step 3 skipped when CL did
            // (statusRaw == 3, whether via IS-timeout fallback or
            // the direct `last_applied_chain_lock` path).
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
            // Lighter checkmark to communicate "we passed this step
            // but didn't need it" — IS came back fast so CL was
            // skipped, or CL resolved directly so IS was skipped.
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
        case 5: return "Submitting the AddressFundingFromAssetLock state transition to Platform."
        default: return ""
        }
    }
}

/// Standalone navigation destination for an address funding in
/// flight, completed, or failed. Pushed from `FundFromAssetLockPlatformAddressView`
/// on submit and (later) from the "Resumable Top Up" surface.
struct AddressFundFromAssetLockProgressView: View {
    @ObservedObject var controller: AddressFundFromAssetLockController
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager

    init(controller: AddressFundFromAssetLockController) {
        self.controller = controller
    }

    var body: some View {
        Form {
            AddressFundFromAssetLockProgressSection(controller: controller)
            terminalSection
        }
        .navigationTitle("Top Up Platform Address")
        .navigationBarTitleDisplayMode(.inline)
    }

    @ViewBuilder
    private var terminalSection: some View {
        switch controller.phase {
        case .completed(let newBalance):
            Section {
                VStack(alignment: .leading, spacing: 8) {
                    Label("Address funded", systemImage: "checkmark.seal.fill")
                        .foregroundColor(.green)
                        .font(.headline)
                    HStack {
                        Text("New balance")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text(formatCredits(newBalance))
                            .font(.system(.body, design: .monospaced))
                    }
                    Button {
                        walletManager.addressFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            platformAccountIndex: controller.platformAccountIndex,
                            recipientHash: controller.recipientHash
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
                    Label("Top Up failed", systemImage: "xmark.octagon.fill")
                        .foregroundColor(.red)
                        .font(.headline)
                    Text(message)
                        .font(.callout)
                        .foregroundColor(.primary)
                        .textSelection(.enabled)
                    // Dismissal path mirroring the inline terminal
                    // section in `FundFromAssetLockPlatformAddressView`. Without
                    // this the only way to clear a `.failed`
                    // controller from a pushed progress view was to
                    // relaunch the app — the `Pending Platform
                    // Funding` row's `.swipeActions` doesn't fire
                    // outside a List, so neither surface had a
                    // working dismissal.
                    Button {
                        walletManager.addressFundFromAssetLockCoordinator.dismiss(
                            walletId: controller.walletId,
                            platformAccountIndex: controller.platformAccountIndex,
                            recipientHash: controller.recipientHash
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

    private func formatCredits(_ credits: UInt64) -> String {
        // 1e11 credits per DASH — same divisor used by
        // `CreateIdentityView` for Platform-side amounts.
        let dash = Double(credits) / 100_000_000_000.0
        return String(format: "%.6f DASH (credits)", dash)
    }
}
