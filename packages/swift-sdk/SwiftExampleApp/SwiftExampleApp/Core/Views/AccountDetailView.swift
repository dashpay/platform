import SwiftUI
import SwiftData
import DashSDKFFI

// MARK: - Account Detail Info
public struct AccountDetailInfo {
    public let account: AccountInfo
    public let accountType: FFIAccountType
    public let xpub: String?
    public let derivationPath: String
    public let gapLimit: UInt32
    public let usedAddresses: Int
    public let unusedAddresses: Int
    public let externalAddresses: [AddressDetail]
    public let internalAddresses: [AddressDetail]
    
    public init(account: AccountInfo, accountType: FFIAccountType, xpub: String?, derivationPath: String, gapLimit: UInt32, usedAddresses: Int, unusedAddresses: Int, externalAddresses: [AddressDetail], internalAddresses: [AddressDetail]) {
        self.account = account
        self.accountType = accountType
        self.xpub = xpub
        self.derivationPath = derivationPath
        self.gapLimit = gapLimit
        self.usedAddresses = usedAddresses
        self.unusedAddresses = unusedAddresses
        self.externalAddresses = externalAddresses
        self.internalAddresses = internalAddresses
    }
}

public struct AddressDetail {
    public let address: String
    public let index: UInt32
    public let path: String
    public let isUsed: Bool
    public let publicKey: String
    
    public init(address: String, index: UInt32, path: String, isUsed: Bool, publicKey: String) {
        self.address = address
        self.index = index
        self.path = path
        self.isUsed = isUsed
        self.publicKey = publicKey
    }
}

// MARK: - Account Detail View
struct AccountDetailView: View {
    @EnvironmentObject var walletService: WalletService
    let wallet: HDWallet
    let account: AccountInfo
    
    @State private var detailInfo: AccountDetailInfo?
    @State private var isLoading = true
    @State private var errorMessage: String?
    @State private var selectedTab = 0
    @State private var copiedText: String?
    @State private var showingPrivateKey: String? // Path for which we're showing private key
    @State private var privateKeyToShow: (hex: String, wif: String)?
    @State private var showingPINPrompt = false
    @State private var pinInput = ""
    @State private var pendingAddressDetail: AddressDetail? // Store the address detail while waiting for PIN
    
