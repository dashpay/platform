import SwiftUI
import CoreImage.CIFilterBuiltins

struct ReceiveAddressView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletService: WalletService
    let wallet: HDWallet
    
    @State private var currentAddress: String = ""
    @State private var isLoadingAddress = false
    @State private var copiedToClipboard = false
    
    var body: some View {
        NavigationStack {
            VStack(spacing: 24) {
                if isLoadingAddress {
                    ProgressView("Generating address...")
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                } else if !currentAddress.isEmpty {
                    VStack(spacing: 24) {
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
                        
                        // Address
                        VStack(spacing: 12) {
                            Text("Your Dash Address")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                            
                            Text(currentAddress)
                                .font(.system(.body, design: .monospaced))
                                .multilineTextAlignment(.center)
                                .padding()
                                .background(Color(UIColor.secondarySystemBackground))
                                .cornerRadius(8)
                                .onTapGesture {
                                    copyToClipboard()
                                }
                        }
                        .padding(.horizontal)
                        
                        // Copy Button
                        Button {
                            copyToClipboard()
                        } label: {
                            Label(
                                copiedToClipboard ? "Copied!" : "Copy Address",
                                systemImage: copiedToClipboard ? "checkmark" : "doc.on.doc"
                            )
                            .frame(maxWidth: .infinity)
                        }
                        .buttonStyle(.borderedProminent)
                        .padding(.horizontal)
                        
                        Spacer()
                    }
                }
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
        }
        .task {
            await loadAddress()
        }
    }
    
    private func loadAddress() async {
        isLoadingAddress = true
        
        // Try to get existing receive address or generate new one
        if let address = wallet.receiveAddress {
            currentAddress = address
        } else {
            do {
                currentAddress = try await walletService.getNewAddress()
            } catch {
                // Use a mock address for now
                currentAddress = "yMockReceiveAddress\(wallet.addresses.count)"
            }
        }
        
        isLoadingAddress = false
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
    
    private func copyToClipboard() {
        UIPasteboard.general.string = currentAddress
        copiedToClipboard = true
        
        // Reset after 2 seconds
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            copiedToClipboard = false
        }
    }
}