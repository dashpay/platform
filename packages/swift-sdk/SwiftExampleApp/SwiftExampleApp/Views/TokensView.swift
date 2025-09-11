import SwiftUI

// MARK: - View Extensions
extension View {
    func placeholder<Content: View>(
        when shouldShow: Bool,
        alignment: Alignment = .leading,
        @ViewBuilder placeholder: () -> Content) -> some View {
        
        ZStack(alignment: alignment) {
            placeholder().opacity(shouldShow ? 1 : 0)
            self
        }
    }
}

struct TokensView: View {
    @EnvironmentObject var appState: AppState
    @State private var selectedToken: TokenModel?
    @State private var selectedIdentity: IdentityModel?
    
    var body: some View {
        NavigationView {
            VStack {
                if appState.identities.isEmpty {
                    EmptyStateView(
                        systemImage: "person.3",
                        title: "No Identities",
                        message: "Add identities in the Identities tab to use tokens"
                    )
                } else {
                    List {
                        Section("Select Identity") {
                            Picker("Identity", selection: $selectedIdentity) {
                                Text("Select an identity").tag(nil as IdentityModel?)
                                ForEach(appState.identities) { identity in
                                    Text(identity.alias ?? identity.idString)
                                        .tag(identity as IdentityModel?)
                                }
                            }
                            .pickerStyle(MenuPickerStyle())
                        }
                        
                        if selectedIdentity != nil {
                            Section("Available Tokens") {
                                ForEach(appState.tokens) { token in
                                    TokenRow(token: token) {
                                        selectedToken = token
                                    }
                                }
                            }
                        }
                    }
                }
            }
            .navigationTitle("Tokens")
            .sheet(item: $selectedToken) { token in
                TokenActionsView(token: token, selectedIdentity: selectedIdentity)
                    .environmentObject(appState)
            }
            .onAppear {
                if appState.tokens.isEmpty {
                    loadSampleTokens()
                }
            }
        }
    }
    
    private func loadSampleTokens() {
        // Add sample tokens for demonstration
        appState.tokens = [
            TokenModel(
                id: "token1",
                contractId: "contract1",
                name: "Dash Platform Token",
                symbol: "DPT",
                decimals: 8,
                totalSupply: 1000000000000000,
                balance: 10000000000,
                frozenBalance: 250000000, // 2.5 DPT frozen
                availableClaims: [
                    ("Reward Distribution", 100000000), // 1 DPT
                    ("Airdrop #42", 50000000) // 0.5 DPT
                ],
                pricePerToken: 0.001
            ),
            TokenModel(
                id: "token2",
                contractId: "contract2",
                name: "Test Token",
                symbol: "TEST",
                decimals: 6,
                totalSupply: 500000000000,
                balance: 5000000,
                frozenBalance: 0,
                availableClaims: [],
                pricePerToken: 0.0001
            )
        ]
    }
}

struct TokenRow: View {
    let token: TokenModel
    let onTap: () -> Void
    
    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(token.name)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    Text(token.symbol)
                        .font(.subheadline)
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Text("Balance: \(token.formattedBalance)")
                        .font(.subheadline)
                        .foregroundColor(.blue)
                    
                    if token.frozenBalance > 0 {
                        Text("(\(token.formattedFrozenBalance) frozen)")
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                }
                
                HStack {
                    Text("Total Supply: \(token.formattedTotalSupply)")
                        .font(.caption)
                        .foregroundColor(.secondary)
                    
                    if !token.availableClaims.isEmpty {
                        Spacer()
                        Label("\(token.availableClaims.count)", systemImage: "gift")
                            .font(.caption)
                            .foregroundColor(.green)
                    }
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct TokenActionsView: View {
    let token: TokenModel
    let selectedIdentity: IdentityModel?
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var selectedAction: TokenAction?
    
    var body: some View {
        NavigationView {
            List {
                Section("Token Information") {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("Name:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(token.name)
                                .font(.subheadline)
                        }
                        HStack {
                            Text("Symbol:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(token.symbol)
                                .font(.subheadline)
                        }
                        HStack {
                            Text("Balance:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(token.formattedBalance)
                                .font(.subheadline)
                                .foregroundColor(.blue)
                        }
                    }
                }
                
                Section("Actions") {
                    ForEach(TokenAction.allCases, id: \.self) { action in
                        Button(action: {
                            if action.isEnabled {
                                selectedAction = action
                            }
                        }) {
                            HStack {
                                Image(systemName: action.systemImage)
                                    .frame(width: 24)
                                    .foregroundColor(action.isEnabled ? .blue : .gray)
                                
                                VStack(alignment: .leading) {
                                    Text(action.rawValue)
                                        .foregroundColor(action.isEnabled ? .primary : .gray)
                                    Text(action.description)
                                        .font(.caption)
                                        .foregroundColor(.secondary)
                                }
                                
                                Spacer()
                            }
                            .padding(.vertical, 4)
                        }
                        .disabled(!action.isEnabled)
                    }
                }
            }
            .navigationTitle(token.name)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarTrailing) {
                    Button("Done") {
                        dismiss()
                    }
                }
            }
            .sheet(item: $selectedAction) { action in
                TokenActionDetailView(
                    token: token,
                    action: action,
                    selectedIdentity: selectedIdentity
                )
                .environmentObject(appState)
            }
        }
    }
}

struct TokenActionDetailView: View {
    let token: TokenModel
    let action: TokenAction
    let selectedIdentity: IdentityModel?
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var isProcessing = false
    @State private var recipientId = ""
    @State private var amount = ""
    @State private var tokenNote = ""
    
