import SwiftUI
import SwiftDashSDK

struct TransitionCategoryView: View {
    let category: StateTransitionsView.TransitionCategory
    @EnvironmentObject var appState: AppState

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

                // Debug-only raw (private-key) forms. The production,
                // wallet-signed equivalents now live off the
                // `WalletDetailView` Platform Balance row's ⋯ menu:
                // Transfer Credits (ADDR-02, `TransferPlatformAddressView`)
                // and Withdraw to Core (ADDR-04,
                // `WithdrawPlatformAddressView`). These raw forms paste a
                // 64-char private key and exist only for low-level
                // debugging / arbitrary-address operations.
                //
                // Gated behind `#if DEBUG` so a Release/TestFlight build
                // can't direct users to paste a raw private key, bypassing
                // the `KeychainSigner` boundary the production sheets
                // enforce. The view definitions stay compiled (they live in
                // AddressQueriesView.swift); only these entry-point
                // NavigationLinks are debug-only.
                #if DEBUG
                Section {
                    NavigationLink(destination: TransferAddressFundsView()) {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("🧪 Transfer Address Funds (raw)")
                                .font(.headline)
                            Text("Debug-only: transfer credits between Platform addresses using a pasted private key. Production path: Wallet → Platform Balance → ⋯ → Transfer Credits.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(3)
                        }
                        .padding(.vertical, 4)
                    }

                    NavigationLink(destination: WithdrawAddressFundsView()) {
                        VStack(alignment: .leading, spacing: 8) {
                            Text("🧪 Withdraw Address Funds (raw)")
                                .font(.headline)
                            Text("Debug-only: withdraw credits from Platform to Core (L1) using a pasted private key. Production path: Wallet → Platform Balance → ⋯ → Withdraw to Core.")
                                .font(.caption)
                                .foregroundColor(.secondary)
                                .lineLimit(3)
                        }
                        .padding(.vertical, 4)
                    }
                } header: {
                    Text("Debug / Raw (private-key) forms")
                } footer: {
                    Text("These paste a raw 64-char private key and bypass the wallet signer. Use the production sheets off the wallet's Platform Balance row instead.")
                }
                #endif
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

            // Read-only COUNT aggregation query lives alongside the Document
            // builders so it's discoverable next to the document operations,
            // but routes to its own query view (it neither signs nor
            // broadcasts). Drives QA tests DOC-10/11/12.
            if category == .document {
                NavigationLink(destination: CountDocumentsView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Count Documents")
                            .font(.headline)
                        Text("Count documents (total, filtered by where, or grouped by group_by)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                .accessibilityIdentifier("transition.document.countDocuments")

                // Read-only SUM/AVERAGE aggregation query, sibling to the Count
                // view above. Routes to its own query view (it neither signs
                // nor broadcasts). Drives QA tests DOC-13/14.
                NavigationLink(destination: SumAverageDocumentsView()) {
                    VStack(alignment: .leading, spacing: 8) {
                        Text("Sum / Average Documents")
                            .font(.headline)
                        Text("Sum or average a numeric document property (total, filtered, or grouped)")
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(2)
                    }
                    .padding(.vertical, 4)
                }
                .accessibilityIdentifier("transition.document.sumAverageDocuments")
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
                .environmentObject(AppState())
        }
    }
}
