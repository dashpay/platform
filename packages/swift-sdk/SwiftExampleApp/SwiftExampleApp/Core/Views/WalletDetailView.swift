import SwiftUI
import SwiftData
import DashSDKFFI

struct WalletDetailView: View {
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
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
        }
        .sheet(isPresented: $showSendTransaction) {
            SendTransactionView(wallet: wallet)
                .environmentObject(walletService)
                .environmentObject(unifiedAppState)
        }
        .sheet(isPresented: $showWalletInfo) {
            WalletInfoView(wallet: wallet)
                .environmentObject(walletService)
        }
        .task {
            await walletService.loadWallet(wallet)
        }
    }
}

// MARK: - Wallet Info View

struct WalletInfoView: View {
    @EnvironmentObject var walletService: WalletService
    @Environment(\.dismiss) var dismiss
    @Environment(\.modelContext) var modelContext
    let wallet: HDWallet
    
    @State private var editedName: String = ""
    @State private var isEditingName = false
    @State private var mainnetEnabled: Bool = false
    @State private var testnetEnabled: Bool = false
    @State private var devnetEnabled: Bool = false
    @State private var isUpdatingNetworks = false
    @State private var errorMessage: String?
    @State private var showError = false
    @State private var showDeleteConfirmation = false
    @State private var isDeleting = false
    
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
                    
