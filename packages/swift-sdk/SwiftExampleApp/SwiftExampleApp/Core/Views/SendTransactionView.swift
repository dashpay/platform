import SwiftUI
import SwiftDashSDK

struct SendTransactionView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @EnvironmentObject var shieldedService: ShieldedService
    let wallet: HDWallet

    @StateObject private var viewModel: SendViewModel

    init(wallet: HDWallet) {
        self.wallet = wallet
        // Default to testnet; the actual network is set in onAppear
        _viewModel = StateObject(wrappedValue: SendViewModel(network: wallet.network))
    }

    var body: some View {
        NavigationStack {
            Form {
                // Recipient Section
                Section {
                    TextField("Recipient Address", text: $viewModel.recipientAddress)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()

                    if !viewModel.recipientAddress.isEmpty {
                        AddressTypeBadge(type: viewModel.detectedAddressType)
                    }
                } header: {
                    Text("Recipient")
                }

                // Amount Section
                Section {
                    HStack {
                        TextField("0.00000000", text: $viewModel.amountString)
                            .keyboardType(.decimalPad)
                        Text("DASH")
                            .foregroundColor(.secondary)
                    }

                    // Available balances
                    VStack(alignment: .leading, spacing: 4) {
                        BalanceInfoRow(
                            label: "Shielded:",
                            amount: shieldedService.shieldedBalance,
                            color: .purple
                        )
                        BalanceInfoRow(
                            label: "Platform:",
                            amount: platformBalance,
                            color: .blue
                        )
                        BalanceInfoRow(
                            label: "Core:",
                            amount: coreBalance,
                            color: .primary
                        )
                    }
                } header: {
                    Text("Amount")
                }

                // Flow Detection Section
                if let flow = viewModel.detectedFlow {
                    Section {
                        HStack {
                            Image(systemName: flow.iconName)
                                .foregroundColor(flowColor(for: flow))
                            Text(flow.displayName)
                                .fontWeight(.medium)
                            Spacer()
                        }

                        if let fee = viewModel.estimatedFee {
                            HStack {
                                Text("Estimated Fee:")
                                Spacer()
                                Text("~\(formatBalance(fee))")
                                    .foregroundColor(.secondary)
                            }
                        }
                    } header: {
                        Text("Transaction Type")
                    }
                }

                // Source Toggle (for Orchard destination only)
                if case .orchard = viewModel.detectedAddressType {
                    Section {
                        Toggle("Send from Shielded Pool", isOn: $viewModel.preferShieldedSource)
                            .onChange(of: viewModel.preferShieldedSource) { _, _ in
                                viewModel.detectAddressType()
                            }
                    } header: {
                        Text("Source")
                    } footer: {
                        Text(viewModel.preferShieldedSource
                             ? "Shielded-to-shielded transfer (fully private)"
                             : "Shield credits from platform balance")
                    }
                }

                // Asset Lock (disabled)
                Section {
                    HStack {
                        Image(systemName: "lock.fill")
                            .foregroundColor(.gray)
                        Text("Asset Lock")
                            .foregroundColor(.gray)
                        Spacer()
                        Text("Coming Soon")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .padding(.horizontal, 8)
                            .padding(.vertical, 4)
                            .background(Color(UIColor.tertiarySystemBackground))
                            .cornerRadius(6)
                    }
                } header: {
                    Text("Other Options")
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
                            guard let sdk = unifiedAppState.sdk else { return }
                            await viewModel.executeSend(
                                sdk: sdk,
                                shieldedService: shieldedService,
                                platformState: unifiedAppState.platformState,
                                wallet: wallet
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
        }
    }

    // MARK: - Computed

    private var platformBalance: UInt64 {
        unifiedAppState.platformState.identities
            .filter {
                $0.walletId == wallet.walletId &&
                $0.network == wallet.network.rawValue
            }
            .reduce(0) { $0 + $1.balance }
    }

    private var coreBalance: UInt64 {
        walletService.walletManager.getBalance(for: wallet).confirmed
    }

    // MARK: - Helpers

    private func flowColor(for flow: SendFlow) -> Color {
        switch flow {
        case .platformToShielded: return .purple
        case .shieldedToShielded: return .purple
        case .shieldedToPlatform: return .blue
        case .shieldedToCore: return .green
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

// MARK: - Subviews

private struct AddressTypeBadge: View {
    let type: DashAddressType

    var body: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(badgeColor)
                .frame(width: 8, height: 8)
            Text(badgeText)
                .font(.caption)
                .fontWeight(.medium)
                .foregroundColor(badgeColor)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 4)
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

private struct BalanceInfoRow: View {
    let label: String
    let amount: UInt64
    var color: Color = .primary

    var body: some View {
        HStack {
            Text(label)
                .font(.caption)
                .foregroundColor(.secondary)
            Spacer()
            Text(formatBalance(amount))
                .font(.caption)
                .foregroundColor(amount > 0 ? color : .secondary)
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
