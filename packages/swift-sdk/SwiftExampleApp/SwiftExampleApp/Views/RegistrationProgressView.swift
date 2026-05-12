import SwiftUI
import SwiftData
import SwiftDashSDK

/// 5-step stage-aware progress UI for an in-flight identity
/// registration. Drilled into from `CreateIdentityView`'s submit
/// section or from the "Pending registrations" row on the
/// identities tab.
///
/// Step mapping (from the plan, iter 3 part 2):
///
///   1. Preparing identity keys     → controller `.preparingKeys`
///   2. Building asset-lock tx       → activeLock `statusRaw == 0`
///   3. Broadcasting & waiting       → activeLock `statusRaw == 1`
///   4. Submitting to Platform       → activeLock `statusRaw == 2 or 3`
///                                     AND controller still `.inFlight`
///   5. Identity registered          → controller `.completed`
///
/// `.failed` aliases to step 5 with the error message inline.
struct RegistrationProgressView: View {
    @ObservedObject var controller: IdentityRegistrationController

    /// Asset-lock rows for this slot, queried live so step 2/3/4
    /// transitions are reactive without polling. The predicate is
    /// keyed by the same `(walletId, identityIndex)` tuple the
    /// coordinator uses.
    @Query private var activeLocks: [PersistentAssetLock]

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
        let step = currentStep
        let isFailed = isFailed
        let errorMessage = failureMessage

        Form {
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
        .navigationTitle("Registration")
        .navigationBarTitleDisplayMode(.inline)
    }

    // MARK: - Step computation

    /// 1...5, current active step (or 5 on terminal states).
    private var currentStep: Int {
        switch controller.phase {
        case .idle, .preparingKeys:
            return 1
        case .completed:
            return 5
        case .failed:
            // Pick the step at which we failed by reading the
            // latest known lock status. The terminal indicator
            // (red on step 5) is what the user sees, but the
            // partial fill of earlier steps tells them how far
            // we got.
            if let lock = activeLocks.first {
                switch lock.statusRaw {
                case 0: return 2
                case 1: return 3
                case 2, 3: return 4
                default: return 1
                }
            }
            return 5
        case .inFlight:
            guard let lock = activeLocks.first else {
                // No lock row yet — Rust has the slot but hasn't
                // emitted the first changeset. We're still
                // logically in step 2 (building the asset-lock
                // tx).
                return 2
            }
            switch lock.statusRaw {
            case 0:
                return 2
            case 1:
                return 3
            case 2, 3:
                return 4
            default:
                return 2
            }
        }
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
        case 1: return "Preparing identity keys"
        case 2: return "Building asset-lock transaction"
        case 3: return "Broadcasting & waiting for InstantSend lock"
        case 4: return "Submitting to Platform"
        case 5: return "Identity registered"
        default: return ""
        }
    }

    /// Step-state classification. Drives the icon + tint on the
    /// row.
    enum StepState { case done, active, pending, failed }

    private func stepState(_ idx: Int, currentStep: Int, isFailed: Bool) -> StepState {
        if isFailed && idx == currentStep {
            return .failed
        }
        if idx < currentStep {
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
        case .failed: return .red
        }
    }

    private func footerText(step: Int, isFailed: Bool) -> String {
        if isFailed {
            return "Tap Dismiss in Pending Registrations to clear this entry."
        }
        switch step {
        case 1: return "Deriving keys locally and persisting to the Keychain."
        case 2: return "Building a Core asset-lock transaction from wallet funds."
        case 3: return "Waiting for the InstantSend lock so the asset-lock proof is final."
        case 4: return "Submitting the IdentityCreate state transition to Platform."
        case 5: return "Identity registered."
        default: return ""
        }
    }
}
