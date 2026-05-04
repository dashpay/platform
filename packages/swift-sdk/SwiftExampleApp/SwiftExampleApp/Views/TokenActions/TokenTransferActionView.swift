import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for transferring tokens between identities.
///
/// Inputs: recipient (via `RecipientPickerView`), amount, optional
/// public note. The form refuses to submit unless the recipient is
/// set and the amount is in `(0, balance]`. On success, the view
/// dismisses; the parent screen relies on the next balance sync to
/// pick up the change (Wave 1 doesn't push a synchronous local
/// `PersistentTokenBalance` update — that's a TODO once a token-sync
/// helper exists in `platform-wallet-ffi`).
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
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var recipient: RecipientSelection?
    @State private var amountText: String = ""
    @State private var publicNote: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?

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

    private var matchingBalance: PersistentTokenBalance? {
        identity.tokenBalances.first { tb in
            tb.tokenId == token.id.toBase58String()
                || tb.token?.id == token.id
        }
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
                await MainActor.run {
                    self.isSubmitting = false
                    self.dismiss()
                }
            } catch {
                await MainActor.run {
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }
}

/// Parse a user-entered amount in display units (e.g. "5" or "4.25"
/// for a token with 8 decimals) into raw on-chain units. Accepts both
/// "." and "," as the decimal separator so users in either locale can
/// type naturally. Returns nil for empty / unparseable / negative /
/// out-of-range input.
///
/// Excess fractional digits beyond `decimals` are truncated (rounded
/// down) rather than rounded — silently rounding *up* would let the
/// user submit slightly more than they typed, which would surprise on
/// a balance edge.
fileprivate func parseTokenAmount(_ text: String, decimals: Int) -> UInt64? {
    let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !trimmed.isEmpty else { return nil }

    let normalized = trimmed.replacingOccurrences(of: ",", with: ".")
    guard let entered = Decimal(string: normalized), entered >= 0 else {
        return nil
    }

    let dec = max(0, decimals)
    let multiplier = pow(Decimal(10), dec)
    var scaled = entered * multiplier
    var rounded = Decimal()
    NSDecimalRound(&rounded, &scaled, 0, .down)

    if rounded < 0 || rounded > Decimal(UInt64.max) { return nil }
    return (rounded as NSDecimalNumber).uint64Value
}

/// Format a raw u64 token amount with the given decimals using
/// `Decimal` (not `Double`) so high-decimal tokens with large balances
/// don't lose precision. Mirrors `IdentityTokenRow.formattedBalance`
/// in `IdentityDetailView`.
fileprivate func formatTokenAmount(_ raw: UInt64, decimals: Int) -> String {
    let dec = max(0, decimals)
    let rawDecimal = Decimal(raw)
    let divisor = pow(Decimal(10), dec)
    let scaled = divisor == 0 ? rawDecimal : (rawDecimal / divisor)
    let formatter = NumberFormatter()
    formatter.numberStyle = .decimal
    formatter.maximumFractionDigits = dec
    formatter.minimumFractionDigits = 0
    formatter.usesGroupingSeparator = true
    return formatter.string(from: scaled as NSNumber) ?? "\(raw)"
}