                    if let walletId = wallet.walletId {
                        HStack {
                            Text("Wallet ID")
                            Spacer()
                            Text(String(walletId.toHexString().prefix(16)) + "...")
                                .font(.system(.caption, design: .monospaced))
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    HStack {
                        Text("Total Accounts")
                        Spacer()
                        Text("\(wallet.accounts.count)")
                            .foregroundColor(.secondary)
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
        // Check which networks this wallet is on
        let networks = wallet.networks
        mainnetEnabled = (networks & 1) != 0  // DASH
        testnetEnabled = (networks & 2) != 0  // TESTNET
        devnetEnabled = (networks & 8) != 0   // DEVNET
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
    
    private func enableNetwork(_ network: DashNetwork) async {
        isUpdatingNetworks = true
        defer { isUpdatingNetworks = false }
        
        do {
            // Add the network to the wallet
            let networkBit: UInt32
            switch network {
            case .mainnet:
                networkBit = 1  // DASH
            case .testnet:
                networkBit = 2  // TESTNET
            case .devnet:
                networkBit = 8  // DEVNET
            }
            
            // Update the wallet's networks bitfield
            wallet.networks = wallet.networks | networkBit
            
            // Save to Core Data
            try modelContext.save()
            
            // Reload network states
            loadNetworkStates()
            
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
        isDeleting = true
        defer { 
            Task { @MainActor in
                isDeleting = false
            }
        }
        
        do {
            // Delete the wallet from Core Data
            modelContext.delete(wallet)
            try modelContext.save()
            
            // Dismiss both the info view and the wallet detail view
            await MainActor.run {
                dismiss()
                // The navigation will automatically go back when the wallet is deleted
            }
            
            // Notify the wallet service to reload
            await walletService.walletDeleted(wallet)
            
        } catch {
            await MainActor.run {
                errorMessage = "Failed to delete wallet: \(error.localizedDescription)"
                showError = true
            }
        }
    }
}

struct BalanceCardView: View {
    let wallet: HDWallet
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    
    var platformBalance: UInt64 {
        // Only sum balances of identities that belong to this specific wallet
        // and are on the same network
        
        // For now, if wallet doesn't have a walletId (not yet initialized with FFI),
        // don't show any platform balance
        guard let walletId = wallet.walletId else {
            return 0
        }
        
        return unifiedAppState.platformState.identities
            .filter { identity in
                // Check if identity belongs to this wallet and is on the same network
                // Only count identities that have been explicitly associated with this wallet
                identity.walletId == walletId &&
                identity.network == wallet.dashNetwork.rawValue
            }
            .reduce(0) { sum, identity in
                sum + identity.balance
            }
    }
    
    var body: some View {
        VStack(spacing: 12) {
            // Show main balance or "Empty Wallet"
            if wallet.totalBalance == 0 {
                Text("Empty Wallet")
                    .font(.system(size: 28, weight: .medium, design: .rounded))
                    .foregroundColor(.secondary)
            } else {
                Text("Wallet Balance")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                
                Text(formatBalance(wallet.totalBalance))
                    .font(.system(size: 36, weight: .bold, design: .rounded))
            }
            
            HStack(spacing: 20) {
                // Incoming (unconfirmed) balance
                VStack(spacing: 4) {
                    Text("Incoming")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    if wallet.unconfirmedBalance > 0 {
                        Text(formatBalance(wallet.unconfirmedBalance))
                            .font(.subheadline)
                            .fontWeight(.medium)
                            .foregroundColor(.orange)
                    } else {
                        Text("—")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
                
                Divider()
                    .frame(height: 30)
                
                // Platform balance
                VStack(spacing: 4) {
                    Text("Platform Balance")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    if platformBalance > 0 {
                        Text(formatBalance(platformBalance))
                            .font(.subheadline)
                            .fontWeight(.medium)
                            .foregroundColor(.blue)
                    } else {
                        Text("—")
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }
            }
        }
        .padding()
        .background(Color(UIColor.secondarySystemBackground))
        .cornerRadius(12)
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        
        // Format with up to 8 decimal places, removing trailing zeros
        let formatter = NumberFormatter()
        formatter.minimumFractionDigits = 0
        formatter.maximumFractionDigits = 8
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        formatter.decimalSeparator = "."
        
        if let formatted = formatter.string(from: NSNumber(value: dash)) {
            return "\(formatted) DASH"
        }
        
        return String(format: "%.8f DASH", dash).replacingOccurrences(of: "0+$", with: "", options: .regularExpression).replacingOccurrences(of: "\\.$", with: "", options: .regularExpression)
    }
}

// MARK: - Legacy Views (kept for reference)
// These views show transactions, addresses, and UTXOs directly
// They have been replaced by AccountListView which shows account-level information

/*
struct TransactionListView: View {
    let transactions: [HDTransaction]
    
    var body: some View {
        if transactions.isEmpty {
            ContentUnavailableView(
                "No Transactions",
                systemImage: "list.bullet.rectangle",
                description: Text("Transactions will appear here")
            )
        } else {
            List(transactions.sorted(by: { $0.timestamp > $1.timestamp })) { transaction in
                TransactionRowView(transaction: transaction)
            }
            .listStyle(.plain)
        }
    }
}

struct TransactionRowView: View {
    let transaction: HDTransaction
    
    var body: some View {
        HStack {
            Image(systemName: transaction.amount < 0 ? "arrow.up.circle" : "arrow.down.circle")
                .font(.title2)
                .foregroundColor(transaction.amount < 0 ? .red : .green)
            
            VStack(alignment: .leading, spacing: 4) {
                Text(transaction.type.capitalized)
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Text(transaction.timestamp, style: .date)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            
            Spacer()
            
            VStack(alignment: .trailing, spacing: 4) {
                Text(formatAmount(transaction.amount))
                    .font(.subheadline)
                    .fontWeight(.medium)
                    .foregroundColor(transaction.amount < 0 ? .red : .green)
                
                if transaction.isPending {
                    Text("Pending")
                        .font(.caption)
                        .foregroundColor(.orange)
                } else {
                    Text("\(transaction.confirmations) conf")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        .padding(.vertical, 4)
    }
    
    private func formatAmount(_ amount: Int64) -> String {
        let dash = Double(abs(amount)) / 100_000_000.0
        let sign = amount < 0 ? "-" : "+"
        return "\(sign)\(String(format: "%.8f", dash))"
    }
}

struct AddressListView: View {
    let addresses: [HDAddress]
    
    var body: some View {
        if addresses.isEmpty {
            ContentUnavailableView(
                "No Addresses",
                systemImage: "qrcode",
                description: Text("Addresses will appear here")
            )
        } else {
            List(addresses.sorted(by: { $0.index < $1.index })) { address in
                AddressRowView(address: address)
            }
            .listStyle(.plain)
        }
    }
}

struct AddressRowView: View {
    let address: HDAddress
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text("Address #\(address.index)")
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Spacer()
                
                if address.isUsed {
                    Label("Used", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                }
            }
            
            Text(address.address)
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 4)
    }
}

struct UTXOListView: View {
    let utxos: [HDUTXO]
    
    var body: some View {
        if utxos.isEmpty {
            ContentUnavailableView(
                "No UTXOs",
                systemImage: "bitcoinsign.circle",
                description: Text("Unspent outputs will appear here")
            )
        } else {
            List(utxos) { utxo in
                UTXORowView(utxo: utxo)
            }
            .listStyle(.plain)
        }
    }
}

struct UTXORowView: View {
    let utxo: HDUTXO
    
    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            HStack {
                Text(formatAmount(utxo.amount))
                    .font(.subheadline)
                    .fontWeight(.medium)
                
                Spacer()
                
                if utxo.isConfirmed {
                    Label("Confirmed", systemImage: "checkmark.circle.fill")
                        .font(.caption)
                        .foregroundColor(.green)
                } else {
                    Label("Unconfirmed", systemImage: "clock")
                        .font(.caption)
                        .foregroundColor(.orange)
                }
            }
            
            Text("\(utxo.txid):\(utxo.outputIndex)")
                .font(.system(.caption, design: .monospaced))
                .foregroundColor(.secondary)
                .lineLimit(1)
                .truncationMode(.middle)
        }
        .padding(.vertical, 4)
    }
    
    private func formatAmount(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}
*/