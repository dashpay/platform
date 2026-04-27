import SwiftUI
import SwiftData
import SwiftDashSDK

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
    @Query private var identities: [PersistentIdentity]

    /// Picker selection — `PersistentIdentity` is a `@Model` class,
    /// so `Identifiable` + `Hashable` come for free.
    @State private var selectedIdentity: PersistentIdentity?

    /// Row tapped in the token list. Drives the actions sheet.
    @State private var selectedBalance: PersistentTokenBalance?

    var body: some View {
        NavigationStack {
            VStack {
                if identities.isEmpty {
                    EmptyStateView(
                        systemImage: "person.3",
                        title: "No Identities",
                        message: "Add identities in the Identities tab to use tokens"
                    )
                } else {
                    List {
                        Section("Select Identity") {
                            Picker("Identity", selection: $selectedIdentity) {
                                Text("Select an identity").tag(nil as PersistentIdentity?)
                                ForEach(identities) { identity in
                                    Text(identity.alias ?? identity.identityIdBase58)
                                        .tag(identity as PersistentIdentity?)
                                }
                            }
                            .pickerStyle(MenuPickerStyle())
                        }

                        if let identity = selectedIdentity {
                            tokenList(for: identity)
                        }
                    }
                }
            }
            .navigationTitle("Tokens")
            .sheet(item: $selectedBalance) { balance in
                TokenActionsView(balance: balance, identity: selectedIdentity)
                    .environmentObject(appState)
            }
        }
    }

    @ViewBuilder
    private func tokenList(for identity: PersistentIdentity) -> some View {
        Section("Token Balances") {
            if identity.tokenBalances.isEmpty {
                Text("No token balances yet")
                    .foregroundColor(.secondary)
                    .font(.subheadline)
            } else {
                ForEach(identity.tokenBalances) { balance in
                    TokenRow(balance: balance) {
                        selectedBalance = balance
                    }
                }
            }
        }
    }
}

struct TokenRow: View {
    let balance: PersistentTokenBalance
    let onTap: () -> Void

    private var headlineName: String {
        balance.token?.name ?? balance.tokenName ?? "Token"
    }

    private var trailingSymbol: String? {
        balance.tokenSymbol ?? balance.token?.name
    }

    private var totalSupplyLine: String? {
        // Prefer the cap; fall back to the base supply. Both are
        // stored as strings on PersistentToken (they're bignum-safe).
        guard let token = balance.token else { return nil }
        if let cap = token.maxSupply, !cap.isEmpty {
            return "Max Supply: \(cap)"
        }
        if !token.baseSupply.isEmpty {
            return "Base Supply: \(token.baseSupply)"
        }
        return nil
    }

