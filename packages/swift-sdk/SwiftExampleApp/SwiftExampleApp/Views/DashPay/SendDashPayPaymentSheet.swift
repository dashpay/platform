import SwiftUI
import SwiftData
import SwiftDashSDK

/// Lightweight UI model for an established DashPay contact row,
/// consumed by `ContactDetailView` + `SendDashPayPaymentSheet`.
/// The cached DashPay profile fields are resolved separately via
/// `wallet.getDashPayProfile(identityId:)`.
struct DashPayContact: Identifiable {
    let id: Data
    let displayName: String
    let identityId: Data
    let dpnsName: String?
    let note: String?
    let isHidden: Bool

    init(
        id: Data,
        displayName: String,
        identityId: Data,
        dpnsName: String? = nil,
        note: String? = nil,
        isHidden: Bool = false
    ) {
        self.id = id
        self.displayName = displayName
        self.identityId = identityId
        self.dpnsName = dpnsName
        self.note = note
        self.isHidden = isHidden
    }
}

// MARK: - Send payment sheet

/// Modal sheet for sending a Dash payment to an established DashPay
/// contact via the platform-wallet FFI
/// (`ManagedPlatformWallet.sendDashPayPayment`). Rust handles the
/// address derivation (DIP-14) + Core-chain broadcast + recording
/// a `PaymentEntry` on the sender's `ManagedIdentity`; the
/// identity changeset callback (5f5ac06d6) forwards the state
/// update to SwiftData.
struct SendDashPayPaymentSheet: View {
    let senderIdentity: PersistentIdentity
    let contact: DashPayContact
    /// Fires with the 32-byte txid once the transaction has been
    /// broadcast. Parent uses this to refresh its contact state
    /// (recording a payment may auto-establish a contact on the
    /// Rust side if a reciprocal request just came in).
    let onSent: () -> Void

    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.dismiss) private var dismiss

    /// Input in DASH — we convert to duffs before handing off to
    /// the FFI. Keeping the field in DASH makes the units
    /// human-readable (a 0.001 DASH payment is `0.001` in the
    /// field, not `100000`).
    @State private var amountText = ""
    @State private var isSending = false
    @State private var errorMessage: String?
    @State private var successTxid: Data?

    /// Cached recipient profile resolved from the platform-wallet
    /// cache on appear. Empty until the first lookup completes; the
    /// recipient section falls back to the `DashPayContact` fields
    /// until then so the sheet doesn't flicker.
    @State private var recipientProfile: DashPayProfile?
    @State private var recipientDpnsName: String?

    /// Sender's current Core balance (spendable duffs). Pulled from
    /// the Core wallet's lock-free balance on appear so the user
    /// can see what they actually have before submitting. `nil`
    /// while the async fetch is in flight or if the wallet handle
    /// can't be resolved.
    @State private var senderBalanceDuffs: UInt64?

    /// DASH → duffs. Returns nil when the input isn't a parseable
    /// non-negative decimal or overflows `UInt64`.
    private var amountDuffs: UInt64? {
        let trimmed = amountText.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let dashValue = Decimal(string: trimmed), dashValue >= 0 else {
            return nil
        }
        // 1 DASH = 100_000_000 duffs. Decimal multiply then snap
        // to `UInt64`; a negative or overflowed intermediate
        // yields nil.
        let duffsDecimal = dashValue * 100_000_000
        // `NSDecimalNumber(decimal:).uint64Value` truncates on
        // overflow — detect by re-comparing.
        let duffs = NSDecimalNumber(decimal: duffsDecimal).uint64Value
        let roundTrip = Decimal(duffs)
        return roundTrip == duffsDecimal ? duffs : nil
    }

    /// Pretty name to show in the "To" section. Prefers the
    /// DashPay profile's display name, then the identity's DPNS
    /// label, then the contact's stored display name (truncated
    /// hex by default). Recalculated whenever the resolved
    /// profile / DPNS changes.
    private var recipientDisplayName: String {
        if let trimmed = recipientProfile?.displayName?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !trimmed.isEmpty {
            return trimmed
        }
        if let dpns = recipientDpnsName, !dpns.isEmpty {
            return dpns
        }
        return contact.displayName
    }

    /// Subtitle on the recipient row: public message if present,
    /// else DPNS (when the headline is already the profile name),
    /// else the truncated hex id.
    private var recipientSubtitle: String? {
        if let msg = recipientProfile?.publicMessage?
            .trimmingCharacters(in: .whitespacesAndNewlines),
           !msg.isEmpty {
            return msg
        }
        if let dpns = recipientDpnsName,
           recipientProfile?.displayName?.isEmpty == false {
            return dpns
        }
        return String(contact.identityId.toHexString().prefix(20)) + "…"
    }

    /// "1.23456789 DASH" — only rendered when we have a balance
    /// number to show.
    private var senderBalanceText: String? {
        guard let duffs = senderBalanceDuffs else { return nil }
        let dash = Double(duffs) / 100_000_000
        return String(format: "%.8f DASH", dash)
    }

    /// Duffs available for spending, minus the current amount
    /// input. `nil` when either the balance hasn't loaded or the
    /// amount input doesn't parse. Used to flag over-spends in the
    /// validation row + disable the Send button.
    private var exceedsBalance: Bool {
        guard let balance = senderBalanceDuffs,
              let duffs = amountDuffs else {
            return false
        }
        return duffs > balance
    }

    var body: some View {
        NavigationStack {
            Form {
                Section("To") {
                    VStack(alignment: .leading, spacing: 4) {
                        HStack(spacing: 10) {
                            if let url = recipientProfile?.avatarUrl
                                .flatMap({ URL(string: $0) }) {
                                AsyncImage(url: url) { phase in
                                    if let image = phase.image {
                                        image
                                            .resizable()
                                            .aspectRatio(contentMode: .fill)
                                    } else {
                                        Color.blue.opacity(0.15)
                                    }
                                }
                                .frame(width: 32, height: 32)
                                .clipShape(Circle())
                            } else {
                                Circle()
                                    .fill(Color.blue.opacity(0.2))
                                    .frame(width: 32, height: 32)
                                    .overlay(
                                        Text(recipientDisplayName.prefix(1).uppercased())
                                            .font(.headline)
                                            .foregroundColor(.blue)
                                    )
                            }
                            VStack(alignment: .leading, spacing: 2) {
                                Text(recipientDisplayName)
                                    .font(.headline)
                                if let sub = recipientSubtitle {
                                    Text(sub)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                        .lineLimit(1)
                                        .truncationMode(.middle)
                                }
                            }
                        }
                    }
                }

                Section("Amount (DASH)") {
                    // Zero-balance state: once the async balance
                    // load resolves to 0, swap the interactive form
                    // for an explanation instead of an
                    // always-disabled field.
                    if senderBalanceDuffs == 0 {
                        Text("Your balance is 0 DASH — top up your wallet before sending.")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    } else {
                        TextField("0.001", text: $amountText)
                            .keyboardType(.decimalPad)
                            .autocorrectionDisabled()
                            .textInputAutocapitalization(.never)
                            .accessibilityIdentifier("dashpay.send.amount")
                    }
                    if let balanceText = senderBalanceText {
                        HStack {
                            Text("Your balance")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Text(balanceText)
                                .font(.caption)
                                .fontWeight(.medium)
                                .foregroundColor(exceedsBalance ? .red : .secondary)
                        }
                    }
                    if !amountText.isEmpty, amountDuffs == nil {
                        Text("Enter a valid decimal Dash amount")
                            .font(.caption)
                            .foregroundColor(.red)
                    } else if exceedsBalance {
                        Text("Amount exceeds your spendable balance")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }

                // Memo row intentionally absent. DashPay payments
                // are plain Core-chain transactions — there's no
                // on-chain memo slot and no DashPay document type
                // for per-payment notes — so a memo field here
                // would be misleading. `PaymentEntry.memo` on the
                // Rust side is a local-only record the sender's
                // wallet could populate from elsewhere if needed;
                // the payment sheet stays honest by omitting it.

                if let successTxid = successTxid {
                    Section {
                        Text("Sent! txid: \(txidDisplayHex(successTxid).prefix(16))…")
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                } else if let errorMessage = errorMessage {
                    Section {
                        Text(errorMessage)
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                }
            }
            .navigationTitle("Send Dash")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                        .disabled(isSending)
                        .accessibilityIdentifier("dashpay.send.cancel")
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    if isSending {
                        ProgressView()
                    } else {
                        Button("Send") { send() }
                            .disabled(
                                amountDuffs == nil
                                || (amountDuffs ?? 0) == 0
                                || exceedsBalance
                                || senderBalanceDuffs == 0
                            )
                            .accessibilityIdentifier("dashpay.send.confirm")
                    }
                }
            }
            .task {
                await loadRecipientMetadata()
                await loadSenderBalance()
            }
        }
    }

    /// Resolve the recipient's profile + DPNS label from the
    /// platform-wallet cache. Both are in-memory cache reads (no
    /// network roundtrips) — the profile came from
    /// `syncDashPayProfiles` and the DPNS label from any prior
    /// `syncDpnsNames` for that identity. When the cache is empty
    /// we fall back to the hex id display in the computed
    /// properties above.
    ///
    /// We do NOT trigger a network sync here — opening a payment
    /// sheet for every contact would spam the wallet; recipient
    /// profiles refresh via the recurring DashPay sync that feeds
    /// the DashPay tab.
    private func loadRecipientMetadata() async {
        guard let walletId = senderIdentity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            return
        }
        // Recipient is a contact: read the contact-profile cache first, with
        // an own-profile fallback for a recipient that is one of our own
        // identities. A miss leaves the fallback hex-id rendering.
        recipientProfile = dashPayCachedProfile(
            wallet: wallet,
            ownerIdentityId: senderIdentity.identityId,
            contactId: contact.identityId
        )
        do {
            let managed = try wallet.managedIdentity(identityId: contact.identityId)
            let names = (try? managed.getDpnsNames()) ?? []
            recipientDpnsName = names.first
        } catch {
            // The recipient isn't a managed identity on this
            // wallet (they're a contact, not an owned identity).
            // That's the common case; leave DPNS unset.
            recipientDpnsName = nil
        }
    }

    /// Fetch the sender wallet's current Core balance so the
    /// amount row can show "spendable: X DASH" and block submits
    /// that exceed it. Uses the lock-free balance accessor —
    /// atomic reads, no async work.
    private func loadSenderBalance() async {
        guard let walletId = senderIdentity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            senderBalanceDuffs = nil
            return
        }
        do {
            let balance = try wallet.balance()
            senderBalanceDuffs = balance.spendable
        } catch {
            senderBalanceDuffs = nil
        }
    }

    /// Broadcast the payment via the platform-wallet FFI. Memo is
    /// currently passed to the Rust side but the signing path
    /// doesn't embed it in the transaction yet — it's recorded on
    /// the local `PaymentEntry` so the payment-history UI (when
    /// wired) has context.
    private func send() {
        guard let walletId = senderIdentity.wallet?.walletId,
              let wallet = walletManager.wallet(for: walletId) else {
            errorMessage = "No wallet available for this identity"
            return
        }
        guard let duffs = amountDuffs, duffs > 0 else {
            errorMessage = "Amount must be greater than zero"
            return
        }

        isSending = true
        errorMessage = nil

        Task { @MainActor in
            defer { isSending = false }
            do {
                // `memo: nil` — DashPay payments don't carry memos
                // on-chain or via a document, so there's nothing
                // useful to pass. The Rust-side
                // `PaymentEntry.memo` slot stays available for
                // future local-note wiring.
                let (txid, _) = try await wallet.sendDashPayPayment(
                    fromIdentityId: senderIdentity.identityId,
                    toContactIdentityId: contact.identityId,
                    amountDuffs: duffs,
                    memo: nil
                )
                successTxid = txid
                onSent()
                kickDashPaySync(walletManager)
                // Small settle-in before dismissing so the user
                // sees the confirmation row. Mirrors the pattern in
                // `RegisterNameView`.
                try? await Task.sleep(nanoseconds: 1_500_000_000)
                dismiss()
            } catch {
                errorMessage = error.localizedDescription
            }
        }
    }
}
