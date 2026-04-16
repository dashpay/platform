import SwiftUI
import SwiftDashSDK
import SwiftData

// MARK: - Account Detail View
struct AccountDetailView: View {
    @EnvironmentObject var appUIState: AppUIState
    let wallet: HDWallet
    let account: PersistentAccount

    @State private var errorMessage: String?
    @State private var copiedText: String?
    @State private var showingPrivateKey: String?
    @State private var privateKeyToShow: (hex: String, wif: String)?
    @State private var showingPINPrompt = false
    @State private var pinInput = ""

    var body: some View {
        ScrollView {
            if let error = errorMessage {
                ContentUnavailableView(
                    "Failed to Load Details",
                    systemImage: "exclamationmark.triangle",
                    description: Text(error)
                )
            } else {
                VStack(alignment: .leading, spacing: 20) {
                    accountOverviewCard()

                    if shouldShowBalance {
                        balanceCard()
                    }

                    poolSummaryCard()

                    noteCard()
                }
                .padding()
            }
        }
        .navigationTitle(account.accountTypeName)
        .navigationBarTitleDisplayMode(.large)
        .sheet(isPresented: $showingPINPrompt) {
            PINPromptView(
                pinInput: $pinInput,
                isPresented: $showingPINPrompt,
                onSubmit: {
                    Task {
                        await derivePrivateKeyWithPIN()
                        pinInput = ""
                    }
                }
            )
        }
        .onAppear { appUIState.showWalletsSyncDetails = false }
    }

    // MARK: - Cards

    private func accountOverviewCard() -> some View {
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
                    Text(account.accountTypeName)
                        .fontWeight(.medium)
                }

                HStack {
                    Text("Index:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("#\(account.accountIndex)")
                        .font(.system(.body, design: .monospaced))
                }

                HStack {
                    Text("Network:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(wallet.network.rawValue.capitalized)
                        .fontWeight(.medium)
                }

                HStack {
                    Text("Watch Only:")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text(account.isWatchOnly ? "Yes" : "No")
                        .fontWeight(.medium)
                }
            }
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
                    Text(formatBalance(account.balanceConfirmed))
                        .font(.title3)
                        .fontWeight(.semibold)
                }

                Spacer()

                if account.balanceUnconfirmed > 0 {
                    VStack(alignment: .trailing, spacing: 4) {
                        Text("Pending")
                            .font(.caption)
                            .foregroundColor(.secondary)
                        Text(formatBalance(account.balanceUnconfirmed))
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
                Text(formatBalance(account.balanceConfirmed + account.balanceUnconfirmed))
                    .font(.headline)
                    .fontWeight(.bold)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func poolSummaryCard() -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("Address Pool", systemImage: "square.stack.3d.up.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            HStack {
                Text("Highest Used (External):")
                    .foregroundColor(.secondary)
                Spacer()
                Text(account.externalHighestUsed >= 0
                     ? "\(account.externalHighestUsed)" : "—")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Highest Used (Internal):")
                    .foregroundColor(.secondary)
                Spacer()
                Text(account.internalHighestUsed >= 0
                     ? "\(account.internalHighestUsed)" : "—")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Transactions:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(account.transactions.count)")
                    .fontWeight(.medium)
            }
            HStack {
                Text("UTXOs:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(account.utxos.count)")
                    .fontWeight(.medium)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func noteCard() -> some View {
        VStack(alignment: .leading, spacing: 8) {
            // TODO(platform-wallet): Expose per-address detail + WIF derivation
            // on PlatformWalletManager / ManagedPlatformWallet and bring back
            // the full address list / private-key peek UI here.
            Label("Address Details", systemImage: "info.circle")
                .font(.headline)
                .foregroundColor(.primary)
            Text("Per-address and private-key details are not yet exposed by the new PlatformWalletManager. They will return once the FFI surface is extended.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.secondarySystemBackground))
        .cornerRadius(12)
    }

    // MARK: - Helpers

    private var shouldShowBalance: Bool {
        ["Standard BIP44", "BIP32", "CoinJoin"].contains(account.accountTypeName)
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

    private func derivePrivateKeyWithPIN() async {
        // TODO(platform-wallet): needs new FFI for WIF derivation via
        // PlatformWalletManager. For now, surface a stubbed error.
        await MainActor.run {
            errorMessage = "Private key derivation is not yet available through the new PlatformWalletManager."
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
