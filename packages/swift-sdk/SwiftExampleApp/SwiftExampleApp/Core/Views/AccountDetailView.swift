import SwiftUI
import SwiftDashSDK
import SwiftData

// MARK: - Account Detail View
struct AccountDetailView: View {
    @EnvironmentObject var appUIState: AppUIState
    let wallet: PersistentWallet
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

                    if account.accountType == 14 {
                        // PlatformPayment accounts keep their address
                        // list in `platformAddresses`, with no
                        // external/internal pool split.
                        let sorted = account.platformAddresses.sorted {
                            $0.addressIndex < $1.addressIndex
                        }
                        if sorted.isEmpty {
                            emptyAddressesCard()
                        } else {
                            platformAddressListCard(addresses: sorted)
                        }
                    } else {
                        ForEach(addressSections(), id: \.0) { name, addresses in
                            addressListCard(
                                name: name,
                                systemImage: poolIcon(for: name),
                                addresses: addresses
                            )
                        }

                        if account.coreAddresses.isEmpty {
                            emptyAddressesCard()
                        }
                    }
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
                    Text(wallet.network.capitalized)
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
        // PlatformPayment balances are in credits (1e11/DASH), live on
        // `PersistentPlatformAddress.balance`, and have no
        // confirmed / pending split on-chain.
        if account.accountType == 14 {
            return AnyView(platformBalanceCard())
        }

