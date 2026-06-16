import SwiftUI
import SwiftData
import SwiftDashSDK

struct SendTransactionView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var shieldedService: ShieldedService
    let wallet: PersistentWallet

    @StateObject private var viewModel: SendViewModel

    /// Drives the camera QR scanner sheet launched from the recipient row.
    @State private var showQRScanner = false

    @Environment(\.modelContext) private var modelContext

    /// BLAST-synced platform-address balances for this wallet —
    /// same source `WalletDetailView` reads to populate its
    /// "Platform Balance" row. Without these the send screen used
    /// to fall through to `wallet.identities.balance` only, which
    /// is empty for wallets that hold credits at platform
    /// addresses (e.g. faucet-funded Platform Payment accounts)
    /// rather than at registered identities.
    @Query private var addressBalances: [PersistentPlatformAddress]

    /// Persisted BLAST sync watermarks — used to distinguish
    /// "BLAST hasn't synced yet, fall back to identities" from
    /// "BLAST synced and there genuinely are no platform-address
    /// credits".
    @Query private var syncStates: [PersistentPlatformAddressesSyncState]

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        _viewModel = StateObject(wrappedValue: SendViewModel(network: wallet.network ?? .testnet))
        let walletId = wallet.walletId
        let walletNetworkRaw = (wallet.network ?? .testnet).rawValue
        _addressBalances = Query(
            filter: #Predicate<PersistentPlatformAddress> { $0.walletId == walletId }
        )
        _syncStates = Query(
            filter: #Predicate<PersistentPlatformAddressesSyncState> {
                $0.networkRaw == walletNetworkRaw
            }
        )
    }

    var body: some View {
        // Snapshot Core balance once per render — `coreBalance` goes
        // through a blocking FFI call (`accountBalances(for:)`); the
        // prior shape re-evaluated it for the summary row, the source
        // list, the per-source balance, and `availableSources`,
        // hitting the FFI repeatedly on a typing-heavy form.
        let coreBalance = coreBalanceSnapshot()
        let sources = availableSources(coreBalance: coreBalance)
        return NavigationStack {
            Form {
                // Recipient
                Section("Recipient") {
                    HStack(spacing: 8) {
                        TextField("Recipient Address", text: $viewModel.recipientAddress)
                            .textInputAutocapitalization(.never)
                            .autocorrectionDisabled()
                        Button {
                            showQRScanner = true
                        } label: {
                            Image(systemName: "qrcode.viewfinder")
                                .font(.title3)
                                .foregroundColor(.accentColor)
                        }
                        // `.borderless` is REQUIRED: the default button
                        // style inside a Form makes the whole row tappable,
                        // which would swallow taps on the text field.
                        .buttonStyle(.borderless)
                        .accessibilityLabel("Scan recipient address QR code")
                    }

                    if !viewModel.recipientAddress.isEmpty {
                        AddressTypeBadge(type: viewModel.detectedAddressType)
                    }
                }

                // Amount
                Section("Amount") {
                    HStack {
                        TextField("0.00000000", text: $viewModel.amountString)
                            .keyboardType(.decimalPad)
                        Text("DASH")
                            .foregroundColor(.secondary)
                    }

                    VStack(alignment: .leading, spacing: 4) {
                        BalanceInfoRow(
                            label: "Core:",
                            amount: coreBalance,
                            unit: .duffs,
                            color: .green
                        )
                        BalanceInfoRow(
                            label: "Shielded:",
                            amount: shieldedBalance,
                            unit: .credits,
                            color: .purple
                        )
                        BalanceInfoRow(
                            label: "Platform:",
                            amount: platformBalance,
                            unit: .credits,
                            color: .blue
                        )
                    }
                }

                // Additional recipients (Core → Core only). A standard
                // L1 tx can pay any number of outputs in one transaction;
                // the Rust coin-selector builds the multi-output tx, so
                // this is purely extra address/amount input rows. Hidden
                // for every other flow — shield / platform / unshield all
                // remain single-recipient. The primary row above still
                // drives `detectedFlow`, so this section only appears once
                // that row resolves to a Core address with a Core source.
                if viewModel.detectedFlow == .coreToCore {
                    additionalRecipientsSection
                }

                // Memo (shielded → shielded only). The on-chain note
                // carries an optional 32-byte UTF-8 memo. Gate on the
                // flow, not the recipient type: an Orchard recipient
                // with a Platform source is the self-shield path, which
                // has no memo parameter — showing the field there would
                // silently drop the text. Count UTF-8 bytes (not
                // characters) so the limit matches Rust.
                if viewModel.detectedFlow == .shieldedToShielded {
                    Section("Memo (optional)") {
                        TextField("Note for the recipient", text: $viewModel.memoText)
                            .textInputAutocapitalization(.sentences)
                            .autocorrectionDisabled()
                        HStack {
                            Spacer()
                            Text("\(viewModel.memoByteCount)/\(SendViewModel.memoByteLimit) bytes")
                                .font(.caption)
                                .foregroundColor(viewModel.isMemoOverLimit ? .red : .secondary)
                        }
                    }
                }

                // Fund Source
                if !sources.isEmpty {
                    Section("Send From") {
                        ForEach(sources) { source in
                            Button {
                                viewModel.selectedSource = source
                                viewModel.updateFlow()
                            } label: {
                                HStack {
                                    Image(systemName: source.iconName)
                                        .foregroundColor(source.color)
                                        .frame(width: 24)
                                    Text(source.rawValue)
                                        .foregroundColor(.primary)
                                    Spacer()
                                    Text(formatBalance(
                                        balance(for: source, coreBalance: coreBalance),
                                        unit: unit(for: source)
                                    ))
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                    if viewModel.selectedSource == source {
                                        Image(systemName: "checkmark")
                                            .foregroundColor(.accentColor)
                                    }
                                }
                            }
                        }
                    }
                }

                // Transaction Type
                if let flow = viewModel.detectedFlow {
                    Section("Transaction Type") {
                        HStack {
                            Image(systemName: flow.iconName)
                                .foregroundColor(flowColor(for: flow))
                            Text(flow.displayName)
                                .fontWeight(.medium)
                        }

                        if let fee = viewModel.estimatedFee {
                            HStack {
                                Text("Estimated Fee:")
                                Spacer()
                                Text("~\(formatBalance(fee, unit: feeUnit(for: flow)))")
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                }

                // Per-output summary (Core → Core, multi-output only).
                // `coreRecipients` is the same ordered, fully-validated
                // list `executeSend` marshals, so the summary can't show
                // a row the send wouldn't include. Only rendered with >1
                // output — the single-output case is already covered by
                // the "Transaction Type" / "Estimated Fee" section above.
                if viewModel.detectedFlow == .coreToCore,
                   let outputs = viewModel.coreRecipients, outputs.count > 1 {
                    coreOutputsSummarySection(outputs: outputs)
                }

            }
            .navigationTitle("Send Dash")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") { dismiss() }
                }
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Send") {
                        Task {
                            guard let sdk = platformState.sdk else { return }
                            // Look up the managed wallet by the
                            // `PersistentWallet` we were handed,
                            // not the "active" manager slot —
                            // the Rust manager holds all wallets
                            // and this view's `wallet` may not
                            // be the one that was last created.
                            let managed = walletManager.wallet(for: wallet.walletId)
                            let coreWallet = try? managed?.coreWallet()
                            let platformAddressWallet = try? managed?.platformAddressWallet()
                            // Pick the account holding the platform
                            // balance. Most wallets have a single
                            // PlatformPayment account (index 0);
                            // fallback handles that case too. Scope to
                            // key class 0 because the Rust transfer path
                            // resolves the source via
                            // `platform_payment_managed_account_at_index`
                            // (key class 0); picking an index from a
                            // non-key-class-0 sibling account would tell
                            // Rust to spend a key-class-0 account that may
                            // be empty at that index.
                            let senderAccountIndex = addressBalances
                                .filter { $0.account?.keyClass == 0 }
                                .first(where: { $0.balance > 0 })?
                                .accountIndex ?? 0
                            // Input selection and surplus handling are owned
                            // by the Rust Auto path (surplus stays on the
                            // source addresses in the credit-balance model),
                            // so there's no change address to pick here.
                            let signer = KeychainSigner(
                                modelContainer: modelContext.container
                            )
                            await viewModel.executeSend(
                                sdk: sdk,
                                walletManager: walletManager,
                                shieldedService: shieldedService,
                                platformState: platformState,
                                wallet: wallet,
                                coreWallet: coreWallet,
                                platformAddressWallet: platformAddressWallet,
                                signer: signer,
                                senderAccountIndex: senderAccountIndex,
                                modelContext: modelContext
                            )
                        }
                    }
                    .disabled(!viewModel.canSend)
                }
            }
            .disabled(viewModel.isSending)
            .overlay {
                if viewModel.isSending {
                    ProgressView("Sending...")
                        .padding()
                        .background(Color.gray.opacity(0.9))
                        .cornerRadius(10)
                }
            }
            .alert("Error", isPresented: .constant(viewModel.error != nil)) {
                Button("OK") { viewModel.error = nil }
            } message: {
                if let error = viewModel.error {
                    Text(error)
                }
            }
            .alert("Success", isPresented: .constant(viewModel.successMessage != nil)) {
                Button("Done") { dismiss() }
            } message: {
                if let msg = viewModel.successMessage {
                    Text(msg)
                }
            }
            .onChange(of: viewModel.detectedAddressType) { _, _ in
                autoSelectSource()
            }
            .sheet(isPresented: $showQRScanner) {
                // Same network the view model was built with
                // (`wallet.network ?? .testnet`) so the scanner validates
                // against the wallet's chain. Assigning `recipientAddress`
                // triggers the view model's `didSet` address-type
                // detection; the amount is only adopted when the user
                // hasn't already typed one.
                QRScannerView(network: wallet.network ?? .testnet) { payment in
                    viewModel.recipientAddress = payment.address
                    if let amount = payment.amount, viewModel.amountString.isEmpty {
                        viewModel.amountString = amount
                    }
                }
            }
        }
    }

    // MARK: - Multi-recipient sections (Core → Core)

    /// The extra-output input rows plus the "Add recipient" button.
    /// Split out of `body` so the type-checker doesn't have to solve the
    /// whole Form in one pass — the per-row editor is a separate
    /// `CoreRecipientRow` subview for the same reason. Iterates over a
    /// `$`-binding so each row's edits flow straight back into
    /// `viewModel.additionalCoreRecipients`.
    @ViewBuilder
    private var additionalRecipientsSection: some View {
        Section("Additional Recipients") {
            ForEach($viewModel.additionalCoreRecipients) { $recipient in
                CoreRecipientRow(
                    recipient: $recipient,
                    network: wallet.network ?? .testnet,
                    onRemove: { viewModel.removeCoreRecipient(recipient.id) }
                )
            }

            Button {
                viewModel.addCoreRecipient()
            } label: {
                Label("Add recipient", systemImage: "plus.circle.fill")
            }
            // `.borderless` so only the label is the tap target inside the
            // Form row, matching the QR button on the primary recipient.
            .buttonStyle(.borderless)
        }
    }

    /// Per-output breakdown + Total + Fee for a multi-output Core send.
    /// `outputs` is the already-validated `coreRecipients` list, so each
    /// row's amount is a real duffs value. Total is the sum of outputs;
    /// the fee reuses `viewModel.estimatedFee` rendered in the
    /// `.coreToCore` fee unit (duffs per `feeUnit(for:)`).
    @ViewBuilder
    private func coreOutputsSummarySection(
        outputs: [(address: String, amountDuffs: UInt64)]
    ) -> some View {
        Section("Outputs") {
            ForEach(Array(outputs.enumerated()), id: \.offset) { _, output in
                HStack {
                    Text(abbreviatedAddress(output.address))
                        .font(.caption)
                        .foregroundColor(.secondary)
                        .lineLimit(1)
                        .truncationMode(.middle)
                    Spacer()
                    Text(formatBalance(output.amountDuffs, unit: .duffs))
                        .font(.caption)
                }
            }

            HStack {
                Text("Total:")
                    .fontWeight(.medium)
                Spacer()
                Text(formatBalance(viewModel.coreSendTotalDuffs, unit: .duffs))
                    .fontWeight(.medium)
            }

            if let fee = viewModel.estimatedFee {
                HStack {
                    Text("Estimated Fee:")
                    Spacer()
                    // coreToCore fee is denominated in duffs (see
                    // feeUnit(for:)); reuse the same formatter as the
                    // single-output fee row.
                    Text("~\(formatBalance(fee, unit: .duffs))")
                        .foregroundColor(.secondary)
                }
            }
        }
    }

    /// Middle-truncate a Core address for the compact summary rows so a
    /// long base58 string doesn't blow out the row width. Short addresses
    /// (shouldn't happen for valid Core addresses, but be safe) are shown
    /// whole.
    private func abbreviatedAddress(_ address: String) -> String {
        guard address.count > 16 else { return address }
        return "\(address.prefix(8))…\(address.suffix(6))"
    }

    // MARK: - Computed

    /// Spendable Core balance, summed from Rust's in-memory per-account
    /// totals. The persisted `PersistentWallet.balanceConfirmed` field
    /// was removed; `accountBalances(for:)` is now the canonical
    /// source (same path `BalanceCardView` uses). Exposed as a
    /// function rather than a computed property so callers can
    /// snapshot once per render and thread the value through.
    private func coreBalanceSnapshot() -> UInt64 {
        walletManager.accountBalances(for: wallet.walletId)
            .reduce(0) { $0 + $1.confirmed }
    }

    private var shieldedBalance: UInt64 {
        shieldedService.shieldedBalance
    }

    /// Mirrors `WalletDetailView.platformBalance`: BLAST-synced
    /// address balances are the canonical source once a sync has
    /// landed; before that, fall back to summing identity credits
    /// so a freshly-restored wallet still shows something
    /// approximate.
    private var platformBalance: UInt64 {
        let blastBalance = addressBalances.reduce(UInt64(0)) { $0 + $1.balance }
        let hasSynced = syncStates.first.map { $0.syncHeight > 0 || $0.syncTimestamp > 0 }
            ?? false
        if blastBalance > 0 || hasSynced {
            return blastBalance
        }
        return wallet.identities.reduce(UInt64(0)) { $0 + UInt64(bitPattern: $1.balance) }
    }

    private func availableSources(coreBalance: UInt64) -> [FundSource] {
        viewModel.availableSources(
            coreBalance: coreBalance,
            shieldedBalance: shieldedBalance,
            platformBalance: platformBalance
        )
    }

    private func balance(for source: FundSource, coreBalance: UInt64) -> UInt64 {
        switch source {
        case .core: return coreBalance
        case .shielded: return shieldedBalance
        case .platform: return platformBalance
        }
    }

    /// Auto-select the first available source when address type changes.
    /// Snapshots `coreBalance` once for the duration of this call so
    /// the underlying FFI accessor isn't hit twice.
    private func autoSelectSource() {
        let coreBalance = coreBalanceSnapshot()
        if let first = availableSources(coreBalance: coreBalance).first {
            viewModel.selectedSource = first
            viewModel.updateFlow()
        }
    }

    // MARK: - Helpers

    private func flowColor(for flow: SendFlow) -> Color {
        switch flow {
        case .coreToCore: return .green
        case .coreToShielded: return .purple
        case .platformToPlatform: return .blue
        case .platformToShielded: return .purple
        case .shieldedToShielded: return .purple
        case .shieldedToPlatform: return .blue
        case .shieldedToCore: return .green
        }
    }

    /// Format a `UInt64` balance for display.
    ///
    /// `unit` controls the divisor — Core/duffs are 1e8 per DASH,
    /// Platform/shielded credits are 1e11 per DASH. Mixing the two
    /// would over-report Platform balances by 1000×.
    private func formatBalance(_ amount: UInt64, unit: SendBalanceUnit = .duffs) -> String {
        let dash = Double(amount) / unit.dashDivisor
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }

    private func unit(for source: FundSource) -> SendBalanceUnit {
        switch source {
        case .core: return .duffs
        case .platform, .shielded: return .credits
        }
    }

    /// Settlement unit of a flow's *fee*, which can differ from the
    /// selected source's balance unit. `coreToShielded` spends Core
    /// duffs but its Type 18 pool fee is denominated in Platform
    /// credits, so the fee row must use credits even though the Core
    /// source row uses duffs. Behaviour-preserving for every other
    /// flow (their fee unit already matches their source unit).
    private func feeUnit(for flow: SendFlow) -> SendBalanceUnit {
        switch flow {
        case .coreToCore: return .duffs
        case .coreToShielded, .platformToPlatform, .platformToShielded,
             .shieldedToShielded, .shieldedToPlatform, .shieldedToCore:
            return .credits
        }
    }
}

