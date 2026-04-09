import SwiftUI
import SwiftDashSDK
import SwiftData

/// View for adding a new account to a wallet
struct AddAccountView: View {
    @EnvironmentObject var walletService: WalletService
    @Environment(\.dismiss) private var dismiss

    let wallet: HDWallet

    @State private var selectedAccountType: AddableAccountType = .bip44
    @State private var accountIndex: String = ""
    @State private var keyClass: String = "0"
    @State private var isCreating = false
    @State private var errorMessage: String?
    @State private var showError = false

    /// Account types that can be added by the user
    enum AddableAccountType: String, CaseIterable, Identifiable {
        case bip44 = "BIP44 (Standard)"
        case bip32 = "BIP32 (Legacy)"
        case coinjoin = "CoinJoin (Privacy)"
        case platformPayment = "Platform Payment"
        case identityTopup = "Identity Top-up"

        var id: String { rawValue }

        var accountType: AccountType {
            switch self {
            case .bip44: return .standardBIP44
            case .bip32: return .standardBIP32
            case .coinjoin: return .coinJoin
            case .platformPayment: return .platformPayment
            case .identityTopup: return .identityTopUp
            }
        }

        var description: String {
            switch self {
            case .bip44:
                return "Standard account for receiving and sending DASH. Recommended for most users."
            case .bip32:
                return "Legacy account type for compatibility with older systems."
            case .coinjoin:
                return "Privacy-enhanced account for mixing transactions."
            case .platformPayment:
                return "Platform payment account (DIP-17) for receiving credits to platform payment addresses."
            case .identityTopup:
                return "Account for topping up platform identity credits."
            }
        }

        var icon: String {
            switch self {
            case .bip44: return "folder.fill"
            case .bip32: return "tray.full.fill"
            case .coinjoin: return "shuffle.circle.fill"
            case .platformPayment: return "creditcard.fill"
            case .identityTopup: return "arrow.up.circle.fill"
            }
        }

        var color: Color {
            switch self {
            case .bip44: return .blue
            case .bip32: return .teal
            case .coinjoin: return .orange
            case .platformPayment: return .green
            case .identityTopup: return .purple
            }
        }

        var requiresIndex: Bool {
            // All these account types require an index
            return true
        }

        var indexPlaceholder: String {
            switch self {
            case .bip44: return "e.g., 1, 2, 3..."
            case .bip32: return "e.g., 0, 1, 2..."
            case .coinjoin: return "e.g., 0, 1, 2..."
            case .platformPayment: return "e.g., 0, 1, 2..."
            case .identityTopup: return "Identity index (e.g., 0)"
            }
        }

        /// Returns true if this account type needs a key class parameter
        var requiresKeyClass: Bool {
            self == .platformPayment
        }

        /// Returns the derivation path template for this account type
        func derivationPath(index: UInt32, keyClass: UInt32, isTestnet: Bool) -> String {
            let coinType = isTestnet ? "1'" : "5'"
            switch self {
            case .bip44:
                return "m/44'/\(coinType)/\(index)'"
            case .bip32:
                return "m/\(index)'"
            case .coinjoin:
                return "m/9'/\(coinType)/4'/\(index)'"
            case .platformPayment:
                return "m/9'/\(coinType)/17'/\(index)'/\(keyClass)'/..."
            case .identityTopup:
                return "m/9'/\(coinType)/5'/2'/\(index)'/..."
            }
        }
    }

