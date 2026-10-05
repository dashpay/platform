import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for claiming a token distribution payout.
///
/// Inputs: distribution-type picker (`Perpetual` / `PreProgrammed` /
/// `OncePerIdentity`) listing only the kinds this identity can claim, not
/// every kind the token declares, plus an optional public note. Drive
/// charges for a claim it rejects, so a kind that pays someone else is not
/// an option worth offering. When only one is available the picker
/// auto-selects it and is disabled; when none is, the form refuses to
/// submit. Claim is not group-gated, so there's no group-action banner.
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
    /// Whether this identity's single once-per-identity claim is already
    /// spent. Seeded from the local record and re-set when Drive rejects a
    /// claim as already taken, so the kind drops out of the picker for the
    /// rest of the session rather than staying tappable at a fee per tap.
    @State private var oncePerIdentityClaimed: Bool
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance
    /// after the user pops + repushes mid-broadcast.
    @State private var submitGeneration: Int = 0

    /// Local record of once-per-identity claims. Injected so the form and
    /// the permission resolver read the same source, and so tests can
    /// supply their own.
    private let claims: OncePerIdentityClaimRecording

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    init(
        token: PersistentToken,
        identity: PersistentIdentity,
        claims: OncePerIdentityClaimRecording = OncePerIdentityClaimStore.shared
    ) {
        self.token = token
        self.identity = identity
        self.claims = claims
        // Start on the first kind this identity can claim, which is not
        // always the first kind the token declares: see
        // `TokenActionResolver.preferredClaimDistribution`. It returns nil
        // when the identity can claim nothing, and `.perpetual` is then a
        // placeholder that keeps the Picker's selection valid; nothing can
        // be submitted, because `canSubmit` requires the selected kind to
        // be in `availableDistributions`, which is empty in that case.
        self._selectedDistribution = State(
            initialValue: TokenActionResolver.preferredClaimDistribution(
                token: token,
                identity: identity,
                claims: claims
            ) ?? .perpetual
        )
        self._oncePerIdentityClaimed = State(
            initialValue: claims.hasClaimed(token: token, identity: identity)
        )
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

    /// What this identity can actually claim, not what the token declares.
    /// A kind it is not eligible for is not an option the picker should
    /// offer: submitting one is a rejection Drive charges for.
    private var availableDistributions: [TokenDistributionType] {
        let claimable = TokenActionResolver.claimableDistributions(
            token: token,
            identity: identity,
            claims: claims
        )
        // An identity gets one claim of the once-per-identity kind ever. The
        // resolver reads that from the same store, but the filter is on the
        // view's own state so the picker updates the moment a claim lands in
        // this session, without depending on when the store's write becomes
        // visible.
        guard oncePerIdentityClaimed else { return claimable }
        return claimable.filter { $0 != .oncePerIdentity }
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
        case .oncePerIdentity: return "Once per identity"
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
                    if dist == .oncePerIdentity {
                        self.recordOncePerIdentityClaim()
                    }
                    self.isSubmitting = false
                    self.dismiss()
                }
            } catch {
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    // Drive charges for a rejected claim, so a rejection
                    // that says the single claim is already taken is worth
                    // remembering: it is the only way this app learns about
                    // a claim it did not make itself (there is no DAPI query
                    // for it yet).
                    if OncePerIdentityClaimRejection.isAlreadyClaimed(error) {
                        self.recordOncePerIdentityClaim()
                    }
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }

    /// Remember that this identity's single once-per-identity claim is
    /// spent, and take the kind out of the picker. If it was the selected
    /// one, move the selection to whatever is left so the form does not sit
    /// on a value it can no longer submit.
    @MainActor
    private func recordOncePerIdentityClaim() {
        claims.recordClaim(token: token, identity: identity)
        oncePerIdentityClaimed = true
        if selectedDistribution == .oncePerIdentity,
           let fallback = availableDistributions.first {
            selectedDistribution = fallback
        }
    }
}
