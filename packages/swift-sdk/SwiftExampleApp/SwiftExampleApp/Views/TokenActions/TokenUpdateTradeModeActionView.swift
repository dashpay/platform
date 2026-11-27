import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for changing a token's marketplace trade mode, the configuration
/// that says whether the token may be traded through Platform. It changes the
/// configuration only: placing and filling orders are separate transitions.
///
/// The submission is one `TokenConfigChange.marketplaceTradeMode`, which Rust
/// turns into `TokenConfigurationChangeItem::MarketplaceTradeMode`. Platform
/// decides whether the token may become tradeable (it must be transferable,
/// and the network at protocol version 16 or later; setting `NotTradeable`
/// has neither condition); this form does not check either and shows
/// Platform's refusal as it arrives.
///
/// Single-signer only: the update is sent without group information, and
/// Platform decides whether the signing identity alone may make the change.
/// When the rule needs more signatures than this one, the update is refused:
/// proposing and co-signing a configuration update as a group is not wired
/// in this app yet.
struct TokenUpdateTradeModeActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var newTradeMode: TokenTradeMode
    @State private var publicNote: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?
    /// Shown after a confirmed update whose contract could not be stored.
    @State private var refreshNotice: AlertMessage?
    /// Set when an update was broadcast and its outcome is unknown: it may
    /// have taken effect, so the form does not offer to send another one.
    @State private var outcomeUnknown: AlertMessage?
    @State private var submissionMayHaveLanded: Bool = false
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance.
    @State private var submitGeneration: Int = 0

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    init(token: PersistentToken, identity: PersistentIdentity) {
        self.token = token
        self.identity = identity
        // Start on the mode the token does not have.
        _newTradeMode = State(
            initialValue: token.tradeMode == .tradeable ? .notTradeable : .tradeable
        )
    }

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
                LabeledContent("Current trade mode", value: token.tradeMode.displayName)
                    .accessibilityIdentifier("tokenTradeMode.current")
            }

            Section {
                Text("The trade mode is the token's configuration for trading through Platform; changing it places no order. Platform refuses making a token tradeable if the token is not transferable or the network does not support token trading yet.")
                    .font(.subheadline)
            }

            Section {
                Text("This form sends a single-signer update, and Platform decides whether this identity alone may make the change. If the change needs more signatures, Platform refuses it: group proposals for configuration changes are not supported in this app yet.")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .accessibilityIdentifier("tokenTradeMode.singleSignerNote")
            }

            Section("New trade mode") {
                Picker("Trade mode", selection: $newTradeMode) {
                    ForEach(TokenTradeMode.allCases, id: \.self) { mode in
                        Text(mode.displayName).tag(mode)
                    }
                }
                .pickerStyle(.segmented)
                .accessibilityIdentifier("tokenTradeMode.picker")
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
                            Text("Update Trade Mode")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(managedWallet == nil || isSubmitting || submissionMayHaveLanded)
                .accessibilityIdentifier("tokenTradeMode.submitButton")
            }
        }
        .navigationTitle("Update Trade Mode")
        .navigationBarTitleDisplayMode(.inline)
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Update failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
        .alert(item: $outcomeUnknown) { msg in
            Alert(
                title: Text("Update not confirmed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
        .alert(item: $refreshNotice) { msg in
            Alert(
                title: Text("Trade mode updated"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK")) { dismiss() }
            )
        }
    }

    // MARK: - Derived state

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    // MARK: - Submit

    private func submit() {
        guard let wallet = managedWallet else {
            submitError = .init(message: "The wallet that owns this identity isn't loaded.")
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
        let change = TokenConfigChange.marketplaceTradeMode(newTradeMode)
        let note = publicNote.trimmingCharacters(in: .whitespacesAndNewlines)
        let publicNoteOrNil: String? = note.isEmpty ? nil : note

        Task {
            do {
                let outcome = try await wallet.tokenUpdateConfig(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    change: change,
                    publicNote: publicNoteOrNil,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    // Rust says whether the update took effect and hands back
                    // the contract it produced; storing that shows the new
                    // mode.
                    if let reason = ContractDownloader.storeConfigUpdateOutcome(
                        outcome,
                        contractId: contractId,
                        modelContext: modelContext
                    ) {
                        self.refreshNotice = .init(
                            message: "The update was confirmed, but the updated contract could not be "
                                + "saved on this device (\(reason)). It keeps showing the old trade "
                                + "mode until the contract is refreshed."
                        )
                    } else {
                        self.dismiss()
                    }
                }
            } catch PlatformWalletError.transactionBroadcastUnconfirmed(let detail) {
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    // The update was broadcast and may have taken effect, so
                    // sending another one could pay for it twice.
                    self.submissionMayHaveLanded = true
                    self.outcomeUnknown = .init(
                        message: "The update was sent, but its confirmation did not arrive (\(detail)). "
                            + "It may have taken effect. Refresh the contract and check the token's "
                            + "trade mode before sending another update."
                    )
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
