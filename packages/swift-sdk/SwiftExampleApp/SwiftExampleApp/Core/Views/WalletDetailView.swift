import SwiftUI
import SwiftDashSDK
import SwiftData
import DashSDKFFI
import LocalAuthentication

struct WalletDetailView: View {
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var appUIState: AppUIState
    @Environment(\.dismiss) private var dismiss
    let wallet: HDWallet
    @State private var showReceiveAddress = false
    @State private var showSendTransaction = false
    @State private var showWalletInfo = false

    // Transactions for this wallet come from SwiftData now.
    @Query private var accounts: [PersistentAccount]

    init(wallet: HDWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _accounts = Query(filter: #Predicate<PersistentAccount> { acc in
            acc.wallet?.walletId == walletId
        })
    }

    private var transactionCount: Int {
        accounts.reduce(0) { $0 + $1.transactions.count }
    }

    var body: some View {
        VStack(spacing: 0) {
            // Network indicator
            HStack {
                Label(platformState.currentNetwork.displayName, systemImage: "network")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .padding(.horizontal, 12)
                    .padding(.vertical, 6)
                    .background(Color(UIColor.tertiarySystemBackground))
                    .cornerRadius(8)
                Spacer()
            }
            .padding(.horizontal)
            .padding(.top, 8)

            // Balance Card
            BalanceCardView(wallet: wallet)
                .padding()

            // Action Buttons
            HStack(spacing: 16) {
                Button {
                    showSendTransaction = true
                } label: {
                    Label("Send", systemImage: "arrow.up.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)

                Button {
                    showReceiveAddress = true
                } label: {
                    Label("Receive", systemImage: "arrow.down.circle.fill")
                        .frame(maxWidth: .infinity)
                }
                .buttonStyle(.bordered)
            }
            .padding(.horizontal)

            Divider()
                .padding(.vertical, 8)

            // Transactions Section
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Transactions")
                        .font(.headline)
                    Spacer()
                }
                .padding(.horizontal)

                NavigationLink {
                    TransactionListView(wallet: wallet)
                } label: {
                    HStack {
                        Label("View All Transactions", systemImage: "list.bullet.rectangle")
                            .font(.subheadline)

                        Spacer()

                        if transactionCount > 0 {
                            Text("\(transactionCount)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(Color(UIColor.secondarySystemBackground))
                                .cornerRadius(8)
                        }

                        Image(systemName: "chevron.right")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    .padding(.horizontal)
                    .padding(.vertical, 12)
                    .background(Color(UIColor.secondarySystemBackground))
                    .cornerRadius(10)
                }
                .buttonStyle(.plain)
                .padding(.horizontal)
            }

            Divider()
                .padding(.vertical, 8)

            // Section header
            HStack {
                Text("Accounts")
                    .font(.headline)
                    .padding(.horizontal)
                Spacer()
            }
            .padding(.top)

            // Account List
            AccountListView(wallet: wallet)
        }
        .navigationTitle(wallet.label)
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .navigationBarTrailing) {
                Button {
                    showWalletInfo = true
                } label: {
                    Image(systemName: "info.circle")
                }
            }
        }
        .sheet(isPresented: $showReceiveAddress) {
            ReceiveAddressView(wallet: wallet)
        }
        .sheet(isPresented: $showSendTransaction) {
            SendTransactionView(wallet: wallet)
        }
        .sheet(isPresented: $showWalletInfo) {
            WalletInfoView(wallet: wallet) {
                dismiss()
            }
        }
        .onAppear { appUIState.showWalletsSyncDetails = false }
    }
}

// MARK: - Wallet Info View