        return AnyView(VStack(alignment: .leading, spacing: 12) {
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
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2))
    }

    private func platformBalanceCard() -> some View {
        let total = account.platformAddresses.reduce(0) { $0 + $1.balance }
        return VStack(alignment: .leading, spacing: 12) {
            Label("Balance", systemImage: "creditcard")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            HStack {
                Text("Platform Credits")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text(formatCredits(total))
                    .font(.title3)
                    .fontWeight(.semibold)
            }

            HStack {
                Text("Raw Credits")
                    .font(.caption)
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(total)")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func poolSummaryCard() -> some View {
        // DIP-17 PlatformPayment pools are flat — there's no
        // external/internal split, no on-chain transactions, and no
        // UTXOs. Surface only address-pool cardinality instead.
        if account.accountType == 14 {
            return AnyView(platformPoolSummaryCard())
        }

        let externalCount = account.coreAddresses.filter { $0.poolTypeTag == 0 }.count
        let internalCount = account.coreAddresses.filter { $0.poolTypeTag == 1 }.count
        return AnyView(VStack(alignment: .leading, spacing: 12) {
            Label("Address Pool", systemImage: "square.stack.3d.up.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            if externalCount > 0 {
                HStack {
                    Text("Pool Size (External):")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(externalCount)")
                        .fontWeight(.medium)
                }
            }
            if internalCount > 0 {
                HStack {
                    Text("Pool Size (Internal):")
                        .foregroundColor(.secondary)
                    Spacer()
                    Text("\(internalCount)")
                        .fontWeight(.medium)
                }
            }
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
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2))
    }

    /// Platform-payment-specific pool summary. Shows total vs used
    /// addresses plus the highest-seen index, and omits external /
    /// internal / transactions / UTXOs (not meaningful for DIP-17).
    private func platformPoolSummaryCard() -> some View {
        let total = account.platformAddresses.count
        let used = account.platformAddresses.filter { $0.isUsed }.count
        let highest = account.platformAddresses
            .filter { $0.isUsed }
            .map { $0.addressIndex }
            .max()
        return VStack(alignment: .leading, spacing: 12) {
            Label("Address Pool", systemImage: "square.stack.3d.up.fill")
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            HStack {
                Text("Total Addresses:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(total)")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Addresses Used:")
                    .foregroundColor(.secondary)
                Spacer()
                Text("\(used)")
                    .fontWeight(.medium)
            }
            HStack {
                Text("Highest Used:")
                    .foregroundColor(.secondary)
                Spacer()
                Text(highest.map { "\($0)" } ?? "—")
                    .fontWeight(.medium)
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    /// Group this account's persisted addresses by pool tag, in a
    /// stable display order (External → Internal → Absent → Absent
    /// Hardened). Empty pools are skipped.
    private func addressSections() -> [(String, [PersistentCoreAddress])] {
        let grouped = Dictionary(grouping: account.coreAddresses) { $0.poolTypeTag }
        let order: [(UInt8, String)] = [
            (0, "External"),
            (1, "Internal"),
            (2, "Absent"),
            (3, "Absent (Hardened)"),
        ]
        return order.compactMap { tag, name in
            guard let bucket = grouped[tag], !bucket.isEmpty else { return nil }
            let sorted = bucket.sorted { $0.addressIndex < $1.addressIndex }
            return (name, sorted)
        }
    }

    private func addressListCard(
        name: String,
        systemImage: String,
        addresses: [PersistentCoreAddress]
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label("\(name) Addresses (\(addresses.count))", systemImage: systemImage)
                .font(.headline)
                .foregroundColor(.primary)

            Divider()

            ForEach(Array(addresses.enumerated()), id: \.element.id) { idx, addr in
                NavigationLink(destination: CoreAddressDetailView(record: addr)) {
                    addressRow(addr)
                }
                .buttonStyle(.plain)
                if idx < addresses.count - 1 {
                    Divider()
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    /// DIP-17 Platform Payment address list — single section (no
    /// external/internal split). Rows link to `PlatformAddressDetailView`
    /// so all the platform-specific fields (bech32m string, nonce,
    /// hash, etc.) are reachable from here.
    private func platformAddressListCard(
        addresses: [PersistentPlatformAddress]
    ) -> some View {
        VStack(alignment: .leading, spacing: 12) {
            Label(
                "Platform Addresses (\(addresses.count))",
                systemImage: "creditcard"
            )
            .font(.headline)
            .foregroundColor(.primary)

            Divider()

            ForEach(Array(addresses.enumerated()), id: \.element.id) { idx, addr in
                NavigationLink(destination: PlatformAddressDetailView(record: addr)) {
                    platformAddressRow(addr)
                }
                .buttonStyle(.plain)
                if idx < addresses.count - 1 {
                    Divider()
                }
            }
        }
        .padding()
        .background(Color(.systemBackground))
        .cornerRadius(12)
        .shadow(color: Color.black.opacity(0.05), radius: 5, x: 0, y: 2)
    }

    private func platformAddressRow(_ addr: PersistentPlatformAddress) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(addr.address)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.primary)
                HStack(spacing: 6) {
                    Text("#\(addr.addressIndex)")
                    if addr.isUsed { Text("• used") }
                    if addr.balance > 0 {
                        Text("• \(addr.balance) credits")
                    }
                }
                .font(.caption2)
                .foregroundColor(.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .foregroundColor(.secondary)
                .font(.caption)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
    }

    private func addressRow(_ addr: PersistentCoreAddress) -> some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(addr.address)
                    .font(.system(.caption, design: .monospaced))
                    .lineLimit(1)
                    .truncationMode(.middle)
                    .foregroundColor(.primary)
                HStack(spacing: 6) {
                    Text("#\(addr.addressIndex)")
                    if addr.isUsed { Text("• used") }
                    if addr.balance > 0 {
                        Text("• \(formatBalance(addr.balance))")
                    }
                }
                .font(.caption2)
                .foregroundColor(.secondary)
            }
            Spacer()
            Image(systemName: "chevron.right")
                .foregroundColor(.secondary)
                .font(.caption)
        }
        .padding(.vertical, 4)
        .contentShape(Rectangle())
    }

    private func emptyAddressesCard() -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Label("Addresses", systemImage: "info.circle")
                .font(.headline)
                .foregroundColor(.primary)
            Text("No addresses have been persisted for this account yet. They land here after the wallet is (re)created via `PlatformWalletManager`.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding()
        .background(Color(.secondarySystemBackground))
        .cornerRadius(12)
    }

    private func poolIcon(for name: String) -> String {
        switch name {
        case "External": return "arrow.down.circle"
        case "Internal": return "arrow.triangle.2.circlepath"
        default: return "square.stack"
        }
    }

    // MARK: - Helpers

    private var shouldShowBalance: Bool {
        switch account.accountType {
        case 0, 1, 14: return true
        default: return false
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

    /// Credits (1e11 per DASH) → DASH string. Used for the
    /// PlatformPayment balance row.
    private func formatCredits(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000_000.0
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
