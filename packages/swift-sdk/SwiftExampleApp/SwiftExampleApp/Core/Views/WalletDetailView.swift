import SwiftUI
import SwiftDashSDK
import SwiftData
import DashSDKFFI

struct WalletDetailView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @Environment(\.dismiss) private var dismiss
    let wallet: HDWallet
    @State private var showReceiveAddress = false
    @State private var showSendTransaction = false
    @State private var showWalletInfo = false

    var body: some View {
        VStack(spacing: 0) {
            // Network indicator
            HStack {
                Label(unifiedAppState.platformState.currentNetwork.displayName, systemImage: "network")
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
                        .environmentObject(walletService)
                        .environmentObject(unifiedAppState)
                } label: {
                    HStack {
                        Label("View All Transactions", systemImage: "list.bullet.rectangle")
                            .font(.subheadline)

                        Spacer()

                        let transactions = walletService.walletManager.getTransactions(for: wallet)
                        let transactionCount = transactions.count
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
                .environmentObject(walletService)
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
                .environmentObject(walletService)
                .environmentObject(unifiedAppState)
                .environmentObject(unifiedAppState.shieldedService)
        }
        .sheet(isPresented: $showSendTransaction) {
            SendTransactionView(wallet: wallet)
                .environmentObject(walletService)
                .environmentObject(unifiedAppState)
                .environmentObject(unifiedAppState.shieldedService)
        }
        .sheet(isPresented: $showWalletInfo) {
            WalletInfoView(wallet: wallet) {
                dismiss()
            }
            .environmentObject(walletService)
        }
        .onAppear { unifiedAppState.showWalletsSyncDetails = false }
    }
}

// MARK: - Wallet Info View

struct WalletInfoView: View {
    @EnvironmentObject var walletService: WalletService
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
                Task { await loadAccountCounts() }
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
        }
    }

    private func loadNetworkStates() {
        // TODO: Probably not needed this way anymore?
        switch wallet.network {
        case .mainnet:
            mainnetEnabled = true
        case .testnet:
            testnetEnabled = true
        case .regtest:
            // TODO: Handle this properly in the UI or somehow ignore it.
            regtestEnabled = true
        case .devnet:
            devnetEnabled = true
        }
    }

    private func loadAccountCounts() async {
        // TODO: This can probably be refactored now with with single network manager?
        if mainnetEnabled {
            if let list = try? await walletService.walletManager.getAccounts(for: wallet) {
                mainnetAccountCount = list.count
            }
        } else { mainnetAccountCount = nil }

        if testnetEnabled {
            if let list = try? await walletService.walletManager.getAccounts(for: wallet) {
                testnetAccountCount = list.count
            }
        } else { testnetAccountCount = nil }

        if devnetEnabled {
            if let list = try? await walletService.walletManager.getAccounts(for: wallet) {
                devnetAccountCount = list.count
            }
        } else { devnetAccountCount = nil }
    }

    // Format a block height with thousands separators
    private func formatHeight(_ h: Int) -> String {
        let f = NumberFormatter()
        f.numberStyle = .decimal
        return f.string(from: NSNumber(value: h)) ?? "\(h)"
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

        do {

            // TODO: This needs some love after single wallet refactoring.

            // Save to Core Data
            try modelContext.save()

            // Reload network states
            loadNetworkStates()
            await loadAccountCounts()

            // TODO: Call FFI to actually add the network to the wallet
            // This would involve reinitializing the wallet with the new networks

        } catch {
            await MainActor.run {
                errorMessage = "Failed to enable network: \(error.localizedDescription)"
                showError = true
            }
        }
    }

    private func deleteWallet() async {
        // IMPORTANT: Dismiss views FIRST to prevent UI from accessing deleted relationships
        // This prevents "Never access a full future backing data" crash
        await MainActor.run {
            dismiss()
            onWalletDeleted()
        }

        try! await walletService.walletManager.deleteWallet(wallet)
    }
}

struct BalanceCardView: View {
    let wallet: HDWallet
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var shieldedService: ShieldedService
    @EnvironmentObject var platformBalanceSyncService: PlatformBalanceSyncService

    /// Platform balance from BLAST sync (preferred) or identity sum (fallback).
    var platformBalance: UInt64 {
        let blastBalance = platformBalanceSyncService.totalPlatformBalance
        if blastBalance > 0 || platformBalanceSyncService.lastSyncTime != nil {
            return blastBalance
        }
        // Fallback to identity-based sum if BLAST sync hasn't run yet
        return unifiedAppState.platformState.identities
            .filter { identity in
                identity.walletId == wallet.walletId &&
                identity.network == wallet.network.rawValue
            }
            .reduce(0) { sum, identity in
                sum + identity.balance
            }
    }

    var body: some View {
        let balance = walletService.walletManager.getBalance(for: wallet)
        let allZero = balance.total == 0 && platformBalance == 0 && shieldedService.shieldedBalance == 0

        VStack(spacing: 12) {
            if allZero {
                Text("Empty Wallet")
                    .font(.system(size: 28, weight: .medium, design: .rounded))
                    .foregroundColor(.secondary)
            } else {
                // Core Balance row
                WalletBalanceRow(
                    label: "Core Balance",
                    amount: balance.confirmed,
                    incoming: balance.unconfirmed,
                    color: .primary
                )

                // Platform Balance row
                WalletBalanceRow(
                    label: "Platform Balance",
                    amount: platformBalance,
                    color: .blue,
                    showSyncIndicator: platformBalanceSyncService.isSyncing
                )

                // Shielded Balance row
                WalletBalanceRow(
                    label: "Shielded Balance",
                    amount: shieldedService.shieldedBalance,
                    color: .purple,
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
private struct WalletBalanceRow: View {
    let label: String
    var amount: UInt64
    var incoming: UInt64 = 0
    var color: Color
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
        let dash = Double(amount) / 100_000_000.0
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