// MARK: - Subviews

/// One extra Core output editor: an address field + amount field +
/// remove control, with a per-row Core badge / inline invalid hint that
/// mirrors the primary recipient's `AddressTypeBadge`. Factored out of
/// `SendTransactionView.body` to keep the Form's type-check tractable
/// (the surrounding view already drives the compiler hard). Holds no
/// state of its own — the `recipient` binding writes straight back into
/// the view model's `additionalCoreRecipients` array, and validity is
/// re-parsed here only for the inline hint (the authoritative gate is
/// the view model's `coreRecipients`).
private struct CoreRecipientRow: View {
    @Binding var recipient: CoreRecipient
    let network: Network
    let onRemove: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 8) {
                TextField("Recipient Address", text: $recipient.address)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                Button(role: .destructive, action: onRemove) {
                    Image(systemName: "minus.circle.fill")
                        .foregroundColor(.red)
                }
                // `.borderless` so the delete control doesn't make the
                // whole row tappable and swallow text-field taps — same
                // reason the primary row's QR button uses it.
                .buttonStyle(.borderless)
                .accessibilityLabel("Remove recipient")
            }

            HStack {
                TextField("0.00000000", text: $recipient.amountString)
                    .keyboardType(.decimalPad)
                Text("DASH")
                    .foregroundColor(.secondary)
            }

            // Inline validity hint: show the Core badge once the address
            // resolves on this network, otherwise a red hint so the user
            // sees *which* extra row is blocking Send. Empty address shows
            // nothing (the row is simply incomplete, not wrong).
            if !recipient.address.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty {
                if isCoreAddress {
                    AddressTypeBadge(type: .core(Data()))
                } else {
                    Text("Not a Core address on this network")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }
        }
    }

    /// Whether this row's trimmed address parses as a `.core` address on
    /// `network` — same `DashAddress.parse` the view model gates on, so
    /// the hint can't disagree with the Send button.
    private var isCoreAddress: Bool {
        let trimmed = recipient.address
            .trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return false }
        if case .core = DashAddress.parse(trimmed, network: network).type {
            return true
        }
        return false
    }
}