    var body: some View {
        Button(action: onTap) {
            VStack(alignment: .leading, spacing: 4) {
                HStack {
                    Text(headlineName)
                        .font(.headline)
                        .foregroundColor(.primary)
                    Spacer()
                    if let symbol = trailingSymbol {
                        Text(symbol)
                            .font(.subheadline)
                            .foregroundColor(.secondary)
                    }
                }

                HStack {
                    Text("Balance: \(balance.displayBalance)")
                        .font(.subheadline)
                        .foregroundColor(.blue)

                    if balance.frozen {
                        Text("(frozen)")
                            .font(.caption)
                            .foregroundColor(.orange)
                    }
                }

                if let supply = totalSupplyLine {
                    Text(supply)
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
            .padding(.vertical, 4)
        }
        .buttonStyle(PlainButtonStyle())
    }
}

struct TokenActionsView: View {
    let balance: PersistentTokenBalance
    let identity: PersistentIdentity?
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var selectedAction: TokenAction?

    private var displayName: String {
        balance.token?.name ?? balance.tokenName ?? "Token"
    }

    var body: some View {
        NavigationView {
            List {
                Section("Token Information") {
                    VStack(alignment: .leading, spacing: 8) {
                        HStack {
                            Text("Name:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(displayName)
                                .font(.subheadline)
                        }
                        if let symbol = balance.tokenSymbol {
                            HStack {
                                Text("Symbol:")
                                    .font(.caption)
                                    .foregroundColor(.secondary)
                                Text(symbol)
                                    .font(.subheadline)
                            }
                        }
                        HStack {
                            Text("Balance:")
                                .font(.caption)
                                .foregroundColor(.secondary)
                            Text(balance.displayBalance)
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
            .navigationTitle(displayName)
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
                    balance: balance,
                    action: action,
                    identity: identity
                )
                .environmentObject(appState)
            }
        }
    }
}

struct TokenActionDetailView: View {
    let balance: PersistentTokenBalance
    let action: TokenAction
    let identity: PersistentIdentity?
    @EnvironmentObject var appState: AppState
    @Environment(\.dismiss) var dismiss
    @State private var isProcessing = false
    @State private var recipientId = ""
    @State private var amount = ""
    @State private var tokenNote = ""

    private var displaySymbol: String {
        balance.tokenSymbol ?? balance.token?.name ?? "tokens"
    }

    var body: some View {
        NavigationView {
            Form {
                Section("Selected Identity") {
                    if let identity = identity {
                        VStack(alignment: .leading) {
                            Text(identity.alias ?? "Identity")
                                .font(.headline)
                            Text(identity.identityIdBase58)
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(1)
                                .truncationMode(.middle)
                            Text("Credits: \(identity.formattedBalance)")
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
                        // Claims are wired to distribution configs on
                        // `PersistentToken`; surfacing them is a
                        // separate feature. For now this action just
                        // forwards to the SDK stub.
                        Text("Submit a claim against any available distribution")
                            .font(.caption)
                            .foregroundColor(.secondary)
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
                        if balance.frozen {
                            Text("Balance: \(balance.displayBalance) (frozen)")
                                .font(.subheadline)
                                .foregroundColor(.orange)
                        } else {
                            Text("No frozen tokens available")
                                .font(.subheadline)
                                .foregroundColor(.secondary)
                        }

                        TextField("Amount to Unfreeze", text: $amount)
                            .keyboardType(.numberPad)
                            .disabled(!balance.frozen)

                        Text("Unfrozen tokens will be available for use immediately")
                            .font(.caption)
                            .foregroundColor(.secondary)
                    }

                case .destroyFrozenFunds:
                    Section("Destroy Frozen Funds") {
                        if balance.frozen {
                            Text("Balance: \(balance.displayBalance) (frozen)")
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
                        TextField("Amount to Purchase", text: $amount)
                            .keyboardType(.numberPad)

                        if let identity = identity {
                            Text("Available Credits: \(identity.formattedBalance)")
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
              identity != nil else {
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
                appState.showError(message: "Transfer of \(transferAmount) \(displaySymbol) initiated")

            case .mint:
                guard let mintAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }

                // In a real app, we would use the SDK's token mint functionality
                appState.showError(message: "Minting \(mintAmount) \(displaySymbol)")

            case .burn:
                guard let burnAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }

                // In a real app, we would use the SDK's token burn functionality
                appState.showError(message: "Burning \(burnAmount) \(displaySymbol)")

            case .claim:
                // In a real app, we would fetch available claims and process them
                appState.showError(message: "Claiming available \(displaySymbol) from distributions")

            case .freeze:
                guard let freezeAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }

                // In a real app, we would use the SDK's token freeze functionality
                let reason = tokenNote.isEmpty ? "No reason provided" : tokenNote
                appState.showError(message: "Freezing \(freezeAmount) \(displaySymbol). Reason: \(reason)")

            case .unfreeze:
                guard let unfreezeAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }

                // In a real app, we would use the SDK's token unfreeze functionality
                appState.showError(message: "Unfreezing \(unfreezeAmount) \(displaySymbol)")

            case .destroyFrozenFunds:
                guard let destroyAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }

                guard !tokenNote.isEmpty else {
                    throw TokenError.missingReason
                }

                // In a real app, we would use the SDK's destroy frozen funds functionality
                appState.showError(message: "Destroying \(destroyAmount) frozen \(displaySymbol). Reason: \(tokenNote)")

            case .directPurchase:
                guard let purchaseAmount = UInt64(amount) else {
                    throw TokenError.invalidAmount
                }

                // In a real app, we would use the SDK's direct purchase functionality
                appState.showError(message: "Purchasing \(purchaseAmount) \(displaySymbol) via direct-purchase flow")
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
