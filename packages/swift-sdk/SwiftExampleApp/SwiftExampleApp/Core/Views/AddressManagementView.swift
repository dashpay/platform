import SwiftUI

struct AddressManagementView: View {
    @EnvironmentObject var walletService: WalletService
    let account: HDAccount
    @State private var selectedType: AddressType = .external
    @State private var isGenerating = false
    @State private var error: Error?
    
    var body: some View {
        VStack(spacing: 0) {
            // Address Type Picker
            Picker("Address Type", selection: $selectedType) {
                Text("External").tag(AddressType.external)
                Text("Internal").tag(AddressType.internal)
                Text("CoinJoin").tag(AddressType.coinJoin)
                Text("Identity").tag(AddressType.identity)
            }
            .pickerStyle(.segmented)
            .padding()
            
            // Address List
            List {
                ForEach(addressesForType(selectedType)) { address in
                    AddressDetailRow(address: address)
                }
                
                // Generate More Button
                Section {
                    Button {
                        generateMoreAddresses()
                    } label: {
                        HStack {
                            Image(systemName: "plus.circle.fill")
                            Text("Generate More Addresses")
                        }
                    }
                    .disabled(isGenerating)
                }
            }
            .listStyle(.grouped)
        }
        .navigationTitle("Address Management")
        .navigationBarTitleDisplayMode(.inline)
        .alert("Error", isPresented: .constant(error != nil)) {
            Button("OK") {
                error = nil
            }
        } message: {
            if let error = error {
                Text(error.localizedDescription)
            }
        }
    }
    
    private func addressesForType(_ type: AddressType) -> [HDAddress] {
        switch type {
        case .external:
            return account.externalAddresses
        case .internal:
            return account.internalAddresses
        case .coinJoin:
            return account.coinJoinAddresses
        case .identity:
            return account.identityFundingAddresses
        }
    }
    
    private func generateMoreAddresses() {
        isGenerating = true
        
        Task {
            do {
                try await walletService.generateAddresses(for: account, count: 10, type: selectedType)
                await MainActor.run {
                    isGenerating = false
                }
            } catch {
                await MainActor.run {
                    self.error = error
                    isGenerating = false
                }
            }
        }
    }
}

struct AddressDetailRow: View {
    let address: HDAddress
    @State private var copied = false
    
    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack {
                VStack(alignment: .leading, spacing: 4) {
                    Text("Address #\(address.index)")
                        .font(.headline)
                    
                    Text(address.derivationPath)
                        .font(.caption2)
                        .foregroundColor(.secondary)
                }
                
                Spacer()
                
                VStack(alignment: .trailing, spacing: 4) {
                    if address.isUsed {
                        Label("Used", systemImage: "checkmark.circle.fill")
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                    
                    if address.balance > 0 {
                        Text(formatBalance(address.balance))
                            .font(.caption)
                            .fontWeight(.medium)
                    }
                }
            }
            
            HStack {
                Text(address.address)
                    .font(.system(.caption, design: .monospaced))
                    .foregroundColor(.secondary)
                    .lineLimit(1)
                    .truncationMode(.middle)
                
                Button {
                    copyAddress()
                } label: {
                    Image(systemName: copied ? "checkmark" : "doc.on.doc")
                        .foregroundColor(.accentColor)
                }
                .buttonStyle(.plain)
            }
            
            if let lastSeenTime = address.lastSeenTime {
                Text("Last seen: \(lastSeenTime, style: .relative)")
                    .font(.caption2)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.vertical, 4)
    }
    
    private func copyAddress() {
        UIPasteboard.general.string = address.address
        copied = true
        
        Task {
            try? await Task.sleep(nanoseconds: 2_000_000_000)
            copied = false
        }
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}