    var body: some View {
        ScrollView {
            if isLoading {
                ProgressView("Loading account details...")
                    .padding()
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            } else if let error = errorMessage {
                ContentUnavailableView(
                    "Failed to Load Details",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error)
                )
            } else if let info = detailInfo {
                VStack(alignment: .leading, spacing: 20) {
                    // Account Overview Card
                    accountOverviewCard(info: info)
                    
                    // Extended Public Key Card
                    if let xpub = info.xpub {
                        xpubCard(xpub: xpub)
                    }
                    
                    // Balance Card (if applicable)
                    if WalletManager.shouldShowBalance(for: account.index) {
                        balanceCard()
                    }
                    
                    // Address Pool Information
                    addressPoolCard(info: info)
                    
                    // Address Lists
                    addressListsSection(info: info)
                }
                .padding()
            }
        }
        .navigationTitle(account.label)
        .navigationBarTitleDisplayMode(.large)
        .task {
            await loadAccountDetails()
        }
        .sheet(isPresented: $showingPINPrompt) {
            PINPromptView(
                pinInput: $pinInput,
                isPresented: $showingPINPrompt,
                onSubmit: {
                    if let detail = pendingAddressDetail {
                        Task {
                            await derivePrivateKeyWithPIN(for: detail, pin: pinInput)
                            pinInput = ""
                            pendingAddressDetail = nil
                        }
                    }
                }
            )
        }
    }
    
    // MARK: - View Components
    
    private func accountOverviewCard(info: AccountDetailInfo) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Account Information", systemImage: "info.circle.fill")
                .font(.headline)
                .foregroundColor(.primary)
            
            Divider()
            
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Type:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(accountTypeName)
                        .fontWeight(.medium)
                }
                
                // Only show index for account types that have one
                if hasAccountIndex {
                    HStack {
                        Text("Index:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("#\(accountDisplayIndex)")
                            .font(.system(.body, design: .monospaced))
                    }
                }
                
                HStack {
                    Text("Derivation Path:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(info.derivationPath)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                }
                
                HStack {
                    Text("Network:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(wallet.dashNetwork.rawValue.capitalized)
                        .fontWeight(.medium)
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }
    
    private func xpubCard(xpub: String) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            HStack {
                Label("Extended Public Key", systemImage: "key.horizontal.fill")
                    .font(.headline)
                    .foregroundColor(.primary)
                
                Spacer()
                
                Button(action: {
                    copyToClipboard(xpub, label: "Extended public key")
                }) {
                    Image(systemName: copiedText == xpub ? "checkmark.circle.fill" : "doc.on.doc")
                        .foregroundColor(copiedText == xpub ? .green : .blue)
                }
            }
            
            Divider()
            
            Text(xpub)
                .font(.system(.caption, design: .monospaced))
                .padding(8)
                .background(Color(.secondarySystemBackground))
                .cornerRadius(8)
                .textSelection(.enabled)
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }
    
    private func balanceCard() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Balance", systemImage: "bitcoinsign.circle.fill")
                .font(.headline)
                .foregroundColor(.primary)
            
            Divider()
            
            HStack(spacing: 20) {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Confirmed")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    Text(formatBalance(account.balance.confirmed))
                        .font(.title3)
                        .fontWeight(.semibold)
                }
                
                Spacer()
                
                if account.balance.unconfirmed > 0 {
                    VStack(alignment: .trailing, spacing: 4) {
                        Text("Pending")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(account.balance.unconfirmed))
                            .font(.title3)
                            .fontWeight(.semibold)
                            .foregroundColor(.orange)
                    }
                }
            }
            
            Divider()
            
            HStack {
                Text("Total Balance")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(formatBalance(account.balance.confirmed + account.balance.unconfirmed))
                    .font(.headline)
                    .fontWeight(.bold)
                    .foregroundColor(accountTypeColor)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }
    
    private func addressPoolCard(info: AccountDetailInfo) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Address Pool", systemImage: "square.stack.3d.up.fill")
                .font(.headline)
                .foregroundColor(.primary)
            
            Divider()
            
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Text("Gap Limit:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(info.gapLimit)")
                        .fontWeight(.medium)
                }
                
                // Only show external/internal for BIP44/BIP32 accounts
                if hasInternalExternalAddresses {
                    HStack {
                        Text("External Addresses:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("\(info.externalAddresses.count)")
                            .fontWeight(.medium)
                    }
                    
                    HStack {
                        Text("Internal Addresses:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("\(info.internalAddresses.count)")
                            .fontWeight(.medium)
                    }
                } else {
                    HStack {
                        Text("Addresses:")
                            .foregroundColor(.secondary)
                        Spacer()
                        Text("\(info.externalAddresses.count)")
                            .fontWeight(.medium)
                    }
                }
                
                HStack {
                    Text("Used Addresses:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(info.usedAddresses)")
                        .fontWeight(.medium)
                }
                
                HStack {
                    Text("Unused Addresses:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(info.unusedAddresses)")
                        .fontWeight(.medium)
                        .foregroundColor(.green)
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }
    
    private func addressListsSection(info: AccountDetailInfo) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Addresses", systemImage: "list.bullet.rectangle.fill")
                .font(.headline)
                .foregroundColor(.primary)
            
            if hasInternalExternalAddresses {
                Picker("Address Type", selection: $selectedTab) {
                    Text("Receive (\(info.externalAddresses.count))").tag(0)
                    Text("Change (\(info.internalAddresses.count))").tag(1)
                }
                .pickerStyle(SegmentedPickerStyle())
                .padding(.bottom, 8)
                
                if selectedTab == 0 {
                    addressList(addresses: info.externalAddresses, type: "Receive")
                } else {
                    addressList(addresses: info.internalAddresses, type: "Change")
                }
            } else {
                // For accounts without internal/external distinction, just show all addresses
                addressList(addresses: info.externalAddresses, type: "")
            }
        }
    }
    
    private func addressList(addresses: [AddressDetail], type: String) -> some View {
        VStack(spacing: 8) {
            if addresses.isEmpty {
                let message = type.isEmpty ? "No addresses generated" : "No \(type.lowercased()) addresses generated"
                Text(message)
                    .foregroundColor(.secondary)
                    .padding()
                    .frame(maxWidth: .infinity)
                    .background(Color(.secondarySystemBackground))
                    .cornerRadius(8)
            } else {
                ForEach(addresses, id: \.address) { detail in
                    addressRow(detail: detail)
                }
            }
        }
    }
    
    private func addressRow(detail: AddressDetail) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    HStack {
                        Text("#\(detail.index)")
                            .font(.caption)
                            .fontWeight(.medium)
                            .foregroundColor(.secondary)
                        
                        if detail.isUsed {
                            Label("Used", systemImage: "checkmark.circle.fill")
                                .font(.caption)
                                .foregroundColor(.green)
                        } else {
                            Label("Unused", systemImage: "circle")
                                .font(.caption)
                                .foregroundColor(.orange)
                        }
                    }
                    
                    Text(detail.address)
                        .font(.system(.caption, design: .monospaced))
                        .lineLimit(1)
                        .truncationMode(.middle)
                    
                    if !detail.publicKey.isEmpty {
                        HStack {
                            Text("Public Key:")
                                .font(.system(.caption2))
                                .foregroundColor(.secondary)
                            Text(String(detail.publicKey.prefix(16)) + "...")
                                .font(.system(.caption2, design: .monospaced))
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    Text(detail.path)
                        .font(.system(.caption2, design: .monospaced))
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(spacing: 4) {
                    Button(action: {
                        copyToClipboard(detail.address, label: "Address")
                    }) {
                        Image(systemName: copiedText == detail.address ? "checkmark.circle.fill" : "doc.on.doc")
                            .foregroundColor(copiedText == detail.address ? .green : .blue)
                    }
                    
                    // Show private key button for non-BIP32/BIP44/CoinJoin accounts
                    if shouldShowPrivateKeyButton {
                        Button(action: {
                            pendingAddressDetail = detail
                            showingPINPrompt = true
                        }) {
                            Image(systemName: "key")
                                .foregroundColor(.orange)
                        }
                    }
                }
            }
            .padding(12)
            .background(detail.isUsed ? Color(.tertiarySystemBackground) : Color(.secondarySystemBackground))
            .cornerRadius(8)
            
            // Show private key if requested
            if showingPrivateKey == detail.path, let privateKeyData = privateKeyToShow {
                VStack(alignment: .leading, spacing: 12) {
                    HStack {
                        Text("Private Key")
                            .font(.headline)
                            .fontWeight(.medium)
                        Spacer()
                        Button(action: {
                            showingPrivateKey = nil
                            privateKeyToShow = nil
                        }) {
                            Image(systemName: "xmark.circle.fill")
                                .foregroundColor(.secondary)
                        }
                    }
                    
                    // Hex format
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text("Hex Format:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Button(action: {
                                copyToClipboard(privateKeyData.hex, label: "Hex Private Key")
                            }) {
                                Image(systemName: copiedText == privateKeyData.hex ? "checkmark.circle.fill" : "doc.on.doc")
                                    .font(.caption)
                                    .foregroundColor(copiedText == privateKeyData.hex ? .green : .blue)
                            }
                        }
                        
                        Text(privateKeyData.hex)
                            .font(.system(size: 11, design: .monospaced))
                            .fixedSize(horizontal: false, vertical: true)
                            .lineLimit(nil)
                            .textSelection(.enabled)
                            .padding(8)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(Color(.tertiarySystemBackground))
                            .cornerRadius(4)
                    }
                    
                    // WIF format
                    VStack(alignment: .leading, spacing: 4) {
                        HStack {
                            Text("WIF Format:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Spacer()
                            Button(action: {
                                copyToClipboard(privateKeyData.wif, label: "WIF Private Key")
                            }) {
                                Image(systemName: copiedText == privateKeyData.wif ? "checkmark.circle.fill" : "doc.on.doc")
                                    .font(.caption)
                                    .foregroundColor(copiedText == privateKeyData.wif ? .green : .blue)
                            }
                        }
                        
                        Text(privateKeyData.wif)
                            .font(.system(size: 11, design: .monospaced))
                            .fixedSize(horizontal: false, vertical: true)
                            .lineLimit(nil)
                            .textSelection(.enabled)
                            .padding(8)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .background(Color(.tertiarySystemBackground))
                            .cornerRadius(4)
                    }
                }
                .padding()
                .background(Color(.systemYellow).opacity(0.1))
                .cornerRadius(8)
            }
        }
    }
    
    // MARK: - Helper Properties
    
    private var hasAccountIndex: Bool {
        switch account.index {
        case 0...999,        // BIP44 accounts
             1000...1999,    // CoinJoin accounts
             5000...5999,    // BIP32 accounts
             9100...9199:    // Identity TopUp accounts (have registration index)
            return true
        default:
            return false
        }
    }
    
    private var accountDisplayIndex: UInt32 {
        switch account.index {
        case 0...999:
            return account.index  // BIP44 account index
        case 1000...1999:
            return account.index - 1000  // CoinJoin account index
        case 5000...5999:
            return account.index - 5000  // BIP32 account index
        case 9100...9199:
            return account.index - 9100  // Identity TopUp registration index
        default:
            return account.index
        }
    }
    
    private var hasInternalExternalAddresses: Bool {
        guard let info = detailInfo else { return false }
        switch info.accountType {
        case STANDARD_BIP44, STANDARD_BIP32:
            return true
        default:
            return false
        }
    }
    
    private var shouldShowPrivateKeyButton: Bool {
        guard let info = detailInfo else { return false }
        switch info.accountType {
        case STANDARD_BIP44, STANDARD_BIP32, COIN_JOIN:
            // These account types use HD derivation, don't show individual private keys
            return false
        case IDENTITY_REGISTRATION, IDENTITY_TOP_UP, IDENTITY_TOP_UP_NOT_BOUND_TO_IDENTITY, IDENTITY_INVITATION,
             PROVIDER_VOTING_KEYS, PROVIDER_OWNER_KEYS, PROVIDER_OPERATOR_KEYS, PROVIDER_PLATFORM_KEYS:
            // These special accounts have single keys that can be shown
            return true
        default:
            return false
        }
    }
    
    private var accountTypeName: String {
        guard let info = detailInfo else { return "Unknown Account" }
        switch info.accountType {
        case STANDARD_BIP44:
            return account.index == 0 ? "Main Account" : "BIP44 Account"
        case STANDARD_BIP32:
            return "BIP32 Account"
        case COIN_JOIN:
            return "CoinJoin Account"
        case IDENTITY_REGISTRATION:
            return "Identity Registration"
        case IDENTITY_TOP_UP:
            return "Identity Top-up"
        case IDENTITY_TOP_UP_NOT_BOUND_TO_IDENTITY:
            return "Identity Top-up (Not Bound)"
        case IDENTITY_INVITATION:
            return "Identity Invitation"
        case PROVIDER_VOTING_KEYS:
            return "Provider Voting Keys"
        case PROVIDER_OWNER_KEYS:
            return "Provider Owner Keys"
        case PROVIDER_OPERATOR_KEYS:
            return "Provider Operator Keys (BLS)"
        case PROVIDER_PLATFORM_KEYS:
            return "Provider Platform Keys (EdDSA)"
        default:
            return "Special Account"
        }
    }
    
    private var accountTypeColor: Color {
        guard let info = detailInfo else { return .gray }
        switch info.accountType {
        case STANDARD_BIP44:
            return account.index == 0 ? .green : .blue
        case STANDARD_BIP32:
            return .teal
        case COIN_JOIN:
            return .orange
        case IDENTITY_REGISTRATION, IDENTITY_TOP_UP, IDENTITY_TOP_UP_NOT_BOUND_TO_IDENTITY, IDENTITY_INVITATION:
            return .purple
        case PROVIDER_VOTING_KEYS:
            return .red
        case PROVIDER_OWNER_KEYS:
            return .pink
        case PROVIDER_OPERATOR_KEYS:
            return .indigo
        case PROVIDER_PLATFORM_KEYS:
            return .cyan
        default:
            return .gray
        }
    }
    
    // MARK: - Helper Methods
    
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
    
    private func copyToClipboard(_ text: String, label: String) {
        #if os(iOS)
        UIPasteboard.general.string = text
        #endif
        
        copiedText = text
        
        // Reset after 2 seconds
        DispatchQueue.main.asyncAfter(deadline: .now() + 2) {
            if copiedText == text {
                copiedText = nil
            }
        }
    }
    
    private func derivePrivateKeyWithPIN(for detail: AddressDetail, pin: String) async {
        do {
            // Use WalletStorage to retrieve the encrypted seed with PIN
            let walletStorage = WalletStorage()
            let seedData = try walletStorage.retrieveSeed(pin: pin)
            
            // Derive private key using the path
            guard let walletManager = walletService.walletManager else {
                throw WalletError.walletError("Wallet manager not available")
            }
            
            // Use the FFI function to derive private key from seed
            let privateKeyData = try await walletManager.derivePrivateKey(
                from: seedData,
                path: detail.path,
                network: wallet.dashNetwork
            )
            
            // Generate hex format
            let hexPrivateKey = privateKeyData.toHexString()
            
            // Generate WIF format
            let wifPrivateKey = try await walletManager.derivePrivateKeyAsWIF(
                from: seedData,
                path: detail.path,
                network: wallet.dashNetwork
            )
            
            await MainActor.run {
                self.showingPrivateKey = detail.path
                self.privateKeyToShow = (hex: hexPrivateKey, wif: wifPrivateKey)
            }
        } catch {
            await MainActor.run {
                // Check if it's a wrong PIN error
                if error is WalletStorageError {
                    errorMessage = "Invalid PIN. Please try again."
                } else {
                    errorMessage = "Failed to derive private key: \(error.localizedDescription)"
                }
            }
        }
    }
    
    // MARK: - Data Loading
    
    private func loadAccountDetails() async {
        isLoading = true
        errorMessage = nil
        
        do {
            guard let walletManager = walletService.walletManager else {
                throw WalletError.walletError("Wallet manager not available")
            }
            
            // Get extended public key and other details
            let details = try await walletManager.getAccountDetails(
                for: wallet,
                accountInfo: account
            )
            
            await MainActor.run {
                self.detailInfo = details
                self.isLoading = false
            }
        } catch {
            await MainActor.run {
                self.errorMessage = error.localizedDescription
                self.isLoading = false
            }
        }
    }
}

// MARK: - PIN Prompt View

struct PINPromptView: View {
    @Binding var pinInput: String
    @Binding var isPresented: Bool
    let onSubmit: () -> Void
    
    var body: some View {
        NavigationView {
            VStack(spacing: 20) {
                Text("Enter Wallet PIN")
                    .font(.title2)
                    .fontWeight(.semibold)
                
                Text("Your PIN is required to access private keys")
                    .font(.subheadline)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
                
                SecureField("PIN", text: $pinInput)
                    .textFieldStyle(.roundedBorder)
                    .keyboardType(.numberPad)
                    .padding(.horizontal)
                
                HStack(spacing: 20) {
                    Button("Cancel") {
                        pinInput = ""
                        isPresented = false
                    }
                    .buttonStyle(.bordered)
                    
                    Button("Unlock") {
                        onSubmit()
                        isPresented = false
                    }
                    .buttonStyle(.borderedProminent)
                    .disabled(pinInput.isEmpty)
                }
                
                Spacer()
            }
            .padding()
            .navigationBarHidden(true)
        }
    }
}