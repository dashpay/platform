import SwiftUI
import SwiftDashSDK

struct TransitionCategoryView: View {
    let category: StateTransitionsView.TransitionCategory
    @EnvironmentObject var appState: UnifiedAppState
    
    var transitions: [(key: String, label: String, description: String)] {
        switch category {
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
                ("documentPurchase", "Purchase Document", "Purchase a document")
            ]
        case .token:
            return [
                ("tokenMint", "Mint Tokens", "Create new tokens"),
                ("tokenBurn", "Burn Tokens", "Destroy existing tokens"),
                ("tokenTransfer", "Transfer Tokens", "Transfer tokens between identities"),
                ("tokenFreeze", "Freeze Tokens", "Freeze token transfers"),
                ("tokenUnfreeze", "Unfreeze Tokens", "Unfreeze token transfers"),
                ("tokenDestroyFrozen", "Destroy Frozen Tokens", "Destroy frozen tokens")
            ]
        case .voting:
            return [
                ("masternodeVote", "Cast Vote", "Vote on a governance proposal")
            ]
        }
    }
    
    var body: some View {
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

// Preview
struct TransitionCategoryView_Previews: PreviewProvider {
    static var previews: some View {
        NavigationView {
            TransitionCategoryView(category: .identity)
                .environmentObject(UnifiedAppState())
        }
    }
}