    var body: some View {
        NavigationView {
            Form {
                Section("Selected Identity") {
                    if let identity = selectedIdentity {
                        VStack(alignment: .leading) {
                            Text(identity.alias ?? "Identity")
                                .font(.headline)
                            Text(identity.idString)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Text("Balance: \(identity.formattedBalance)")
                                .font(.subheadline)
                                .foregroundColor(.blue)
                        }
                    }
                }
                
                switch action {
                case .transfer:
                    Section("Transfer Details") {
                        TextField("Recipient Identity ID", text: $recipientId)
                            .textContentType(.none)
                            .autocapitalization(.none)
                        
                        TextField("Amount", text: $amount)
                            .keyboardType(.numberPad)
                        
                        TextField("Note (Optional)", text: $tokenNote)
                    }
                    
                case .mint:
                    Section("Mint Details") {
                        TextField("Amount", text: $amount)
                            .keyboardType(.numberPad)
                        
                        TextField("Recipient Identity ID (Optional)", text: $recipientId)
                            .textContentType(.none)
                            .autocapitalization(.none)
                    }
                    
                case .burn:
                    Section("Burn Details") {
                        TextField("Amount", text: $amount)
                            .keyboardType(.numberPad)
                        
                        Text("Warning: This action is irreversible")
                            .font(.caption)
                            .foregroundColor(.red)
                    }
                    
                case .claim:
                    Section("Claim Details") {
                        if token.availableClaims.isEmpty {
                            Text("No claims available at this time")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        } else {
                            Text("Available claims:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            
                            VStack(alignment: .leading, spacing: 8) {
                                ForEach(token.availableClaims, id: \.name) { claim in
                                    HStack {
                                        Text(claim.name)
                                        Spacer()
                                        let divisor = pow(10.0, Double(token.decimals))
                                        let claimAmount = Double(claim.amount) / divisor
                                        Text(String(format: "%.\(token.decimals)f %@", claimAmount, token.symbol))
                                            .foregroundColor(.green)
                                    }
                                }
                            }
                            .padding(.vertical, 4)
                            
                            Text("All available claims will be processed")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                    
                case .freeze:
                    Section("Freeze Details") {
                        TextField("Amount to Freeze", text: $amount)
                            .keyboardType(.numberPad)
                        
                        TextField("Reason (Optional)", text: $tokenNote)
                        
                        Text("Frozen tokens cannot be transferred until unfrozen")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                case .unfreeze:
                    Section("Unfreeze Details") {
                        if token.frozenBalance > 0 {
                            Text("Frozen Balance: \(token.formattedFrozenBalance)")
                                .font(.subheadline)
                                .foregroundColor(.orange)
                        } else {
                            Text("No frozen tokens available")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        
                        TextField("Amount to Unfreeze", text: $amount)
                            .keyboardType(.numberPad)
                            .disabled(token.frozenBalance == 0)
                        
                        Text("Unfrozen tokens will be available for use immediately")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                    
                case .destroyFrozenFunds:
                    Section("Destroy Frozen Funds") {
                        if token.frozenBalance > 0 {
                            Text("Frozen Balance: \(token.formattedFrozenBalance)")
                                .font(.subheadline)
                                .foregroundColor(.orange)
                        } else {
                            Text("No frozen tokens available")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }
                        
                        TextField("Amount to Destroy", text: $amount)
                            .keyboardType(.numberPad)
                        
                        Text("⚠️ This action permanently destroys frozen tokens")
                            .font(.caption)
                            .foregroundColor(.red)
                        
                        TextField("Confirmation Reason", text: $tokenNote)
                            .placeholder(when: tokenNote.isEmpty) {
                                Text("Required for audit trail")
                                    .foregroundColor(.secondary)
                            }
                    }
                    
                case .directPurchase:
                    Section("Direct Purchase") {
                        Text("Price: \(token.pricePerToken, specifier: "%.6f") DASH per \(token.symbol)")
                            .font(.subheadline)
                        
                        TextField("Amount to Purchase", text: $amount)
                            .keyboardType(.numberPad)
                        
                        if let purchaseAmount = Double(amount) {
                            let totalCost = purchaseAmount * token.pricePerToken
                            Text("Total Cost: \(totalCost, specifier: "%.6f") DASH")
                                .font(.caption)
                                .foregroundColor(.blue)
                        }
                        
                        if let identity = selectedIdentity {
                            Text("Available Balance: \(identity.formattedBalance)")
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                        
                        Text("Purchase will be deducted from your identity balance")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }
                }
                
                Section {
                    Button(action: {
                        Task {
                            isProcessing = true
                            await performTokenAction()
                            isProcessing = false
                            dismiss()
                        }
                    }) {
                        HStack {
                            Spacer()
                            if isProcessing {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle())
                            } else {
                                Text("Execute \(action.rawValue)")
                            }
                            Spacer()
                        }
                    }
                    .disabled(isProcessing || !isActionValid)
                }
            }
            .navigationTitle(action.rawValue)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .navigationBarLeading) {
                    Button("Cancel") {
                        dismiss()
                    }
                }
            }
        }
    }
    
    private var isActionValid: Bool {
        switch action {
        case .transfer:
            return !recipientId.isEmpty && !amount.isEmpty
        case .mint:
            return !amount.isEmpty
        case .burn, .freeze, .unfreeze, .directPurchase:
            return !amount.isEmpty
        case .destroyFrozenFunds:
            return !amount.isEmpty && !tokenNote.isEmpty
        case .claim:
            return true // Claims don't require input
        }
    }
    
    private func performTokenAction() async {
        guard appState.sdk != nil,
              selectedIdentity != nil else {
            appState.showError(message: "Please select an identity")
            return
        }
        
        do {
            switch action {
            case .transfer:
                guard !recipientId.isEmpty else {
                    throw TokenError.invalidRecipient
                }
                
                guard let transferAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                // In a real app, we would use the SDK's token transfer functionality
                appState.showError(message: "Transfer of \(transferAmount) \(token.symbol) tokens initiated")
                
            case .mint:
                guard let mintAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                // In a real app, we would use the SDK's token mint functionality
                appState.showError(message: "Minting \(mintAmount) \(token.symbol) tokens")
                
            case .burn:
                guard let burnAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                // In a real app, we would use the SDK's token burn functionality
                appState.showError(message: "Burning \(burnAmount) \(token.symbol) tokens")
                
            case .claim:
                // In a real app, we would fetch available claims and process them
                appState.showError(message: "Claiming available \(token.symbol) tokens from distributions")
                
            case .freeze:
                guard let freezeAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                // In a real app, we would use the SDK's token freeze functionality
                let reason = tokenNote.isEmpty ? "No reason provided" : tokenNote
                appState.showError(message: "Freezing \(freezeAmount) \(token.symbol) tokens. Reason: \(reason)")
                
            case .unfreeze:
                guard let unfreezeAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                // In a real app, we would use the SDK's token unfreeze functionality
                appState.showError(message: "Unfreezing \(unfreezeAmount) \(token.symbol) tokens")
                
            case .destroyFrozenFunds:
                guard let destroyAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                guard !tokenNote.isEmpty else {
                    throw TokenError.missingReason
                }
                
                // In a real app, we would use the SDK's destroy frozen funds functionality
                appState.showError(message: "Destroying \(destroyAmount) frozen \(token.symbol) tokens. Reason: \(tokenNote)")
                
            case .directPurchase:
                guard let purchaseAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }
                
                let cost = Double(purchaseAmount) * token.pricePerToken
                // In a real app, we would use the SDK's direct purchase functionality
                appState.showError(message: "Purchasing \(purchaseAmount) \(token.symbol) tokens for \(String(format: "%.6f", cost)) DASH")
            }
        } catch {
            appState.showError(message: "Failed to perform \(action.rawValue): \(error.localizedDescription)")
        }
    }
}

enum TokenError: LocalizedError {
    case invalidRecipient
    case invalidAmount
    case missingReason
    
    var errorDescription: String? {
        switch self {
        case .invalidRecipient:
            return "Please enter a valid recipient ID"
        case .invalidAmount:
            return "Please enter a valid amount"
        case .missingReason:
            return "Please provide a reason for this action"
        }
    }
}

struct EmptyStateView: View {
    let systemImage: String
    let title: String
    let message: String
    
    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: systemImage)
                .font(.system(size: 60))
                .foregroundColor(.gray)
            
            Text(title)
                .font(.title2)
                .fontWeight(.semibold)
            
            Text(message)
                .font(.body)
                .foregroundColor(.secondary)
                .multilineTextAlignment(.center)
                .padding(.horizontal)
        }
        .padding()
    }
}
