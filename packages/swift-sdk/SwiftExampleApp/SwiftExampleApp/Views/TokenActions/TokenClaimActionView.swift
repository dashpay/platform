import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for claiming a token distribution payout.
///
/// Inputs: distribution-type picker (`PreProgrammed` / `Perpetual`)
/// driven by which schedule the token has, plus an optional public
/// note. When only one schedule is configured the picker auto-selects
/// it and is disabled; when neither is configured the form refuses to
/// submit. Claim is not group-gated, so there's no group-action
/// banner.
struct TokenClaimActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var selectedDistribution: TokenDistributionType
    @State private var publicNote: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance
    /// after the user pops + repushes mid-broadcast.
    @State private var submitGeneration: Int = 0

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    init(token: PersistentToken, identity: PersistentIdentity) {
        self.token = token
        self.identity = identity
        // Default to whichever schedule is present; perpetual wins
        // when both exist (matches Drive's claim ordering). When
        // neither exists we still need a default — `.perpetual`
        // keeps the picker valid; submission is gated by
        // `availableDistributions.isEmpty`.
        let perpetual = token.perpetualDistribution != nil
        let preProgrammed = token.preProgrammedDistribution != nil
        if perpetual {
            self._selectedDistribution = State(initialValue: .perpetual)
        } else if preProgrammed {
            self._selectedDistribution = State(initialValue: .preProgrammed)
        } else {
            self._selectedDistribution = State(initialValue: .perpetual)
        }
    }

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
            }

            if availableDistributions.isEmpty {
                Section {
                    Label(
                        "No distributions available to claim.",
                        systemImage: "exclamationmark.circle"
                    )
                    .foregroundColor(.secondary)
                }
            } else {
                Section("Distribution type") {
                    Picker("Distribution", selection: $selectedDistribution) {
                        ForEach(availableDistributions, id: \.self) { dist in
                            Text(displayName(for: dist)).tag(dist)
                        }
                    }
                    .pickerStyle(.segmented)
                    .disabled(availableDistributions.count <= 1)
                }
            }

            Section("Public note (optional)") {
                TextField("Note", text: $publicNote, axis: .vertical)
                    .lineLimit(1...3)
            }

            Section {
                Button {
                    submit()
                } label: {
                    HStack {
                        if isSubmitting {
                            ProgressView().controlSize(.small)
                            Text("Submitting…")
                        } else {
                            Text("Claim")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Claim")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Claim failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
    }

    // MARK: - Derived state

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    private var availableDistributions: [TokenDistributionType] {
        var types: [TokenDistributionType] = []
        if token.perpetualDistribution != nil {
            types.append(.perpetual)
        }
        if token.preProgrammedDistribution != nil {
            types.append(.preProgrammed)
        }
        return types
    }

    private var canSubmit: Bool {
        guard !availableDistributions.isEmpty else { return false }
        guard availableDistributions.contains(selectedDistribution) else { return false }
        return managedWallet != nil
    }

    private func displayName(for dist: TokenDistributionType) -> String {
        switch dist {
        case .perpetual: return "Perpetual"
        case .preProgrammed: return "Pre-Programmed"
        }
    }

    // MARK: - Submit

    private func submit() {
        guard let wallet = managedWallet else {
            submitError = .init(message: "Wallet unavailable.")
            return
        }
        guard availableDistributions.contains(selectedDistribution) else {
            submitError = .init(message: "Selected distribution type is not available.")
            return
        }

        guard let position = UInt16(exactly: token.position) else {
            submitError = .init(message: "Invalid token position.")
            return
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note
        let dist = selectedDistribution

        Task {
            do {
                try await wallet.tokenClaim(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    distributionType: dist,
                    publicNote: publicNoteOrNil,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.dismiss()
                }
            } catch {
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }
}
