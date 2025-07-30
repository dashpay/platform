import SwiftUI

struct SendTransactionView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletService: WalletService
    let wallet: HDWallet
    
    @State private var recipientAddress = ""
    @State private var amountString = ""
    @State private var memo = ""
    @State private var isSending = false
    @State private var error: Error?
    @State private var successTxid: String?
    
    private var amount: UInt64? {
        guard let double = Double(amountString) else { return nil }
        return UInt64(double * 100_000_000) // Convert DASH to duffs
    }
    
    private var canSend: Bool {
        !recipientAddress.isEmpty &&
        amount != nil &&
        amount! > 0 &&
        amount! <= wallet.confirmedBalance
    }
    
    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("Recipient Address", text: $recipientAddress)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                } header: {
                    Text("Recipient")
                }
                
                Section {
                    HStack {
                        TextField("0.00000000", text: $amountString)
                            .keyboardType(.decimalPad)
                        
                        Text("DASH")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Available:")
                        Spacer()
                        Text(formatBalance(wallet.confirmedBalance))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                } header: {
                    Text("Amount")
                } footer: {
                    if let amount = amount, amount > wallet.confirmedBalance {
                        Text("Insufficient balance")
                            .foregroundColor(.red)
                    }
                }
                
                Section {
                    TextField("Optional message", text: $memo)
                } header: {
                    Text("Memo (Optional)")
                }
                
                Section {
                    HStack {
                        Text("Network Fee:")
                        Spacer()
                        Text("~0.00001000 DASH")
                            .foregroundColor(.secondary)
                    }
                }
            }
            .navigationTitle("Send Dash")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
                
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Send") {
                        sendTransaction()
                    }
                    .disabled(!canSend || isSending)
                }
            }
            .disabled(isSending)
            .overlay {
                if isSending {
                    ProgressView("Sending transaction...")
                        .padding()
                        .background(Color.gray.opacity(0.9))
                        .cornerRadius(10)
                }
            }
            .alert("Error", isPresented: .constant(error != nil)) {
                Button("OK") {
                    error = nil
                }
            } message: {
                if let error = error {
                    Text(error.localizedDescription)
                }
            }
            .alert("Success", isPresented: .constant(successTxid != nil)) {
                Button("Done") {
                    dismiss()
                }
            } message: {
                if successTxid != nil {
                    Text("Transaction sent successfully!")
                }
            }
        }
    }
    
    private func sendTransaction() {
        guard let amount = amount else { return }
        
        isSending = true
        
        Task {
            do {
                let txid = try await walletService.sendTransaction(
                    to: recipientAddress,
                    amount: amount,
                    memo: memo.isEmpty ? nil : memo
                )
                
                await MainActor.run {
                    successTxid = txid
                }
            } catch {
                await MainActor.run {
                    self.error = error
                    isSending = false
                }
            }
        }
    }
    
    private func formatBalance(_ amount: UInt64) -> String {
        let dash = Double(amount) / 100_000_000.0
        return String(format: "%.8f DASH", dash)
    }
}