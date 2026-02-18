import SwiftUI
import SwiftDashSDK

struct SendTransactionView: View {
    @Environment(\.dismiss) private var dismiss
    @EnvironmentObject var walletService: WalletService
    let wallet: HDWallet
    
    @State private var recipientAddress = ""
    @State private var amountString = ""
    @State private var fee: UInt64 = 0
    @State private var error: Error?
    
    @State private var tx: Data? = nil
    
    private var feeString: String {
        return formatDash(fee)
    }
    
    private var amount: UInt64 {
        guard let double = Double(amountString) else { return 0 }
        return UInt64(double * 100_000_000) // Convert DASH to duffs
    }
    
    private var canSend: Bool {
        !recipientAddress.isEmpty &&
        amount > 0 &&
        amount + fee <= balance.spendable &&
        tx != nil
    }
    
    private var balance: Balance {
        walletService.walletManager.getBalance(for: wallet, accType: .standardBIP44, accIndex: 0)
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
                        TextField("0", text: $amountString)
                            .keyboardType(.decimalPad)
                        
                        Text("DASH")
                            .foregroundColor(.secondary)
                    }
                    
                    HStack {
                        Text("Available:")
                        Spacer()
                        Text(formatDash(balance.spendable))
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                } header: {
                    Text("Amount")
                } footer: {
                    if amount > balance.spendable {
                        Text("Insufficient balance")
                            .foregroundColor(.red)
                    }
                }
                
                Section {
                    HStack {
                        Text("Network Fee:")
                        Spacer()
                        Text(feeString)
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
                    .disabled(!canSend)
                }
            }
            .onChange(of: recipientAddress) {
                recalculateTransaction()
            }
            .onChange(of: amountString) {
                recalculateTransaction()
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
        }
    }
    
    private func recalculateTransaction() {
        guard !recipientAddress.isEmpty,
              amount > 0,
              amount <= balance.spendable
        else {
            self.fee = 0
            self.tx = nil
            return
        }
    
        do {
            let (tx, fee) = try createTransaction()
            
            self.fee = fee
            self.tx = tx
        } catch {
            self.fee = 0
            self.tx = nil
        }
    }
    
    private func sendTransaction() {
        guard canSend else { return }
        guard let tx else { return }
    
        do {
            try walletService.broadcastTransaction(tx)
            dismiss()
    
        } catch {
            self.error = error
        }
    }
    
    private func createTransaction() throws -> (Data, UInt64) {
        let outputs = [
            Transaction.Output(address: recipientAddress, amount: amount)
        ]

        return try walletService.walletManager
            .buildSignedTransaction(
                for: wallet,
                accIndex: 0,
                outputs: outputs,
                feeRate: .normal
            )
    }
    
    private func formatDash(_ dash: UInt64) -> String {
        if dash == 0 {
            return "0 DASH"
        }
        
        let dashPart = dash / 100_000_000
        let decimalPart = dash % 100_000_000
        return String(format: "~%llu.%08llu DASH", dashPart, decimalPart)
    }
}