    var body: some View {
        NavigationView {
            Form {
                // Account Type Selection
                Section {
                    Picker("Account Type", selection: $selectedAccountType) {
                        ForEach(AddableAccountType.allCases) { type in
                            HStack {
                                Image(systemName: type.icon)
                                    .foregroundColor(type.color)
                                Text(type.rawValue)
                            }
                            .tag(type)
                        }
                    }
                    .pickerStyle(.navigationLink)
                } header: {
                    Text("Account Type")
                } footer: {
                    Text(selectedAccountType.description)
                        .foregroundColor(.secondary)
                }

                // Account Index
                Section {
                    TextField(selectedAccountType.indexPlaceholder, text: $accountIndex)
                        .keyboardType(.numberPad)
                } header: {
                    Text("Account Index")
                } footer: {
                    Text("Enter the account index number. Each account type can have multiple accounts with different indices.")
                        .foregroundColor(.secondary)
                }

                // Key Class (for Platform Payment accounts)
                if selectedAccountType.requiresKeyClass {
                    Section {
                        TextField("e.g., 0", text: $keyClass)
                            .keyboardType(.numberPad)
                    } header: {
                        Text("Key Class")
                    } footer: {
                        Text("The key class level in the DIP-17 derivation path. Typically 0 for main addresses.")
                            .foregroundColor(.secondary)
                    }
                }

                // Preview
                Section("Preview") {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Image(systemName: selectedAccountType.icon)
                                .foregroundColor(selectedAccountType.color)
                                .font(.title2)

                            VStack(alignment: .leading, spacing: 4) {
                                Text(accountLabel)
                                    .font(.headline)

                                Text(selectedAccountType.rawValue)
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                            }

                            Spacer()

                            if let index = parsedIndex {
                                Text("#\(index)")
                                    .font(.system(.body, design: .monospaced))
                                    .foregroundColor(.secondary)
                            }
                        }

                        // Derivation Path
                        if parsedIndex != nil {
                            HStack {
                                Text("Path:")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                Text(derivationPath)
                                    .font(.system(.caption, design: .monospaced))
                                    .foregroundColor(.secondary)
                            }
                        }
                    }
                    .padding(.vertical, 4)
                }

                // Create Button
                Section {
                    Button(action: createAccount) {
                        HStack {
                            Spacer()
                            if isCreating {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                                    .scaleEffect(0.8)
                                Text("Creating...")
                            } else {
                                Image(systemName: "plus.circle.fill")
                                Text("Create Account")
                            }
                            Spacer()
                        }
                    }
                    .disabled(!canCreate || isCreating)
                }
            }
            .navigationTitle("Add Account")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
            .alert("Error", isPresented: $showError) {
                Button("OK") { }
            } message: {
                Text(errorMessage ?? "An unknown error occurred")
            }
        }
    }

    // MARK: - Computed Properties

    private var parsedIndex: UInt32? {
        guard !accountIndex.isEmpty else { return nil }
        return UInt32(accountIndex)
    }

    private var parsedKeyClass: UInt32 {
        UInt32(keyClass) ?? 0
    }

    private var canCreate: Bool {
        parsedIndex != nil
    }

    private var isTestnet: Bool {
        wallet.network == .testnet || wallet.network == .regtest || wallet.network == .devnet
    }

    private var derivationPath: String {
        guard let index = parsedIndex else { return "" }
        return selectedAccountType.derivationPath(index: index, keyClass: parsedKeyClass, isTestnet: isTestnet)
    }

    private var accountLabel: String {
        guard let index = parsedIndex else {
            return "Account"
        }

        switch selectedAccountType {
        case .bip44:
            return index == 0 ? "Main Account" : "Account \(index)"
        case .bip32:
            return "BIP32 Account \(index)"
        case .coinjoin:
            return "CoinJoin Account \(index)"
        case .platformPayment:
            return "Platform Payment \(index)"
        case .identityTopup:
            return "Top-up Account \(index)"
        }
    }

    // MARK: - Actions

    private func createAccount() {
        guard let index = parsedIndex else { return }

        isCreating = true
        errorMessage = nil

        Task {
            do {
                // Add the account via the wallet manager
                try walletService.walletManager.addAccount(
                    to: wallet,
                    type: selectedAccountType.accountType,
                    index: index,
                    keyClass: parsedKeyClass
                )

                await MainActor.run {
                    isCreating = false
                    dismiss()
                }
            } catch {
                await MainActor.run {
                    isCreating = false
                    errorMessage = error.localizedDescription
                    showError = true
                }
            }
        }
    }
}

// MARK: - Preview

struct AddAccountView_Previews: PreviewProvider {
    static var previews: some View {
        Text("Preview requires wallet context")
    }
}
