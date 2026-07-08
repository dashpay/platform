import SwiftUI
import SwiftData
import SwiftDashSDK

/// Form for buying tokens at the configured direct-purchase price.
///
/// Inputs: amount of tokens to buy. The configured direct-purchase price is
/// fetched when the view appears via `SDK.getTokenDirectPurchasePrices` (keyed
/// by the canonical token id derived with `calculateTokenId`, mirroring how
/// `TokenActionPermissionsView` fetches live token state), then modelled as a
/// ``TokenDirectPurchasePricing``. The Buy button computes
/// `expectedTotalCost = pricing.cost(forAmount:)` client-side — the *same*
/// tier rule Drive uses to validate the purchase — so the submitted total
/// equals the chain's `required_price`.
///
/// Buy stays disabled while the price loads, when the token has no
/// direct-purchase price configured, and when the entered amount isn't
/// purchasable at the configured price (below the minimum, a free tier, or an
/// overflowing total). Direct Purchase is not group-gated, so there's no
/// group-action banner.
struct TokenPurchaseActionView: View {
    let token: PersistentToken
    let identity: PersistentIdentity

    @EnvironmentObject var appState: AppState
    @EnvironmentObject var walletManager: PlatformWalletManager
    @Environment(\.modelContext) private var modelContext
    @Environment(\.dismiss) private var dismiss

    @State private var amountText: String = ""
    @State private var isSubmitting: Bool = false
    @State private var submitError: AlertMessage?
    /// Generation counter so a late `MainActor.run` from a previous
    /// `submit()` Task can't write back to a re-entered view instance
    /// after the user pops + repushes mid-broadcast.
    @State private var submitGeneration: Int = 0

    /// Loading / loaded state of the token's configured direct-purchase price.
    /// Stays `.loading` until the SDK connects and the query resolves; on a
    /// resolved query with no price set it becomes `.loaded(nil)` and Buy stays
    /// disabled with a clear reason.
    private enum PriceState {
        case loading
        case loaded(TokenDirectPurchasePricing?)
    }