struct WalletInfoView: View {
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) var modelContext
    let wallet: HDWallet
    var onWalletDeleted: () -> Void = {}

    @State private var editedName: String = ""
    @State private var isEditingName = false
    @State private var mainnetEnabled: Bool = false
    @State private var testnetEnabled: Bool = false
    @State private var regtestEnabled: Bool = false
    @State private var devnetEnabled: Bool = false
    @State private var isUpdatingNetworks = false
    @State private var errorMessage: String?
    @State private var showError = false
    @State private var showDeleteConfirmation = false
    @State private var isDeleting = false
    @State private var mainnetAccountCount: Int? = nil
    @State private var testnetAccountCount: Int? = nil
    @State private var devnetAccountCount: Int? = nil

    // "View Seed Phrase" flow.
    @State private var isAuthorizingSeedPhrase = false
    @State private var revealedMnemonic: String?

    // Account counts come from SwiftData now.
    @Query private var accounts: [PersistentAccount]

    init(wallet: HDWallet, onWalletDeleted: @escaping () -> Void = {}) {
        self.wallet = wallet
        self.onWalletDeleted = onWalletDeleted
        let walletId = wallet.walletId
        _accounts = Query(filter: #Predicate<PersistentAccount> { $0.wallet?.walletId == walletId })
    }

    var body: some View {
        NavigationView {
            Form {
                // Wallet Name Section
                Section("Wallet Name") {
                    if isEditingName {
                        HStack {
                            TextField("Wallet Name", text: $editedName)
                                .textFieldStyle(.plain)

                            Button("Cancel") {
                                editedName = wallet.label
                                isEditingName = false
                            }
                            .buttonStyle(.bordered)

                            Button("Save") {
                                saveWalletName()
                            }
                            .buttonStyle(.borderedProminent)
                            .disabled(editedName.isEmpty)
                        }
                    } else {
                        HStack {
                            Text(wallet.label)
                            Spacer()
                            Button("Edit") {
                                editedName = wallet.label
                                isEditingName = true
                            }
                        }
                    }
                }

                // Networks Section
                Section("Networks") {
                    HStack {
                        Text("Mainnet")
                        Spacer()
                        if mainnetEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.mainnet)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }

                    HStack {
                        Text("Testnet")
                        Spacer()
                        if testnetEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.testnet)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }

                    HStack {
                        Text("Devnet")
                        Spacer()
                        if devnetEnabled {
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        } else {
                            Button(action: {
                                Task {
                                    await enableNetwork(.devnet)
                                }
                            }) {
                                Image(systemName: "plus.circle")
                                    .foregroundColor(.blue)
                            }
                            .disabled(isUpdatingNetworks)
                        }
                    }
                }

                Section {
                    Text("Once a network is enabled, it cannot be removed. Tap + to add a network.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }

                // Wallet Info Section
                Section("Information") {
                    HStack {
                        Text("Created")
                        Spacer()
                        Text(wallet.createdAt, style: .date)
                            .foregroundColor(.secondary)
                    }

                    HStack {
                        Text("Wallet ID")
                        Spacer()
                        Text(wallet.walletId.toHexString())
                            .font(.system(.footnote, design: .monospaced))
                            .foregroundColor(.secondary)
                            .textSelection(.enabled)
                            .multilineTextAlignment(.trailing)
                    }

                    if mainnetEnabled {
                        HStack {
                            Text("Mainnet Accounts")
                            Spacer()
                            Text(mainnetAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                    if testnetEnabled {
                        HStack {
                            Text("Testnet Accounts")
                            Spacer()
                            Text(testnetAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                    if devnetEnabled {
                        HStack {
                            Text("Devnet Accounts")
                            Spacer()
                            Text(devnetAccountCount.map(String.init) ?? "–")
                                .foregroundColor(.secondary)
                        }
                    }
                }

                // View Seed Phrase Section — above Delete so the
                // destructive action stays at the bottom.
                Section {
                    Button(action: {
                        Task { await authorizeAndRevealMnemonic() }
                    }) {
                        HStack {
                            Spacer()
                            if isAuthorizingSeedPhrase {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                            } else {
                                Label("View Seed Phrase", systemImage: "eye")
                            }
                            Spacer()
                        }
                    }
                    .disabled(isAuthorizingSeedPhrase)
                }

                // Delete Wallet Section
                Section {
                    Button(action: {
                        showDeleteConfirmation = true
                    }) {
                        HStack {
                            Spacer()
                            if isDeleting {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                            } else {
                                Label("Delete Wallet", systemImage: "trash")
                                    .foregroundColor(.white)
                            }
                            Spacer()
                        }
                    }
                    .disabled(isDeleting)
                    .listRowBackground(Color.red)
                }
            }
            .navigationTitle("Wallet Info")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .onAppear {
                loadNetworkStates()
                loadAccountCounts()
            }
            .onChange(of: accounts.count) { _, _ in
                loadAccountCounts()
            }
            .alert("Error", isPresented: $showError) {
                Button("OK") { }
            } message: {
                Text(errorMessage ?? "An error occurred")
            }
            .alert("Delete Wallet", isPresented: $showDeleteConfirmation) {
                Button("Cancel", role: .cancel) { }
                Button("Delete", role: .destructive) {
                    Task {
                        await deleteWallet()
                    }
                }
            } message: {
                Text("Are you sure you want to delete this wallet? This action cannot be undone and you will lose access to all funds unless you have backed up your recovery phrase.")
            }
            .sheet(
                isPresented: Binding(
                    get: { revealedMnemonic != nil },
                    set: { if !$0 { revealedMnemonic = nil } }
                )
            ) {
                if let phrase = revealedMnemonic {
                    SeedPhraseRevealSheet(mnemonic: phrase)
                }
            }
        }
    }

    /// Prompt the user via biometric / passcode, then pull the
    /// wallet's mnemonic out of the keychain for display. On failure
    /// surfaces the error via `errorMessage`/`showError`.
    @MainActor
    private func authorizeAndRevealMnemonic() async {
        guard !isAuthorizingSeedPhrase else { return }
        isAuthorizingSeedPhrase = true
        defer { isAuthorizingSeedPhrase = false }

        let context = LAContext()
        context.localizedCancelTitle = "Cancel"
        var policyError: NSError?
        guard context.canEvaluatePolicy(
            .deviceOwnerAuthentication,
            error: &policyError
        ) else {
            errorMessage = "Authentication is unavailable on this device: "
                + (policyError?.localizedDescription ?? "unknown")
            showError = true
            return
        }

        do {
            let authorized = try await context.evaluatePolicy(
                .deviceOwnerAuthentication,
                localizedReason: "Reveal your wallet's recovery phrase."
            )
            guard authorized else { return }
        } catch {
            errorMessage = "Authorization failed: \(error.localizedDescription)"
            showError = true
            return
        }

        do {
            revealedMnemonic = try WalletStorage().retrieveMnemonic(for: wallet.walletId)
        } catch {
            errorMessage = "This wallet's recovery phrase isn't stored on this device."
            showError = true
        }
    }

    private func loadNetworkStates() {
        switch wallet.network {
        case .mainnet:
            mainnetEnabled = true
        case .testnet:
            testnetEnabled = true
        case .regtest:
            regtestEnabled = true
        case .devnet:
            devnetEnabled = true
        }
    }

    private func loadAccountCounts() {
        let count = accounts.count
        mainnetAccountCount = mainnetEnabled ? count : nil
        testnetAccountCount = testnetEnabled ? count : nil
        devnetAccountCount = devnetEnabled ? count : nil
    }

    private func saveWalletName() {
        wallet.label = editedName
        do {
            try modelContext.save()
            isEditingName = false
        } catch {
            errorMessage = "Failed to save wallet name: \(error.localizedDescription)"
            showError = true
        }
    }

    private func enableNetwork(_ network: AppNetwork) async {
        isUpdatingNetworks = true
        defer { isUpdatingNetworks = false }

        // TODO(platform-wallet): Proper multi-network wallet support once the
        // Rust side exposes add-network. For now we only refresh UI state.
        do {
            try modelContext.save()
            loadNetworkStates()
            loadAccountCounts()
        } catch {
            await MainActor.run {
                errorMessage = "Failed to enable network: \(error.localizedDescription)"
                showError = true
            }
        }
    }

    private func deleteWallet() async {
        // Dismiss views FIRST to prevent UI from accessing deleted relationships
        // and then delete both the HDWallet and any persistent wallet records.
        await MainActor.run {
            dismiss()
            onWalletDeleted()
        }

        // Remove PersistentWallet cascade-records matching walletId.
        let walletId = wallet.walletId
        let descriptor = FetchDescriptor<PersistentWallet>(
            predicate: #Predicate<PersistentWallet> { $0.walletId == walletId }
        )
        if let persisted = try? modelContext.fetch(descriptor) {
            for record in persisted {
                modelContext.delete(record)
            }
        }
        modelContext.delete(wallet)
        try? modelContext.save()
        // TODO(platform-wallet): expose wallet removal on PlatformWalletManager
        // so the Rust side also drops the in-memory handle.
    }
}

struct BalanceCardView: View {
    let wallet: HDWallet
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var shieldedService: ShieldedService
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    @Query private var persistentWallets: [PersistentWallet]
    /// Per-wallet platform-address balance rows. SwiftData drives
    /// the sum directly so every wallet's card reflects its own
    /// funds — the previous code read
    /// `platformBalanceSyncService.totalPlatformBalance`, a
    /// singleton tied to whichever wallet was most recently
    /// configured, which caused every wallet's balance to show the
    /// last-synced wallet's total.
    @Query private var addressBalances: [PersistentAddressBalance]
    /// Network-scoped BLAST sync watermark. One row per network —
    /// shared across every wallet on that network — so this query
    /// filters by `network` rather than `walletId`. Used only to
    /// distinguish "synced with zero balance" from "never synced".
    @Query private var syncStates: [PersistentSyncState]

    init(wallet: HDWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        let networkName = wallet.network.rawValue
        _persistentWallets = Query(filter: #Predicate<PersistentWallet> { $0.walletId == walletId })
        _addressBalances = Query(
            filter: #Predicate<PersistentAddressBalance> { $0.walletId == walletId }
        )
        _syncStates = Query(
            filter: #Predicate<PersistentSyncState> { $0.network == networkName }
        )
    }

    private var confirmedBalance: UInt64 {
        persistentWallets.first?.balanceConfirmed ?? 0
    }

    private var unconfirmedBalance: UInt64 {
        persistentWallets.first?.balanceUnconfirmed ?? 0
    }

    /// Platform balance from BLAST sync (preferred) or identity sum (fallback).
    var platformBalance: UInt64 {
        let blastBalance = addressBalances.reduce(0) { $0 + $1.balance }
        let hasSynced = syncStates.first.map { $0.syncHeight > 0 || $0.syncTimestamp > 0 }
            ?? false
        if blastBalance > 0 || hasSynced {
            return blastBalance
        }
        return platformState.identities
            .filter { identity in
                identity.walletId == wallet.walletId &&
                identity.network == wallet.network.rawValue
            }
            .reduce(0) { sum, identity in
                sum + identity.balance
            }
    }

    var body: some View {
        let totalCore = confirmedBalance + unconfirmedBalance
        let allZero = totalCore == 0 && platformBalance == 0 && shieldedService.shieldedBalance == 0

        VStack(spacing: 12) {
            if allZero {
                Text("Empty Wallet")
                    .font(.system(size: 28, weight: .medium, design: .rounded))
                    .foregroundColor(.secondary)
            } else {
                // Core Balance row
                WalletBalanceRow(
                    label: "Core Balance",
                    amount: confirmedBalance,
                    incoming: unconfirmedBalance,
                    color: .primary,
                    unit: .duffs
                )

                // Platform Balance row
                WalletBalanceRow(
                    label: "Platform Balance",
                    amount: platformBalance,
                    color: .blue,
                    unit: .credits,
                    showSyncIndicator: platformBalanceSyncService.isSyncing
                )

                // Shielded Balance row
                WalletBalanceRow(
                    label: "Shielded Balance",
                    amount: shieldedService.shieldedBalance,
                    color: .purple,
                    unit: .credits,
                    showSyncIndicator: shieldedService.isSyncing
                )
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
}

/// A single balance row showing label, amount, and optional incoming amount.
private enum WalletBalanceUnit {
    case duffs
    case credits
}

private struct WalletBalanceRow: View {
    let label: String
    var amount: UInt64
    var incoming: UInt64 = 0
    var color: Color
    var unit: WalletBalanceUnit = .duffs
    var showSyncIndicator: Bool = false

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 4) {
                    Text(label)
                        .font(.caption)
                        .foregroundColor(.secondary)
                    if showSyncIndicator {
                        ProgressView()
                            .scaleEffect(0.5)
                    }
                }
            }
            Spacer()
            VStack(alignment: .trailing, spacing: 2) {
                if amount > 0 {
                    Text(formatBalance(amount))
                        .font(.subheadline)
                        .fontWeight(.medium)
                        .foregroundColor(color)
                } else {
                    Text("—")
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                if incoming > 0 {
                    Text("(+\(formatBalance(incoming)) incoming)")
                        .font(.caption2)
                        .foregroundColor(.orange)
                }
            }
        }
    }

    private func formatBalance(_ amount: UInt64) -> String {
        let dashDivisor: Double = switch unit {
        case .duffs:
            100_000_000.0
        case .credits:
            100_000_000_000.0
        }
        let dash = Double(amount) / dashDivisor
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

// MARK: - Seed Phrase Reveal Sheet

/// Read-only reveal of the mnemonic, gated by biometric auth on the
/// caller side. Renders the 12-word phrase in a numbered grid with a
/// copy-to-clipboard convenience and a warning banner.
private struct SeedPhraseRevealSheet: View {
    let mnemonic: String
    @Environment(\.dismiss) private var dismiss
    @State private var copied = false

    private var words: [String] {
        mnemonic.split(separator: " ").map(String.init)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: 16) {
                    Label(
                        "Never share this phrase. Anyone who sees it can spend your funds.",
                        systemImage: "exclamationmark.triangle.fill"
                    )
                    .font(.subheadline)
                    .foregroundColor(.white)
                    .padding()
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .background(Color.red)
                    .cornerRadius(10)

                    let columns = [GridItem(.flexible()), GridItem(.flexible())]
                    LazyVGrid(columns: columns, spacing: 8) {
                        ForEach(Array(words.enumerated()), id: \.offset) { idx, word in
                            HStack(spacing: 8) {
                                Text(String(format: "%2d.", idx + 1))
                                    .font(.body.monospacedDigit())
                                    .foregroundColor(.secondary)
                                    .frame(width: 28, alignment: .trailing)
                                Text(word)
                                    .font(.body)
                                    .textSelection(.enabled)
                                Spacer()
                            }
                            .padding(8)
                            .background(Color(.secondarySystemBackground))
                            .cornerRadius(8)
                        }
                    }

                    Button {
                        UIPasteboard.general.string = mnemonic
                        copied = true
                        Task {
                            try? await Task.sleep(nanoseconds: 2_000_000_000)
                            copied = false
                        }
                    } label: {
                        Label(
                            copied ? "Copied!" : "Copy to Clipboard",
                            systemImage: copied ? "checkmark" : "doc.on.doc"
                        )
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.bordered)
                }
                .padding()
            }
            .navigationTitle("Recovery Phrase")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") { dismiss() }
                }
            }
        }
    }
}