private struct AddressTypeBadge: View {
    let type: DashAddressType

    var body: some View {
        HStack(spacing: 6) {
            Circle().fill(badgeColor).frame(width: 8, height: 8)
            Text(badgeText)
                .font(.caption).fontWeight(.medium).foregroundColor(badgeColor)
        }
        .padding(.horizontal, 10).padding(.vertical, 4)
        .background(badgeColor.opacity(0.1))
        .cornerRadius(8)
    }

    private var badgeText: String {
        switch type {
        case .core: return "Core Address"
        case .platform: return "Platform Address"
        case .orchard: return "Shielded Address"
        case .unknown: return "Unknown Address"
        }
    }

    private var badgeColor: Color {
        switch type {
        case .core: return .green
        case .platform: return .blue
        case .orchard: return .purple
        case .unknown: return .red
        }
    }
}

/// Whether a `UInt64` balance reads as L1 duffs (1 DASH = 1e8) or
/// Platform / shielded credits (1 DASH = 1e11). The two scales
/// differ by 1000× — formatting Platform credits as duffs over-
/// reports balances by exactly that factor.
enum SendBalanceUnit {
    case duffs
    case credits

    fileprivate var dashDivisor: Double {
        switch self {
        case .duffs: return 100_000_000.0
        case .credits: return 100_000_000_000.0
        }
    }
}

private struct BalanceInfoRow: View {
    let label: String
    let amount: UInt64
    var unit: SendBalanceUnit = .duffs
    var color: Color = .primary

    var body: some View {
        HStack {
            Text(label).font(.caption).foregroundColor(.secondary)
            Spacer()
            Text(formatBalance(amount, unit: unit))
                .font(.caption).foregroundColor(amount > 0 ? color : .secondary)
        }
    }

    private func formatBalance(_ amount: UInt64, unit: SendBalanceUnit) -> String {
        let dash = Double(amount) / unit.dashDivisor
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        return String(format: "%.8f DASH", dash)
    }
}