    @State private var priceState: PriceState = .loading

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
            }

            Section {
                Text("Direct purchase debits credits from this identity in exchange for the requested tokens. The buyer pays the configured price set by the token's pricing controller.")
                    .font(.subheadline)
            }

            Section("Amount") {
                TextField("Tokens to buy", text: $amountText)
                    .keyboardType(.decimalPad)
                if let amount = parsedAmount, amount == 0 {
                    Text("Amount must be greater than zero.")
                        .font(.caption)
                        .foregroundColor(.red)
                }
            }

            Section("Total cost") {
                priceStatus
            }

            Section {
                Button {
                    submit()
                } label: {
                    HStack {
                        if isSubmitting {
                            ProgressView().controlSize(.small)
                            Text("Submitting…")
                        } else {
                            Text("Buy")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
            }
        }
        .navigationTitle("Direct Purchase")
        .navigationBarTitleDisplayMode(.inline)
        // Keyed on SDK connectivity so a form opened before the SDK finishes
        // connecting re-fetches once it does (an un-keyed `.task` would stick
        // with the initial run's result forever).
        .task(id: appState.sdk == nil) { await loadPrice() }
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Purchase failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
    }

    // MARK: - Total-cost section

    /// The "Total cost" section body, driven by the price-load state, the
    /// entered amount, and whether that amount resolves to a submittable cost.
    @ViewBuilder
    private var priceStatus: some View {
        switch priceState {
        case .loading:
            Text("Loading the configured direct-purchase price…")
                .font(.caption)
                .foregroundColor(.secondary)

        case let .loaded(pricing):
            if let pricing {
                loadedPriceStatus(pricing)
            } else {
                Text("This token has no direct-purchase price configured, so it can't be bought directly.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
    }

    /// The "Total cost" body once a pricing schedule is known: prompt for an
    /// amount, show a resolved cost, or explain why the entered amount can't be
    /// purchased (under-minimum hint or a generic reason).
    @ViewBuilder
    private func loadedPriceStatus(_ pricing: TokenDirectPurchasePricing) -> some View {
        if let amount = parsedAmount, amount > 0 {
            if let cost = pricing.cost(forAmount: amount) {
                Text("\(cost) credits")
                    .font(.body)
            } else if amount < pricing.minimumPurchaseAmount {
                Text("The minimum direct purchase for this token is \(formatTokenAmount(pricing.minimumPurchaseAmount, decimals: token.decimals)).")
                    .font(.caption)
                    .foregroundColor(.red)
            } else {
                Text("This amount can't be purchased at the configured price.")
                    .font(.caption)
                    .foregroundColor(.red)
            }
        } else {
            Text("Enter an amount to see the total cost.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
    }

    // MARK: - Derived state

    private var managedWallet: ManagedPlatformWallet? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.wallet(for: walletId)
    }

    /// Tokens-to-buy is in display units; the FFI takes raw u64.
    private var parsedAmount: UInt64? {
        parseTokenAmount(amountText, decimals: token.decimals)
    }

    /// The pricing schedule once loaded (`nil` while loading or when the token
    /// has no configured price).
    private var pricing: TokenDirectPurchasePricing? {
        if case let .loaded(pricing) = priceState { return pricing }
        return nil
    }

    /// The required total the purchase must pay, computed with the same tier
    /// rule Drive validates against, or `nil` when the entered amount isn't
    /// purchasable at the configured price.
    private var expectedTotalCost: UInt64? {
        guard let pricing, let amount = parsedAmount else { return nil }
        return pricing.cost(forAmount: amount)
    }

    private var canSubmit: Bool {
        guard let amount = parsedAmount, amount > 0 else { return false }
        guard expectedTotalCost != nil else { return false }
        return managedWallet != nil
    }

    // MARK: - Price fetch

    /// Fetch the token's configured direct-purchase price and model it as a
    /// ``TokenDirectPurchasePricing``. The canonical token id — which the price
    /// query is keyed by — is derived the same way `TokenActionPermissionsView`
    /// derives it (`calculateTokenId(contractId:position:)` on the base58
    /// contract id), *not* the persisted `(contractId + position)` composite.
    /// An invalid position or a failed query resolves to `.loaded(nil)` so Buy
    /// is disabled with a clear reason rather than left spinning; a
    /// not-yet-connected SDK stays `.loading` — the `.task(id:)` key re-runs
    /// this once the SDK lands, so "no price configured" is never shown for a
    /// price that simply hasn't been fetchable yet.
    private func loadPrice() async {
        priceState = .loading

        guard let sdk = appState.sdk else { return }
        guard let position = UInt16(exactly: token.position) else {
            priceState = .loaded(nil)
            return
        }

        let contractIdString = token.contractId.toBase58String()
        do {
            let canonicalTokenId = try sdk.calculateTokenId(
                contractId: contractIdString,
                position: position
            )
            let response = try await sdk.getTokenDirectPurchasePrices(
                tokenIds: [canonicalTokenId]
            )
            let pricing = TokenDirectPurchasePricing.parse(
                response,
                canonicalTokenId: canonicalTokenId
            )
            priceState = .loaded(pricing)
        } catch {
            print("⚠️ TokenPurchaseActionView: failed to load direct-purchase price for \(contractIdString):\(token.position): \(error)")
            priceState = .loaded(nil)
        }
    }

    // MARK: - Submit

    private func submit() {
        guard let wallet = managedWallet else {
            submitError = .init(message: "The wallet that owns this identity isn't loaded.")
            return
        }
        guard let amount = parsedAmount, amount > 0 else {
            submitError = .init(message: "Amount must be greater than zero.")
            return
        }
        guard let totalCost = expectedTotalCost else {
            submitError = .init(message: "This amount can't be purchased at the configured price.")
            return
        }

        guard let position = UInt16(exactly: token.position) else {
            submitError = .init(message: "Invalid token position.")
            return
        }

        isSubmitting = true
        submitGeneration &+= 1
        let gen = submitGeneration
        let signer = KeychainSigner(modelContainer: modelContext.container)
        let identityId = identity.identityId
        let contractId = token.contractId
        // `cost(forAmount:)` applies the exact tier rule Drive validates
        // against, so this equals the chain's `required_price`.
        let expectedTotalCost = totalCost

        Task {
            do {
                try await wallet.tokenPurchase(
                    identityId: identityId,
                    contractId: contractId,
                    tokenPosition: position,
                    amount: amount,
                    expectedTotalCost: expectedTotalCost,
                    signer: signer
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.dismiss()
                }
            } catch {
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.submitError = .init(message: error.localizedDescription)
                    self.isSubmitting = false
                }
            }
        }
    }
}
