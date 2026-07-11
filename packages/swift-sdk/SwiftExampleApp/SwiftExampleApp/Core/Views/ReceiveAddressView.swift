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
    @Environment(\.openURL) private var openURL
    @EnvironmentObject var walletManager: PlatformWalletManager
    @EnvironmentObject var platformState: AppState
    let wallet: PersistentWallet

    /// The single primary BIP44 account for this wallet.
    /// Predicate uses a single-hop relationship traversal for the
    /// wallet id; single-hop is well within the type-checker budget
    /// unlike the multi-condition relationship predicates that
    /// previously caused compiler timeouts.
    @Query private var bip44Accounts: [PersistentAccount]
    /// PlatformPayment (DIP-17) addresses for this wallet.
    @Query private var platformAddresses: [PersistentPlatformAddress]

    init(wallet: PersistentWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        _bip44Accounts = Query(
            filter: #Predicate<PersistentAccount> {
                $0.wallet.walletId == walletId &&
                $0.accountType == 0 &&
                $0.standardTag == 0
            },
            sort: \.accountIndex
        )
        _platformAddresses = Query(filter: PersistentPlatformAddress.predicate(walletId: walletId))
    }

    @State private var selectedTab: ReceiveAddressTab = .core
    @State private var selectedAccountIndex: UInt32 = 0
    @State private var copiedToClipboard = false
    @State private var faucetStatus: String?
    @State private var isFaucetLoading = false
    @State private var testnetFaucetStatus: String?
    @State private var isTestnetFaucetLoading = false

    /// Lowest-indexed external address on the primary BIP44 account
    /// that has never received an inbound transaction.
    /// `PersistentCoreAddress` rows are populated by the Rust
    /// `on_persist_account_address_pools_fn` callback at wallet creation
    /// (initial gap-limit fill), so they're available without a
    /// runtime FFI hop.
    private var nextCoreReceiveAddress: PersistentCoreAddress? {
        guard let account = primaryBip44Account else { return nil }
        return firstUnreceivedAddress(in: account, poolTag: 0)
    }

    /// Lowest-indexed unused address on the primary PlatformPayment
    /// account. Queries the dedicated `PersistentPlatformAddress`
    /// store directly (address-emit populates it for type-14
    /// accounts). The previous implementation walked
    /// `PersistentAccount.coreAddresses` with a pool-tag filter; that
    /// is unnecessary now that Platform addresses have their own
    /// model.
    private var nextPlatformReceiveAddress: PersistentPlatformAddress? {
        platformAddresses
            .filter { !$0.isUsed }
            .min(by: { $0.addressIndex < $1.addressIndex })
    }

    private var primaryBip44Account: PersistentAccount? {
        bip44Accounts.first { $0.accountIndex == selectedAccountIndex }
            ?? bip44Accounts.first
    }

    /// Lowest-indexed address in the given pool on the given account
    /// that has never received an inbound transaction. `PersistentTxo`
    /// rows are created on the SPV inbound-UTXO path and only ever
    /// flagged spent (never deleted), so `addr.txos.isEmpty` is a
    /// reliable "never received" signal — strictly stronger than the
    /// `isUsed` flag, which doesn't always survive sync edge cases.
    private func firstUnreceivedAddress(
        in account: PersistentAccount,
        poolTag: UInt8
    ) -> PersistentCoreAddress? {
        var best: PersistentCoreAddress? = nil
        for addr in account.coreAddresses {
            if addr.poolTypeTag != poolTag { continue }
            if !addr.txos.isEmpty { continue }
            if let current = best, current.addressIndex <= addr.addressIndex {
                continue
            }
            best = addr
        }
        return best
    }

    /// Orchard receive address for THE WALLET BEING VIEWED (account 0),
    /// resolved per-wallet from the engine rather than the single-mirror
    /// `shieldedService`. Every loaded wallet is engine-bound
    /// (`rebindWalletScopedServices`), so `shieldedDefaultAddress`
    /// resolves for any of them; `nil` until this wallet's bind lands.
    private var shieldedReceiveAddress: String? {
        walletManager.shieldedDisplayAddress(
            walletId: wallet.walletId,
            network: platformState.currentNetwork
        )
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
            return shieldedReceiveAddress ?? "Not available"
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
            return shieldedReceiveAddress != nil
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

                if selectedTab == .core && bip44Accounts.count > 1 {
                    if bip44Accounts.count <= 4 {
                        Picker("Account", selection: $selectedAccountIndex) {
                            ForEach(bip44Accounts, id: \.accountIndex) { account in
                                Text("Account \(account.accountIndex)").tag(account.accountIndex)
                            }
                        }
                        .pickerStyle(.segmented)
                        .padding(.horizontal)
                    } else {
                        Picker("Account", selection: $selectedAccountIndex) {
                            ForEach(bip44Accounts, id: \.accountIndex) { account in
                                Text("Account \(account.accountIndex)").tag(account.accountIndex)
                            }
                        }
                        .pickerStyle(.menu)
                        .padding(.horizontal)
                    }
                }

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

                    // Dev tool: public testnet faucet (faucet.thepasta.org).
                    // Only on the Core tab and only when the SDK is on
                    // testnet — hidden on mainnet/devnet/regtest.
                    if selectedTab == .core && platformState.currentNetwork == .testnet {
                        Button {
                            Task { await requestFromTestnetFaucet() }
                        } label: {
                            HStack {
                                if isTestnetFaucetLoading {
                                    ProgressView().scaleEffect(0.8)
                                } else {
                                    Image(systemName: "wrench.and.screwdriver.fill")
                                }
                                Text(testnetFaucetStatus ?? "Get 1 tDASH — Testnet Faucet")
                            }
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(.orange)
                        .padding(.horizontal)
                        .disabled(isTestnetFaucetLoading)
                        .accessibilityIdentifier("receive.testnetFaucetButton")
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
            .onChange(of: selectedAccountIndex) { _, _ in
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

    /// Dev/QA tool: request 1 tDASH from the public testnet faucet
    /// (`faucet.thepasta.org`). Solves the cap.js proof-of-work captcha
    /// on-device. On any failure (rate limit, network, solve, non-200) we
    /// fall back to copying the address and opening the web faucet rather
    /// than failing silently.
    private func requestFromTestnetFaucet() async {
        isTestnetFaucetLoading = true
        testnetFaucetStatus = "Solving captcha…"
        defer { isTestnetFaucetLoading = false }

        let address = currentAddress
        guard hasValidAddress, !address.isEmpty else {
            testnetFaucetStatus = "No address available"
            clearTestnetFaucetStatusSoon()
            return
        }

        let outcome = await TestnetFaucet().requestCoreDash(address: address)
        switch outcome {
        case .sent(let txid, let amount):
            testnetFaucetStatus = "Sent \(formattedAmount(amount)) tDASH! tx: \(txid.prefix(12))…"
        case .rateLimited(let message):
            openWebFaucetFallback(address: address, reason: message)
        case .failed(let reason):
            openWebFaucetFallback(address: address, reason: reason)
        }
        clearTestnetFaucetStatusSoon()
    }

    /// Web fallback for any rate-limit/failure: the faucet page does not
    /// prefill `?address=`, so we copy the Core receive address to the
    /// clipboard and open the faucet in the browser. The `reason` is folded
    /// into the toast so the failure isn't silently swallowed.
    private func openWebFaucetFallback(address: String, reason: String) {
        UIPasteboard.general.string = address
        openURL(TestnetFaucet.webURL)
        testnetFaucetStatus = "\(reason) — opened web faucet (address copied)"
    }

    private func formattedAmount(_ amount: Double) -> String {
        amount == amount.rounded()
            ? String(format: "%.0f", amount)
            : String(format: "%g", amount)
    }

    private func clearTestnetFaucetStatusSoon() {
        Task {
            try? await Task.sleep(nanoseconds: 5_000_000_000)
            testnetFaucetStatus = nil
        }
    }
}
