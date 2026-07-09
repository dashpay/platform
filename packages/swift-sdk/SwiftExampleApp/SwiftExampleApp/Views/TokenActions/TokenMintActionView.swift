import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for minting tokens.
///
/// Inputs: amount, optional recipient (defaults to self), optional
/// public note. Validates `amount > 0` and — when the token has a
/// `maxSupply` configured — that `amount <= maxSupply`. The screen
/// surfaces a group-action banner whenever the caller's authorization
/// to mint is gated on a `MainGroup` / `Group:<position>` rule, in
/// which case the submission is sent as a group-action proposal
/// (`GroupActionMode.propose`).
struct TokenMintActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var amountText: String = ""
    @State private var publicNote: String = ""
    @State private var mintToSelf: Bool = true
    @State private var recipient: RecipientSelection?
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
                if let maxSupplyRaw = parsedMaxSupply {
                    LabeledContent(
                        "Max supply",
                        value: formatTokenAmount(maxSupplyRaw, decimals: token.decimals)
                    )
                }
            }

            if !token.mintingAllowChoosingDestination {
                Section {
                    Label(
                        "This token does not allow choosing a mint destination — tokens are issued to the configured recipient.",
                        systemImage: "info.circle"
                    )
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                }
            }

            if let groupPosition = inferredGroupPosition {
                Section("Group action") {
                    Label(
                        "This mint requires a group action.",
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

            Section("Recipient") {
                // Label tracks what the toggle actually does:
                // - When the contract permits a runtime destination,
                //   "Mint to self" is honest (toggle off → pick recipient).
                // - When it doesn't, the toggle is force-on/disabled and
                //   tokens go to the contract's `newTokensDestinationIdentity`,
                //   not the caller — the literal "Mint to self" lies.
                Toggle(
                    token.mintingAllowChoosingDestination
                        ? "Mint to self"
                        : "Use configured destination",
                    isOn: $mintToSelf
                )
                .disabled(!token.mintingAllowChoosingDestination)
                if !mintToSelf {
                    if let wallet = managedWallet {
                        RecipientPickerView(
                            selection: $recipient,
                            wallet: wallet,
                            network: identity.network,
                            exclude: nil
                        )
                    } else {
                        Text("The wallet that owns this identity isn't loaded.")
                            .font(.subheadline)
                            .foregroundColor(.red)
                    }
                }
            }

            Section("Amount") {
                TextField("Amount", text: $amountText)
                    .keyboardType(.decimalPad)
                if let amountValue = parsedAmount,
                   let maxSupplyValue = parsedMaxSupply,
                   amountValue > maxSupplyValue {
                    Text("Amount exceeds the configured max supply.")
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
                            Text("Mint")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Mint")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Mint failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
        .onAppear {
            // Token forbids choosing the destination -> force mint-to-self.
            if !token.mintingAllowChoosingDestination {
                mintToSelf = true
            }
        }
    }

    // MARK: - Derived state

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    /// User input is in display units; scale to raw on-chain units so
    /// the `amount > maxSupply` check (both raw u64) is meaningful.
    private var parsedAmount: UInt64? {
        parseTokenAmount(amountText, decimals: token.decimals)
    }

    /// `token.maxSupply` is a string-encoded raw u64.
    private var parsedMaxSupply: UInt64? {
        guard let raw = token.maxSupply else { return nil }
        return UInt64(raw)
    }

    private var canSubmit: Bool {
        guard let amount = parsedAmount, amount > 0 else { return false }
        if let maxSupply = parsedMaxSupply, amount > maxSupply {
            return false
        }
        if !mintToSelf, recipient == nil { return false }
        return managedWallet != nil
    }

    /// If the mint rule on this token is gated on a group, return the
    /// group position the caller should propose under. Otherwise nil
    /// (single-signer mint). Mirrors Wave 1's burn evaluator.
    private var inferredGroupPosition: Int? {
        guard let rule = token.manualMintingRules else { return nil }
        let authorized = rule.authorizedToMakeChange

        if authorized == AuthorizedActionTakers.mainGroup.rawValue {
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
        guard
            let wallet = managedWallet,
            let amount = parsedAmount,
            amount > 0
        else {
            submitError = .init(message: "Amount is invalid.")
            return
        }

        let recipientId: Data?
        if mintToSelf {
            if token.mintingAllowChoosingDestination {
                // The contract permits a caller-supplied destination,
                // so be explicit: pass our own identity id. Passing nil
                // would defer to `newTokensDestinationIdentity`, which
                // may be unset — the FFI then surfaces "Destination
                // identity for minting not set" instead of minting to
                // self.
                recipientId = identity.identityId
            } else {
                // The contract forbids overriding the destination; any
                // non-nil recipient would be rejected. Defer to the
                // configured `newTokensDestinationIdentity` via nil.
                recipientId = nil
            }
        } else {
            guard let chosen = recipient else {
                submitError = .init(message: "Pick a recipient or enable mint-to-self.")
                return
            }
            recipientId = chosen.identityId
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
        // Grab the token relationship key while still on the main actor so
        // the post-mint persist doesn't read a SwiftData model from inside
        // the Task. (`contractId` above is already `token.contractId`.)
        let tokenRelationshipKey = token.id

        Task {
            do {
                // The mint's broadcast result already carries the
                // proof-verified post-mint balance of the recipient —
                // persist it straight into the local row the UI observes so
                // a mint-to-self shows up (and Transfer / Burn unlock)
                // immediately, without waiting for the next periodic sync
                // and with no extra round-trip.
                let balances = try await wallet.tokenMint(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    recipient: recipientId,
                    amount: amount,
                    publicNote: publicNoteOrNil,
                    groupAction: groupAction,
                    signer: signer
                )
                await self.persistBalancesAfterMint(
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

    /// Persist the proof-verified post-mint balance the mint returned into
    /// the local `PersistentTokenBalance` row. Keyed by the recipient the
    /// FFI reports, so a mint-to-self updates the holder's row (and the
    /// Transfer / Burn forms, which gate on the local balance, immediately
    /// see it). A group-action proposal / history-tracking token returns an
    /// empty map and nothing is written. Best-effort — any failure is
    /// logged and swallowed; the periodic sync is the backstop.
    ///
    /// `@MainActor`-isolated: writes the main-context `modelContext` via the
    /// SDK's `@MainActor` persist helper. No network round-trip.
    @MainActor
    private func persistBalancesAfterMint(
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
            print("⚠️ Post-mint balance persist failed: \(error)")
        }
    }
}
