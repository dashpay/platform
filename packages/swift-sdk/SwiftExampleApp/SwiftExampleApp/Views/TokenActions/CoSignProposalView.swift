import SwiftUI
import SwiftData
import SwiftDashSDK

/// Review-and-sign view for an existing group-action proposal.
///
/// Loads the current signer set + group membership data, then exposes
/// a single "Co-sign" button that dispatches to the appropriate
/// `tokenXxx` method on `ManagedPlatformWallet` with
/// `GroupActionMode.signExisting(...)` and the original proposal's
/// parameters. The form fields the original proposer filled out are
/// shown read-only — co-signers don't get to edit them, they just
/// approve or walk away.
struct CoSignProposalView: View {
    let token: PersistentToken
    let identity: PersistentIdentity
    let proposal: TokenGroupAction
    /// The group contract position the proposal was discovered under
    /// — i.e. the position passed to
    /// `tokenPendingGroupActions(groupContractPosition:)`. This is the
    /// position used to query signers and to dispatch the co-sign;
    /// the proposal's `tokenContractPosition` is a different field
    /// (the token's slot in the contract).
    let groupContractPosition: UInt16

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var signers: [TokenGroupActionSigner] = []
    @State private var isLoadingSigners: Bool = false
    @State private var loadError: String?
    @State private var isSubmitting: Bool = false
    @State private var submitMessage: AlertMessage?
    /// Generation counter so a late `MainActor.run` from a previous
    /// `dispatch()` Task can't write back to a re-entered view
    /// instance after the user pops + repushes mid-broadcast.
    @State private var submitGeneration: Int = 0

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let title: String
        let message: String
    }

    var body: some View {
        Form {
            Section("Proposal") {
                LabeledContent("Type", value: proposal.type.displayLabel)
                LabeledContent(
                    "Action ID",
                    value: short(proposal.actionId.toBase58String())
                )
                LabeledContent(
                    "Proposer",
                    value: short(proposal.proposer.toBase58String())
                )
                LabeledContent(
                    "Token position",
                    value: "\(proposal.tokenContractPosition)"
                )
            }

            paramsSection

            Section("Signatures") {
                if isLoadingSigners {
                    ProgressView()
                } else {
                    LabeledContent(
                        "Total power",
                        value: "\(totalPower) of \(requiredPower)"
                    )
                    LabeledContent(
                        "Threshold",
                        value: "\(signers.count) of \(memberCount) members signed"
                    )
                    if hasUserSigned {
                        Label("You have signed this proposal.", systemImage: "checkmark.seal.fill")
                            .foregroundColor(.green)
                            .font(.caption)
                    } else if isUserMember {
                        Label("You have not signed yet.", systemImage: "hourglass")
                            .foregroundColor(.orange)
                            .font(.caption)
                    } else {
                        Label(
                            "You are not a member of this group.",
                            systemImage: "lock.fill"
                        )
                        .foregroundColor(.secondary)
                        .font(.caption)
                    }
                    if let loadError {
                        Text(loadError)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }

            if !signers.isEmpty {
                Section("Signers") {
                    ForEach(signers, id: \.identityId) { signer in
                        HStack {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                            VStack(alignment: .leading) {
                                Text(short(signer.identityId.toBase58String()))
                                    .font(.caption.monospaced())
                                Text("Power \(signer.power)")
                                    .font(.caption2)
                                    .foregroundColor(.secondary)
                            }
                            if signer.identityId == proposal.proposer {
                                Spacer()
                                Text("Proposer")
                                    .font(.caption2)
                                    .foregroundColor(.accentColor)
                            }
                        }
                    }
                }
            }

            Section {
                Button {
                    coSign()
                } label: {
                    HStack {
                        if isSubmitting {
                            ProgressView().controlSize(.small)
                            Text("Signing…")
                        } else {
                            Text(coSignButtonLabel)
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canCoSign || isSubmitting)
                if let reason = coSignDisabledReason {
                    Text(reason)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .navigationTitle("Co-sign Proposal")
        .navigationBarTitleDisplayMode(.inline)
        .task {
            await reload()
        }
        .alert(item: $submitMessage) { msg in
            Alert(
                title: Text(msg.title),
                message: Text(msg.message),
                dismissButton: .default(Text("OK")) {
                    if msg.title == "Signed" {
                        dismiss()
                    }
                }
            )
        }
    }

    @ViewBuilder
    private var paramsSection: some View {
        Section("Parameters") {
            switch proposal.params {
            case .mint(let amount, let recipient, let note):
                LabeledContent("Amount", value: "\(amount)")
                LabeledContent("Recipient", value: short(recipient.toBase58String()))
                if let note { LabeledContent("Note", value: note) }
            case .burn(let amount, let burnFrom, let note):
                LabeledContent("Amount", value: "\(amount)")
                LabeledContent("Burn from", value: short(burnFrom.toBase58String()))
                if let note { LabeledContent("Note", value: note) }
            case .freeze(let target, let note):
                LabeledContent("Target", value: short(target.toBase58String()))
                if let note { LabeledContent("Note", value: note) }
            case .unfreeze(let target, let note):
                LabeledContent("Target", value: short(target.toBase58String()))
                if let note { LabeledContent("Note", value: note) }
            case .destroyFrozenFunds(let target, let amount, let note):
                LabeledContent("Target", value: short(target.toBase58String()))
                LabeledContent("Amount", value: "\(amount)")
                if let note { LabeledContent("Note", value: note) }
            case .pause(let note):
                Text("Halts all token operations.")
                    .font(.subheadline)
                if let note { LabeledContent("Note", value: note) }
            case .resume(let note):
                Text("Resumes all token operations.")
                    .font(.subheadline)
                if let note { LabeledContent("Note", value: note) }
            case .setPrice(let price, let note):
                if let price {
                    LabeledContent("Price per token", value: "\(price) credits")
                } else {
                    Text("Disables direct purchase.").font(.subheadline)
                }
                if let note { LabeledContent("Note", value: note) }
            case .directPurchase(let amount, let totalCost):
                LabeledContent("Amount", value: "\(amount)")
                LabeledContent("Total cost", value: "\(totalCost) credits")
            case .other:
                Text("This proposal type isn't supported by the co-sign UI yet.")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
            }
        }
    }

    // MARK: - Derived

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    private var groupPosition: Int? {
        Int(groupContractPosition)
    }

    private var groupMembers: [String: Int] {
        guard let contract = token.dataContract,
              let groups = contract.groups,
              let position = groupPosition,
              let group = groups[String(position)] as? [String: Any],
              let members = group["members"] as? [String: Any]
        else { return [:] }
        return members.compactMapValues { value in
            if let i = value as? Int { return i }
            if let n = value as? NSNumber { return n.intValue }
            return nil
        }
    }

    private var memberCount: Int { groupMembers.count }

    private var totalPower: UInt32 {
        signers.reduce(0) { $0 + $1.power }
    }

    private var requiredPower: UInt32 {
        guard let contract = token.dataContract,
              let groups = contract.groups,
              let position = groupPosition,
              let group = groups[String(position)] as? [String: Any],
              let value = group["requiredPower"]
        else { return 0 }
        if let i = value as? Int {
            // Negative or out-of-range values are non-sensical for a
            // signature threshold; treat as "no required power known".
            guard let v = UInt32(exactly: i) else { return 0 }
            return v
        }
        if let n = value as? NSNumber { return n.uint32Value }
        return 0
    }

    private var isUserMember: Bool {
        groupMembers.keys.contains(identity.identityIdBase58)
    }

    private var hasUserSigned: Bool {
        signers.contains { $0.identityId == identity.identityId }
    }

    private var isUserProposer: Bool {
        proposal.proposer == identity.identityId
    }

    private var canCoSign: Bool {
        guard managedWallet != nil else { return false }
        guard isUserMember else { return false }
        guard !hasUserSigned else { return false }
        switch proposal.params {
        case .directPurchase, .other:
            return false
        default:
            return true
        }
    }

    private var coSignButtonLabel: String {
        if hasUserSigned { return "Already signed" }
        if !isUserMember { return "Not a group member" }
        if isUserProposer { return "Re-sign as proposer" }
        return "Co-sign"
    }

    private var coSignDisabledReason: String? {
        if !isUserMember { return "Only members of the gating group can sign." }
        if hasUserSigned { return "You already signed this proposal." }
        switch proposal.params {
        case .directPurchase, .other:
            return "This proposal type isn't supported by the co-sign UI yet."
        default:
            return nil
        }
    }

    // MARK: - Loading signers

    @MainActor
    private func reload() async {
        guard let wallet = managedWallet,
              let position = groupPosition
        else { return }
        isLoadingSigners = true
        loadError = nil
        defer { isLoadingSigners = false }

        do {
            signers = try await wallet.tokenGroupActionSigners(
                contractId: token.contractId,
                groupContractPosition: UInt16(position),
                status: .pending,
                actionId: proposal.actionId
            )
        } catch {
            loadError = error.localizedDescription
        }
    }

    // MARK: - Co-sign dispatch

    private func coSign() {
        guard let wallet = managedWallet,
              let position = groupPosition
        else {
            submitMessage = .init(
                title: "Co-sign failed",
                message: "Wallet or group position not available."
            )
            return
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let mode = GroupActionMode.signExisting(
            position: UInt16(position),
            actionId: proposal.actionId,
            actionIsProposer: isUserProposer
        )
        let identityId = identity.identityId
        let contractId = token.contractId
        let tokenPosition = proposal.tokenContractPosition

        Task {
            do {
                try await dispatch(
                    wallet: wallet,
                    mode: mode,
                    signer: signer,
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: tokenPosition
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.submitMessage = .init(
                        title: "Signed",
                        message: "Your signature was broadcast."
                    )
                }
            } catch {
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.submitMessage = .init(
                        title: "Co-sign failed",
                        message: error.localizedDescription
                    )
                }
            }
        }
    }

    /// Route to the right `tokenXxx` wrapper based on the proposal's
    /// payload. Each branch passes the original proposal's params
    /// straight through — the co-signer doesn't get to edit them.
    private func dispatch(
        wallet: ManagedPlatformWallet,
        mode: GroupActionMode,
        signer: KeychainSigner,
        identityId: Data,
        contractId: Data,
        tokenPosition: UInt16
    ) async throws {
        switch proposal.params {
        case .mint(let amount, let recipient, let note):
            // A co-signature that CLOSES the group mint returns the
            // recipient's proof-verified post-mint balance (empty while the
            // action is still awaiting signatures). Persist it so the
            // closing co-signer's local row updates immediately instead of
            // waiting for the next periodic sync.
            let balances = try await wallet.tokenMint(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                recipient: recipient,
                amount: amount,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
            await persistCoSignedBalances(
                balances: balances,
                contractId: contractId,
                tokenPosition: tokenPosition
            )
        case .burn(let amount, _, let note):
            // The burn proposal carries `burnFrom` (the account being
            // debited), but the FFI burns from `identityId` — which
            // for a co-signer is themselves. Platform validates that
            // the signature lines up with the original proposal's
            // burn target on its end, so passing through the
            // co-signer's identity here is correct.
            // A co-signature that CLOSES the group burn returns the
            // caller's proof-verified post-burn balance (empty while the
            // action is still awaiting signatures). This is the call site
            // where that balance is most likely non-empty — persist it so
            // the closing co-signer's local row updates immediately
            // instead of waiting for the next periodic sync.
            let balances = try await wallet.tokenBurn(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                amount: amount,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
            await persistCoSignedBalances(
                balances: balances,
                contractId: contractId,
                tokenPosition: tokenPosition
            )
        case .freeze(let target, let note):
            try await wallet.tokenFreeze(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                frozenIdentityId: target,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
        case .unfreeze(let target, let note):
            try await wallet.tokenUnfreeze(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                frozenIdentityId: target,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
        case .destroyFrozenFunds(let target, _, let note):
            try await wallet.tokenDestroyFrozenFunds(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                frozenIdentityId: target,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
        case .pause(let note):
            try await wallet.tokenPause(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
        case .resume(let note):
            try await wallet.tokenResume(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
        case .setPrice(let price, let note):
            try await wallet.tokenSetPrice(
                identityId: identityId,
                contractId: contractId,
                tokenPosition: tokenPosition,
                pricePerToken: price ?? 0,
                publicNote: note,
                groupAction: mode,
                signer: signer
            )
        case .directPurchase, .other:
            throw NSError(
                domain: "CoSignProposalView",
                code: -1,
                userInfo: [NSLocalizedDescriptionKey: "Proposal type not supported by co-sign UI."]
            )
        }
    }

    /// Persist the proof-verified post-action balance a co-signed group
    /// mint / burn returned into the local `PersistentTokenBalance` row
    /// (keyed by this view's `token`). Empty when the action is still
    /// awaiting signatures → no-op. Best-effort: a persist failure is
    /// logged and swallowed; the periodic sync is the backstop. Mirrors
    /// `TokenBurnActionView.persistBalanceAfterBurn`.
    @MainActor
    private func persistCoSignedBalances(
        balances: [Data: UInt64],
        contractId: Data,
        tokenPosition: UInt16
    ) async {
        guard !balances.isEmpty, let sdk = appState.sdk else { return }
        do {
            try sdk.persistProvenTokenBalances(
                contractId: contractId,
                tokenPosition: tokenPosition,
                tokenRelationshipKey: token.id,
                balances: balances,
                in: modelContext
            )
        } catch {
            print("⚠️ Co-signed balance persist failed: \(error)")
        }
    }

    private func short(_ s: String) -> String {
        guard s.count > 12 else { return s }
        return "\(s.prefix(6))..\(s.suffix(4))"
    }
}
