import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for transferring tokens between identities.
///
/// Inputs: recipient (via `RecipientPickerView`), amount, optional
/// public note. The form refuses to submit unless the recipient is
/// set and the amount is in `(0, balance]`. On success it refreshes
/// the affected local `PersistentTokenBalance` rows — the sender's
/// (always local) and the recipient's (only when the recipient is a
/// local on-device identity) — via `SDK.refreshTokenBalances` so the
/// balance surfaces update immediately, then dismisses. A refresh
/// failure is logged and swallowed: the on-chain transfer already
/// succeeded, and the periodic balance sync remains the backstop.
struct TokenTransferActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity
    /// Balance the parent screen already fetched for this token via
    /// `sdk.getIdentityTokenBalances`. When non-nil, it's the source of
    /// truth; when nil, we fall back to the `PersistentTokenBalance`
    /// row, which may not be populated yet (see file header). Declared
    /// as `var` (not `let`) so the synthesized memberwise init exposes
    /// it with the default of `nil`.
    var initialBalance: UInt64? = nil

    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var appState: AppState
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var recipient: RecipientSelection?
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

            Section("Recipient") {
                if let wallet = managedWallet {
                    RecipientPickerView(
                        selection: $recipient,
                        wallet: wallet,
                        network: identity.network,
                        exclude: identity.identityId
                    )
                } else {
                    Text("The wallet that owns this identity isn't loaded.")
                        .font(.subheadline)
                        .foregroundColor(.red)
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
                            Text("Transfer")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Transfer")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Transfer failed"),
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
        // PersistentTokenBalance stores Int64; we treat it as
        // a UInt64 here (token amounts are non-negative on Platform).
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

    /// Parse the user's input as a decimal number in display units and
    /// scale it to raw on-chain units using `token.decimals`. Without
    /// this, typing "5" against a token with 8 decimals would submit
    /// 5 raw units (0.00000005 of a token) and silently sneak past the
    /// balance check, since the displayed balance is also in display
    /// units.
    private var parsedAmount: UInt64? {
        parseTokenAmount(amountText, decimals: token.decimals)
    }

    private var canSubmit: Bool {
        guard recipient != nil else { return false }
        guard let amount = parsedAmount, amount > 0, amount <= balanceValue else {
            return false
        }
        return managedWallet != nil
    }

    // MARK: - Submit

    private func submit() {
        // Re-check balance at submit time: the user could have spent
        // the underlying tokens between render and tap. Mirror the
        // shape used by `TokenBurnActionView.submit`.
        guard
            let wallet = managedWallet,
            let recipient = recipient,
            let amount = parsedAmount,
            amount > 0,
            amount <= balanceValue
        else {
            submitError = .init(message: "Amount is invalid or exceeds your balance.")
            return
        }

        // Guard before flipping `isSubmitting` so an out-of-range
        // position doesn't leave the spinner stuck.
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
        let recipientId = recipient.identityId
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note

        Task {
            do {
                try await wallet.tokenTransfer(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    recipient: recipientId,
                    amount: amount,
                    publicNote: publicNoteOrNil,
                    signer: signer
                )
                // The transfer landed on chain; refresh the local
                // balance rows the UI observes so the sender's balance
                // drops and a local recipient's balance rises without
                // waiting for the next periodic sync. Resilient: a
                // refresh failure must not turn a successful transfer
                // into a user-visible error.
                await self.refreshBalancesAfterTransfer(
                    senderId: identityId,
                    recipientId: recipientId
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

    /// Refresh the local `PersistentTokenBalance` rows touched by a
    /// just-completed transfer. Always refreshes the sender; also
    /// refreshes the recipient when it's a wallet-owned identity on
    /// this network (so MW-02's "switch to B, verify the tokens
    /// arrived" step works). Best-effort — any failure is logged and
    /// swallowed; the periodic sync is the backstop.
    ///
    /// `@MainActor`-isolated: it reads SwiftData `@Model` instances
    /// (`token`, `identity`) and passes the main-context `modelContext`
    /// to the SDK's `@MainActor` refresh. The blocking network query
    /// runs off-main inside that SDK method.
    @MainActor
    private func refreshBalancesAfterTransfer(
        senderId: Data,
        recipientId: Data
    ) async {
        guard let sdk = appState.sdk else { return }
        guard let position = UInt16(exactly: token.position) else { return }

        // The sender is local by construction; the recipient is
        // included only when a wallet-owned PersistentIdentity row
        // exists for it on this network.
        var identityIds: [Data] = [senderId]
        if recipientId != senderId,
           let recipientRow = PersistentIdentity.fetch(
               in: modelContext,
               identityId: recipientId
           ),
           recipientRow.wallet != nil,
           recipientRow.network == identity.network {
            identityIds.append(recipientId)
        }

        do {
            try await sdk.refreshTokenBalances(
                contractId: token.contractId,
                tokenPosition: position,
                tokenRelationshipKey: token.id,
                identityIds: identityIds,
                in: modelContext
            )
        } catch {
            print("⚠️ Post-transfer balance refresh failed: \(error)")
        }
    }
}
