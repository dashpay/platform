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
        _viewModel = StateObject(wrappedValue: SendViewModel(network: wallet.network))
    }

    var body: some View {
        NavigationStack {
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
                        BalanceInfoRow(label: "Core:", amount: coreBalance, color: .green)
                        BalanceInfoRow(label: "Shielded:", amount: shieldedBalance, color: .purple)
                        BalanceInfoRow(label: "Platform:", amount: platformBalance, color: .blue)
                    }
                }

                // Fund Source
                if !availableSources.isEmpty {
                    Section("Send From") {
                        ForEach(availableSources) { source in
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
                                    Text(formatBalance(balance(for: source)))
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
                            guard let sdk = unifiedAppState.sdk else { return }
                            await viewModel.executeSend(
                                sdk: sdk,
                                shieldedService: shieldedService,
                                platformState: unifiedAppState.platformState,
                                wallet: wallet,
                                coreWallet: try? unifiedAppState.managedWallet?.coreWallet()
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

    private var coreBalance: UInt64 {
        walletService.walletManager.getBalance(for: wallet).confirmed
    }

    private var shieldedBalance: UInt64 {
        shieldedService.shieldedBalance
    }

    private var platformBalance: UInt64 {
        unifiedAppState.platformState.identities
            .filter {
                $0.walletId == wallet.walletId &&
                $0.network == wallet.network.rawValue
            }
            .reduce(0) { $0 + $1.balance }
    }

    private var availableSources: [FundSource] {
        viewModel.availableSources(
            coreBalance: coreBalance,
            shieldedBalance: shieldedBalance,
            platformBalance: platformBalance
        )
    }

    private func balance(for source: FundSource) -> UInt64 {
        switch source {
        case .core: return coreBalance
        case .shielded: return shieldedBalance
        case .platform: return platformBalance
        }
    }

    /// Auto-select the first available source when address type changes.
    private func autoSelectSource() {
        if let first = availableSources.first {
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

private struct BalanceInfoRow: View {
    let label: String
    let amount: UInt64
    var color: Color = .primary

    var body: some View {
        HStack {
            Text(label).font(.caption).foregroundColor(.secondary)
            Spacer()
            Text(formatBalance(amount))
                .font(.caption).foregroundColor(amount > 0 ? color : .secondary)
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
