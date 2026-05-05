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
                    TextField("Recipient Address", text: $viewModel.recipientAddress)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()

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
                                Text("~\(formatBalance(fee))")
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
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
                            let coreWallet = try? walletManager
                                .wallet(for: wallet.walletId)?
                                .coreWallet()
                            await viewModel.executeSend(
                                sdk: sdk,
                                walletManager: walletManager,
                                shieldedService: shieldedService,
                                platformState: platformState,
                                wallet: wallet,
                                coreWallet: coreWallet,
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
        }
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
}

// MARK: - Subviews

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
