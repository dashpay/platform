import SwiftUI
import SwiftDashSDK
import CoreImage.CIFilterBuiltins

enum ReceiveAddressTab: String, CaseIterable {
    case core = "Core"
    case platform = "Platform"
    case shielded = "Shielded"
}

struct ReceiveAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletService: WalletService
    @EnvironmentObject var unifiedAppState: UnifiedAppState
    @EnvironmentObject var shieldedService: ShieldedService
    let wallet: HDWallet

    @State private var selectedTab: ReceiveAddressTab = .core
    @State private var copiedToClipboard = false

    private var currentAddress: String {
        switch selectedTab {
        case .core:
            return walletService.walletManager.getReceiveAddress(for: wallet)
        case .platform:
            return platformAddress ?? "No platform identity"
        case .shielded:
            return shieldedService.orchardDisplayAddress ?? "Not available"
        }
    }

    private var platformAddress: String? {
        // Get the next receive address from the wallet's DIP-17 platform payment account
        // Address encoding is done in Rust via DPP's PlatformAddress::to_bech32m_string
        do {
            try walletService.walletManager.ensurePlatformPaymentAccount(for: wallet)
            let collection = try walletService.walletManager.getManagedAccountCollection(for: wallet)
            guard let platformAccount = collection.getPlatformPaymentAccount(accountIndex: 0, keyClass: 0) else {
                return nil
            }
            guard let pool = platformAccount.getAddressPool() else { return nil }
            let addresses = try pool.getAddresses(from: 0, to: 1)
            guard let addrInfo = addresses.first else { return nil }

            // Encode scriptPubKey as bech32m platform address via Rust FFI
            let network = unifiedAppState.platformState.currentNetwork
            let networkValue: UInt32 = {
                switch network {
                case .mainnet: return 0
                case .testnet: return 1
                case .regtest: return 2
                case .devnet: return 3
                }
            }()

            let result = addrInfo.scriptPubKey.withUnsafeBytes { buffer -> DashSDKResult in
                guard let base = buffer.baseAddress?.assumingMemoryBound(to: UInt8.self) else {
                    return DashSDKResult()
                }
                return dash_sdk_encode_platform_address(base, UInt32(addrInfo.scriptPubKey.count), networkValue)
            }

            guard result.error == nil, let dataPtr = result.data else {
                if let error = result.error {
                    dash_sdk_error_free(error)
                }
                return nil
            }

            let cStr = dataPtr.assumingMemoryBound(to: CChar.self)
            let addressString = String(cString: cStr)
            dash_sdk_string_free(UnsafeMutablePointer(mutating: cStr))
            return addressString
        } catch {
            return nil
        }
    }

    private var hasValidAddress: Bool {
        switch selectedTab {
        case .core:
            return true
        case .platform:
            return platformAddress != nil
        case .shielded:
            return shieldedService.orchardDisplayAddress != nil
        }
    }

    var body: some View {
        NavigationStack {
            VStack(spacing: 20) {
                // Segmented picker
                Picker("Address Type", selection: $selectedTab) {
                    ForEach(ReceiveAddressTab.allCases, id: \.self) { tab in
                        Text(tab.rawValue).tag(tab)
                    }
                }
                .pickerStyle(.segmented)
                .padding(.horizontal)

                if hasValidAddress {
                    // QR Code
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

                    // Address label
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
                    }
                    .padding(.horizontal)

                    // Copy Button
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
                } else {
                    Spacer()
                    Text(unavailableMessage)
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

    private var unavailableMessage: String {
        switch selectedTab {
        case .core: return "No core address available"
        case .platform: return "No platform payment account available.\nEnsure the wallet has a DIP-17 account."
        case .shielded: return "Shielded pool not initialized.\nA wallet with a valid seed is required."
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
}
