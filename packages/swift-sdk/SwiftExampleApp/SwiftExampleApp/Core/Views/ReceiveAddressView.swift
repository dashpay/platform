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
    let wallet: HDWallet

    @Query private var bip44Accounts: [PersistentAccount]

    @State private var selectedTab: ReceiveAddressTab = .core
    @State private var copiedToClipboard = false

    init(wallet: HDWallet) {
        self.wallet = wallet
        let walletId = wallet.walletId
        // Primary BIP44 Standard account:
        //   accountType == 0  (AccountTypeTagFFI::Standard)
        //   accountIndex == 0 (first account)
        //   standardTag == 0  (BIP44)
        // Direct @Query over PersistentAccount is cheaper and more
        // robust than walking `PersistentWallet.accounts`, which can
        // lag on inverse-relationship updates across SwiftData
        // contexts.
        _bip44Accounts = Query(
            filter: #Predicate<PersistentAccount> {
                $0.wallet?.walletId == walletId
                    && $0.accountType == 0
                    && $0.accountIndex == 0
                    && $0.standardTag == 0
            }
        )
    }

    /// Lowest-indexed unused external address on the primary BIP44
    /// account. `PersistentCoreAddress` rows are populated by the Rust
    /// `on_persist_account_addresses_fn` callback at wallet creation
    /// (initial gap-limit fill), so they're available without a
    /// runtime FFI hop.
    private var nextCoreReceiveAddress: PersistentCoreAddress? {
        guard let account = bip44Accounts.first else { return nil }
        return account.coreAddresses
            .filter { $0.poolTypeTag == 0 && !$0.isUsed }
            .min(by: { $0.addressIndex < $1.addressIndex })
    }

    private var currentAddress: String {
        switch selectedTab {
        case .core:
            return nextCoreReceiveAddress?.address
                ?? "No unused receive address available yet — sync the wallet to extend the pool."
        case .platform:
            return "Platform receive addresses are not yet exposed via PlatformWalletManager."
        case .shielded:
            return shieldedService.orchardDisplayAddress ?? "Not available"
        }
    }

    private var hasValidAddress: Bool {
        switch selectedTab {
        case .core:
            return nextCoreReceiveAddress != nil
        case .platform:
            return false
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
}
