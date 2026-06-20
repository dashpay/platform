import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for burning tokens.
///
/// Inputs: amount, optional public note. Validates `amount > 0` and
/// `amount <= balance`. Surfaces a group-action banner whenever the
/// caller's authorization to burn is gated on a `MainGroup` /
/// `Group:<position>` rule — in that case the submission is sent as a
/// group-action proposal (`GroupActionMode.propose`).
struct TokenBurnActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity
    /// Balance the parent screen already fetched for this token via
    /// `sdk.getIdentityTokenBalances`. When non-nil, it's the source of
    /// truth; when nil, we fall back to the `PersistentTokenBalance`
    /// row, which may not be populated yet. Declared as `var` (not
    /// `let`) so the synthesized memberwise init exposes it with the
    /// default of `nil`.
    var initialBalance: UInt64? = nil

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var amountText: String = ""
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

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
                LabeledContent("Balance", value: balanceDisplay)
            }

            Section {
                Label(
                    "Burning is irreversible — the tokens are permanently removed from circulation.",
                    systemImage: "exclamationmark.triangle.fill"
                )
                .font(.subheadline)
                .foregroundColor(.orange)
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This burn requires a group action.",
                        systemImage: "person.3.fill"
                    )
                    .font(.subheadline)
                    LabeledContent("Group position", value: "\(groupPosition)")
                    Text(
                        "Submitting will propose a new group action. Other group members "
                        + "must sign before it broadcasts."
                    )
                    .font(.caption)
                    .foregroundColor(.secondary)
                }
            }

            Section("Amount") {
                TextField("Amount", text: $amountText)
                    .keyboardType(.decimalPad)
                if let amountValue = parsedAmount, amountValue > balanceValue {
                    Text("Amount exceeds your balance.")
                        .font(.caption)
                        .foregroundColor(.red)
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
                            Text("Burn")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .tint(.red)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Burn")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Burn failed"),
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

    private var balanceValue: UInt64 {
        if let initialBalance { return initialBalance }
        guard let balance = matchingBalance else { return 0 }
        return balance.balance < 0 ? 0 : UInt64(balance.balance)
    }

    private var balanceDisplay: String {
        if let initialBalance {
            return formatTokenAmount(initialBalance, decimals: token.decimals)
        }
        guard let balance = matchingBalance else { return "0" }
        return balance.displayBalance
    }

    /// Match against the SwiftData relationship key. The earlier
    /// `tb.tokenId == token.id.toBase58String()` arm of this matcher
    /// was always false: `tb.tokenId` holds the canonical on-chain
    /// token id while `token.id` is a `contractId + position` SwiftData
    /// uniqueness key.
    private var matchingBalance: PersistentTokenBalance? {
        identity.tokenBalances.first { $0.token?.id == token.id }
    }

    /// See `parseTokenAmount` — input is in display units, scaled to
    /// raw on-chain units by `token.decimals` before validation.
    private var parsedAmount: UInt64? {
        parseTokenAmount(amountText, decimals: token.decimals)
    }

    private var canSubmit: Bool {
        guard let amount = parsedAmount, amount > 0, amount <= balanceValue else {
            return false
        }
        return managedWallet != nil
    }

    /// If the burn rule on this token is gated on a group, return the
    /// group position the caller should propose under. Otherwise nil
    /// (single-signer burn).
    ///
    /// Mirrors the resolver logic in `TokenActionEvaluator` — we only
    /// look at the explicit `MainGroup` / `Group:<position>` cases.
    /// More exotic cases land here as nil for Wave 1; the underlying
    /// `_with_signer` path will reject them server-side and surface a
    /// readable error.
    private var inferredGroupPosition: Int? {
        guard let rule = token.manualBurningRules else { return nil }
        let authorized = rule.authorizedToMakeChange

        if authorized == AuthorizedActionTakers.mainGroup.rawValue {
            // Drop positions that don't fit in UInt16 — the FFI takes
            // u16 and we don't want to surprise callers with a trap.
            guard let main = token.mainControlGroupPosition,
                  UInt16(exactly: main) != nil
            else { return nil }
            return main
        }
        if authorized.hasPrefix("Group:"),
           let pos = Int(authorized.dropFirst("Group:".count)),
           UInt16(exactly: pos) != nil {
            return pos
        }
        return nil
    }

    // MARK: - Submit

    private func submit() {
        // Re-check balance at submit time: the user could have spent
        // the underlying tokens between render and tap.
        guard
            let wallet = managedWallet,
            let amount = parsedAmount,
            amount > 0,
            amount <= balanceValue
        else {
            submitError = .init(message: "Amount is invalid or exceeds your balance.")
            return
        }

        guard let position = UInt16(exactly: token.position) else {
            submitError = .init(message: "Invalid token position.")
            return
        }

        let groupAction: GroupActionMode
        if let rawGroupPosition = inferredGroupPosition {
            guard let groupPosition = UInt16(exactly: rawGroupPosition) else {
                submitError = .init(message: "Group position is out of range.")
                return
            }
            groupAction = .propose(position: groupPosition)
        } else {
            groupAction = .none
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note

        // Capture the values the persist step needs off the @Model
        // instance now, on the main actor.
        let tokenRelationshipKey = token.id

        Task {
            do {
                // The burn's broadcast result already carries the
                // acting identity's proof-verified remaining balance —
                // persist it straight into the local row so the burned
                // amount disappears from the UI without waiting for the
                // next periodic sync, and with no extra round-trip.
                // (When the burn was sent as a group-action proposal
                // nothing is burned until the group signs, so the FFI
                // returns an empty map and the persist is a no-op.)
                // Best-effort: a persist failure must not turn a
                // successful burn into a user-visible error.
                let balances = try await wallet.tokenBurn(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    amount: amount,
                    publicNote: publicNoteOrNil,
                    groupAction: groupAction,
                    signer: signer
                )
                await self.persistBalanceAfterBurn(
                    balances: balances,
                    contractId: contractId,
                    tokenPosition: position,
                    tokenRelationshipKey: tokenRelationshipKey
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

    /// Persist the proof-verified post-burn balance the burn returned
    /// into the acting identity's local `PersistentTokenBalance` row.
    /// Best-effort — any failure is logged and swallowed; the periodic
    /// sync is the backstop.
    ///
    /// `@MainActor`-isolated: writes the main-context `modelContext` via
    /// the SDK's `@MainActor` persist helper. No network round-trip.
    @MainActor
    private func persistBalanceAfterBurn(
        balances: [Data: UInt64],
        contractId: Data,
        tokenPosition: UInt16,
        tokenRelationshipKey: Data
    ) async {
        guard let sdk = appState.sdk else { return }
        do {
            try sdk.persistProvenTokenBalances(
                contractId: contractId,
                tokenPosition: tokenPosition,
                tokenRelationshipKey: tokenRelationshipKey,
                balances: balances,
                in: modelContext
            )
        } catch {
            print("⚠️ Post-burn balance persist failed: \(error)")
        }
    }
}
