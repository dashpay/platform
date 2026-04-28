import SwiftUI
import SwiftData
import SwiftDashSDK
import CoreImage.CIFilterBuiltins

enum ReceiveAddressTab: String, CaseIterable {
    case core = "Core"
    case platform = "Platform"
    case shielded = "Shielded"
}

struct ReceiveAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    @EnvironmentObject var shieldedService: ShieldedService
    let wallet: PersistentWallet

    /// All persisted accounts across wallets. Filtered down to this
    /// view's wallet + primary BIP44 account inside
    /// `nextCoreReceiveAddress`. A `Query(filter:)` with the natural
    /// predicate exceeded Swift's type-checker budget, so filtering in
    /// Swift keeps compile times reasonable at negligible runtime
    /// cost (tens of accounts per store, not thousands).
    @Query private var allAccounts: [PersistentAccount]
    /// All PlatformPayment (DIP-17) addresses. Filtered down to this
    /// wallet in `nextPlatformReceiveAddress`.
    @Query private var platformAddresses: [PersistentPlatformAddress]

    @State private var selectedTab: ReceiveAddressTab = .core
    @State private var copiedToClipboard = false
    @State private var faucetStatus: String?
    @State private var isFaucetLoading = false

    /// Lowest-indexed unused external address on the primary BIP44
    /// account. `PersistentCoreAddress` rows are populated by the Rust
    /// `on_persist_account_addresses_fn` callback at wallet creation
    /// (initial gap-limit fill), so they're available without a
    /// runtime FFI hop.
    private var nextCoreReceiveAddress: PersistentCoreAddress? {
        guard let account = primaryBip44Account else { return nil }
        return firstUnusedAddress(in: account, poolTag: 0)
    }

    /// Lowest-indexed unused address on the primary PlatformPayment
    /// account. Queries the dedicated `PersistentPlatformAddress`
    /// store directly (address-emit populates it for type-14
    /// accounts). The previous implementation walked
    /// `PersistentAccount.coreAddresses` with a pool-tag filter; that
    /// is unnecessary now that Platform addresses have their own
    /// model.
    private var nextPlatformReceiveAddress: PersistentPlatformAddress? {
        let walletId = wallet.walletId
        var best: PersistentPlatformAddress? = nil
        for addr in platformAddresses {
            if addr.walletId != walletId { continue }
            if addr.isUsed { continue }
            if let current = best, current.addressIndex <= addr.addressIndex {
                continue
            }
            best = addr
        }
        return best
    }

    /// Primary Standard account (BIP44 or BIP32) for the active wallet,
    /// or nil if it hasn't been persisted yet. `standardTag` is not
    /// required to match — a wallet only ever has one `(accountType=0,
    /// accountIndex=0)` account, so the uniqueness already follows
    /// from the two upstream fields.
    private var primaryBip44Account: PersistentAccount? {
        findAccount(accountType: 0, accountIndex: 0)
    }

    private func findAccount(accountType: UInt32, accountIndex: UInt32) -> PersistentAccount? {
        let walletId = wallet.walletId
        for account in allAccounts {
            if account.wallet?.walletId != walletId { continue }
            if account.accountType != accountType { continue }
            if account.accountIndex != accountIndex { continue }
            return account
        }
        return nil
    }

    /// Lowest-indexed unused address in the given pool on the given
    /// account, or nil if the pool has no unused slots.
    private func firstUnusedAddress(
        in account: PersistentAccount,
        poolTag: UInt8
    ) -> PersistentCoreAddress? {
        var best: PersistentCoreAddress? = nil
        for addr in account.coreAddresses {
            if addr.poolTypeTag != poolTag { continue }
            if addr.isUsed { continue }
            if let current = best, current.addressIndex <= addr.addressIndex {
                continue
            }
            best = addr
        }
        return best
    }

    private var currentAddress: String {
        switch selectedTab {
        case .core:
            return nextCoreReceiveAddress?.address
                ?? "No unused receive address available yet — sync the wallet to extend the pool."
        case .platform:
            return nextPlatformReceiveAddress?.address
                ?? "No Platform receive address available yet — create a wallet after enabling Platform address persistence."
        case .shielded:
            return shieldedService.orchardDisplayAddress ?? "Not available"
        }
    }

    /// BIP32 derivation path for the currently-selected tab's address,
    /// or nil when the address didn't come out of an HD pool (Shielded
    /// today) or no address is available yet.
    private var currentDerivationPath: String? {
        switch selectedTab {
        case .core: return nextCoreReceiveAddress?.derivationPath
        case .platform: return nextPlatformReceiveAddress?.derivationPath
        case .shielded: return nil
        }
    }

    /// 33-byte compressed secp256k1 public key rendered as lowercase
    /// hex. `nil` when the entry was persisted without a public key
    /// (BLS accounts, script-only pool entries) or when the tab
    /// doesn't expose one.
    private var currentPublicKeyHex: String? {
        let bytes: Data?
        switch selectedTab {
        case .core: bytes = nextCoreReceiveAddress?.publicKey
        case .platform: bytes = nextPlatformReceiveAddress?.publicKey
        case .shielded: bytes = nil
        }
        guard let data = bytes, !data.isEmpty else { return nil }
        return data.map { String(format: "%02x", $0) }.joined()
    }

    private var hasValidAddress: Bool {
        switch selectedTab {
        case .core:
            return nextCoreReceiveAddress != nil
        case .platform:
            return nextPlatformReceiveAddress != nil
        case .shielded:
            return shieldedService.orchardDisplayAddress != nil
        }
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 20) {
                Picker("Address Type", selection: $selectedTab) {
                    ForEach(ReceiveAddressTab.allCases, id: \.self) { tab in
                        Text(tab.rawValue).tag(tab)
                    }
                }
                .pickerStyle(.segmented)
                .padding(.horizontal)

                if hasValidAddress {
                    if let qrImage = generateQRCode(from: currentAddress) {
                        Image(uiImage: qrImage)
                            .interpolation(.none)
                            .resizable()
                            .scaledToFit()
                            .frame(width: 250, height: 250)
                            .padding()
                            .background(Color.white)
                            .cornerRadius(12)
                    }

                    VStack(spacing: 12) {
                        Text(addressLabel)
                            .font(.subheadline)
                            .foregroundColor(.secondary)

                        Text(currentAddress)
                            .font(.system(.caption, design: .monospaced))
                            .multilineTextAlignment(.center)
                            .padding()
                            .background(Color(UIColor.secondarySystemBackground))
                            .cornerRadius(8)
                            .onTapGesture {
                                copyToClipboard(currentAddress)
                            }

                        if let path = currentDerivationPath {
                            HStack(spacing: 4) {
                                Text("Path")
                                    .foregroundColor(.secondary)
                                Text(path)
                                    .fontDesign(.monospaced)
                                    .textSelection(.enabled)
                            }
                            .font(.caption2)
                        }

                        if let pubKey = currentPublicKeyHex {
                            VStack(spacing: 2) {
                                Text("Public Key")
                                    .foregroundColor(.secondary)
                                Text(pubKey)
                                    .fontDesign(.monospaced)
                                    .textSelection(.enabled)
                                    .lineLimit(1)
                                    .truncationMode(.middle)
                            }
                            .font(.caption2)
                            .frame(maxWidth: .infinity)
                        }
                    }
                    .padding(.horizontal)

                    Button {
                        copyToClipboard(currentAddress)
                    } label: {
                        Label(
                            copiedToClipboard ? "Copied!" : "Copy Address",
                            systemImage: copiedToClipboard ? "checkmark" : "doc.on.doc"
                        )
                        .frame(maxWidth: .infinity)
                    }
                    .buttonStyle(.borderedProminent)
                    .tint(tabColor)
                    .padding(.horizontal)

                    // Faucet button — only on local Docker, Core tab
                    if selectedTab == .core && platformState.useDockerSetup {
                        Button {
                            Task { await requestFromFaucet() }
                        } label: {
                            HStack {
                                if isFaucetLoading {
                                    ProgressView().scaleEffect(0.8)
                                } else {
                                    Image(systemName: "drop.fill")
                                }
                                Text(faucetStatus ?? "Get 10 DASH from Faucet")
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.green)
                        .padding(.horizontal)
                        .disabled(isFaucetLoading)
                    }
                } else {
                    Spacer()
                    Text(currentAddress)
                        .font(.body)
                        .foregroundColor(.secondary)
                        .multilineTextAlignment(.center)
                        .padding()
                    Spacer()
                }

                Spacer()
            }
            .navigationTitle("Receive Dash")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .onChange(of: selectedTab) { _, _ in
                copiedToClipboard = false
            }
        }
    }

    private var addressLabel: String {
        switch selectedTab {
        case .core: return "Your Core Address"
        case .platform: return "Your Platform Address"
        case .shielded: return "Your Shielded Address"
        }
    }

    private var tabColor: Color {
        switch selectedTab {
        case .core: return .blue
        case .platform: return .indigo
        case .shielded: return .purple
        }
    }

    private func generateQRCode(from string: String) -> UIImage? {
        let context = CIContext()
        let filter = CIFilter.qrCodeGenerator()
        filter.message = Data(string.utf8)

        if let outputImage = filter.outputImage {
            let transform = CGAffineTransform(scaleX: 10, y: 10)
            let scaledImage = outputImage.transformed(by: transform)
            if let cgImage = context.createCGImage(scaledImage, from: scaledImage.extent) {
                return UIImage(cgImage: cgImage)
            }
        }
        return nil
    }

    private func copyToClipboard(_ string: String) {
        UIPasteboard.general.string = string
        copiedToClipboard = true
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            copiedToClipboard = false
        }
    }

    /// Request 10 DASH from the local Docker faucet (seed node Core RPC).
    private func requestFromFaucet() async {
        isFaucetLoading = true
        faucetStatus = nil
        defer { isFaucetLoading = false }

        let address = currentAddress
        guard !address.isEmpty else {
            faucetStatus = "No address available"
            return
        }

        // Read RPC port and password from UserDefaults, with dashmate defaults
        let rpcPort = UserDefaults.standard.string(forKey: "faucetRPCPort") ?? "20302"
        let rpcUser = UserDefaults.standard.string(forKey: "faucetRPCUser") ?? "dashmate"
        let rpcPassword = UserDefaults.standard.string(forKey: "faucetRPCPassword") ?? "dashmate"

        guard let url = URL(string: "http://127.0.0.1:\(rpcPort)/") else {
            faucetStatus = "Invalid RPC URL"
            return
        }

        let body: [String: Any] = [
            "jsonrpc": "1.0",
            "id": "faucet",
            "method": "sendtoaddress",
            "params": [address, 10]
        ]

        guard let jsonData = try? JSONSerialization.data(withJSONObject: body) else {
            faucetStatus = "Failed to encode request"
            return
        }

        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.httpBody = jsonData
        request.setValue("text/plain", forHTTPHeaderField: "Content-Type")

        let credentials = "\(rpcUser):\(rpcPassword)"
        if let credData = credentials.data(using: .utf8) {
            request.setValue("Basic \(credData.base64EncodedString())", forHTTPHeaderField: "Authorization")
        }

        do {
            let (data, response) = try await URLSession.shared.data(for: request)
            guard let httpResponse = response as? HTTPURLResponse else {
                faucetStatus = "Invalid response"
                return
            }

            if httpResponse.statusCode == 200 {
                if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let txid = json["result"] as? String {
                    faucetStatus = "Sent! tx: \(txid.prefix(12))..."
                } else {
                    faucetStatus = "Sent!"
                }
            } else if httpResponse.statusCode == 401 || httpResponse.statusCode == 403 {
                faucetStatus = "Auth failed — set faucetRPCPassword in UserDefaults"
            } else {
                let body = String(data: data, encoding: .utf8) ?? ""
                faucetStatus = "RPC error \(httpResponse.statusCode): \(body.prefix(80))"
            }
        } catch {
            faucetStatus = "Network error: \(error.localizedDescription)"
        }

        // Clear status after 5 seconds
        Task {
            try? await Task.sleep(nanoseconds: 5_000_000_000)
            faucetStatus = nil
        }
    }
}
