import SwiftUI
import SwiftDashSDK

struct TransitionCategoryView: View {
    let category: StateTransitionsView.TransitionCategory
    @EnvironmentObject var appState: UnifiedAppState
    
    var transitions: [(key: String, label: String, description: String)] {
        switch category {
        case .address:
            // Address transitions use dedicated SwiftUI flows (not the generic TransitionDetailView).
            return []
        case .identity:
            return [
                ("identityCreate", "Create Identity", "Create a new identity with initial credits"),
                ("identityTopUp", "Top Up Identity", "Add credits to an existing identity"),
                ("identityUpdate", "Update Identity", "Update identity properties and keys"),
                ("identityCreditTransfer", "Transfer Credits", "Transfer credits between identities"),
                ("identityCreditWithdrawal", "Withdraw Credits", "Withdraw credits to a Dash address")
            ]
        case .dataContract:
            return [
                ("dataContractCreate", "Create Contract", "Deploy a new data contract"),
                ("dataContractUpdate", "Update Contract", "Update an existing data contract")
            ]
        case .document:
            return [
                ("documentCreate", "Create Document", "Create a new document"),
                ("documentReplace", "Replace Document", "Replace an existing document"),
                ("documentDelete", "Delete Document", "Delete a document"),
                ("documentTransfer", "Transfer Document", "Transfer document ownership"),
                ("documentUpdatePrice", "Update Price", "Update document sale price"),
                ("documentPurchase", "Purchase Document", "Purchase a document")
            ]
        case .token:
            return [
                ("tokenMint", "Mint Tokens", "Create new tokens"),
                ("tokenBurn", "Burn Tokens", "Destroy existing tokens"),
                ("tokenTransfer", "Transfer Tokens", "Transfer tokens between identities"),
                ("tokenClaim", "Claim Tokens", "Claim tokens from a distribution"),
                ("tokenFreeze", "Freeze Tokens", "Freeze token transfers"),
                ("tokenUnfreeze", "Unfreeze Tokens", "Unfreeze token transfers"),
                ("tokenDestroyFrozenFunds", "Destroy Frozen Tokens", "Destroy frozen tokens"),
                ("tokenSetPrice", "Set Token Price", "Set or update token pricing")
            ]
        case .voting:
            return [
                ("masternodeVote", "Cast Vote", "Vote on a governance proposal")
            ]
        }
    }
    
    var body: some View {
        if category == .address {
            List {
                NavigationLink(destination: TransferAddressFundsView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Transfer Address Funds")
                            .font(.headline)
                        Text("Transfer credits between Platform addresses")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                
                NavigationLink(destination: WithdrawAddressFundsView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Withdraw Address Funds")
                            .font(.headline)
                        Text("Withdraw credits from Platform to Core (L1)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                
                NavigationLink(destination: TopUpAddressFromAssetLockView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Top Up Address (Asset Lock)")
                            .font(.headline)
                        Text("Fund Platform addresses from Dash Core asset lock")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                
                NavigationLink(destination: TopUpIdentityFromAddressesView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Top Up Identity (From Addresses)")
                            .font(.headline)
                        Text("Top up identity using Platform address balances")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                
                NavigationLink(destination: TransferIdentityToAddressesView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Transfer Identity → Addresses")
                            .font(.headline)
                        Text("Transfer credits from identity to Platform addresses")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                
                NavigationLink(destination: CreateIdentityFromAddressesView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Create Identity (From Addresses)")
                            .font(.headline)
                        Text("Create identity funded by Platform addresses")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
            }
            .navigationTitle(category.rawValue)
            .navigationBarTitleDisplayMode(.inline)
        } else {
        List {
            ForEach(transitions, id: \.key) { transition in
                NavigationLink(destination: TransitionDetailView(
                    transitionKey: transition.key,
                    transitionLabel: transition.label
                )) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text(transition.label)
                            .font(.headline)
                        Text(transition.description)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
            }
        }
        .navigationTitle(category.rawValue)
        .navigationBarTitleDisplayMode(.inline)
        }
    }
}

// Preview
struct TransitionCategoryView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            TransitionCategoryView(category: .identity)
                .environmentObject(UnifiedAppState())
        }
    }
}