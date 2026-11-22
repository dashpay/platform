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
///
/// Funding: **Identity credits** debits the identity (`wallet.tokenPurchase`).
/// **Shielded pool** pays the same agreed total plus the pool-paid fee out of
/// the identity's wallet's shielded notes (account 0) through
/// `walletManager.shieldedTokenPurchaseFromPool` (type 28): Rust selects the
/// notes, proves, broadcasts and mints the tokens as a note to the wallet's
/// own address in the token's shielded pool. The wallet does not sync token
/// pool notes yet, so those tokens are not shown anywhere in the app. The
/// shielded choice is offered only when Rust reports the token can be bought
/// that way (`shieldedTokenPurchaseAvailability`: the token has a shielded
/// pool); identity funding stays available either way. A shielded purchase
/// that returns is submitted, not confirmed: the next shielded sync confirms
/// it.
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

    /// Loading / loaded / failed state of the token's configured
    /// direct-purchase price. Stays `.loading` until the SDK connects and the
    /// query resolves; on a resolved query with no price set it becomes
    /// `.loaded(nil)` and Buy stays disabled with a clear reason. A query
    /// *failure* is `.failed`, not `.loaded(nil)` — "couldn't fetch the price"
    /// must not read as "this token isn't for sale", and it offers a retry.
    private enum PriceState {
        case loading
        case loaded(TokenDirectPurchasePricing?)
        case failed
    }

    @State private var priceState: PriceState = .loading

    /// Where the purchase is paid from.
    private enum FundingSource: String, CaseIterable, Identifiable {
        case identityCredits = "Identity credits"
        case shieldedPool = "Shielded pool"

        var id: String { rawValue }
    }

    @State private var fundingSource: FundingSource = .identityCredits

    /// Rust's answer to whether this token can be bought from the shielded
    /// pool, `nil` while it is being asked. `.failed` when the question
    /// itself could not be asked (the manager is not configured).
    private enum ShieldedAvailabilityState: Equatable {
        case loading
        case loaded(PlatformWalletManager.ShieldedTokenPurchaseAvailability)
        case failed
    }

    @State private var shieldedAvailability: ShieldedAvailabilityState = .loading
    /// Message shown after a shielded purchase returns; dismissing it closes
    /// the form.
    @State private var shieldedOutcome: AlertMessage?

    private struct AlertMessage: Identifiable {
        let id = UUID()
        let message: String
    }

    var body: some View {
        Form {
            Section("Token") {
                LabeledContent("Token", value: token.displayName)
            }

            Section("Pay with") {
                if shieldedAvailability == .loaded(.available) {
                    Picker("Pay with", selection: $fundingSource) {
                        ForEach(FundingSource.allCases) { source in
                            Text(source.rawValue).tag(source)
                        }
                    }
                    .pickerStyle(.segmented)
                    .accessibilityIdentifier("tokenPurchase.fundingSource")
                } else {
                    LabeledContent("Pay with", value: FundingSource.identityCredits.rawValue)
                    shieldedUnavailableNote
                }
            }

            Section {
                switch fundingSource {
                case .identityCredits:
                    Text("Direct purchase debits credits from this identity in exchange for the requested tokens. The buyer pays the configured price set by the token's pricing controller.")
                        .font(.subheadline)
                case .shieldedPool:
                    Text("The price and a pool fee are paid from this wallet's shielded balance; no identity is involved. The tokens are minted to this wallet's own address in the token's shielded pool.")
                        .font(.subheadline)
                }
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

            if fundingSource == .shieldedPool {
                shieldedPoolSection
            }

            Section {
                Button {
                    submit()
                } label: {
                    HStack {
                        if isSubmitting {
                            ProgressView().controlSize(.small)
                            Text(fundingSource == .shieldedPool
                                ? "Proving and broadcasting…"
                                : "Submitting…")
                        } else {
                            Text("Buy")
                        }
                    }
                    .frame(maxWidth: .infinity)
                }
                .buttonStyle(.borderedProminent)
                .disabled(!canSubmit || isSubmitting)
                .accessibilityIdentifier("tokenPurchase.buyButton")
                if isSubmitting && fundingSource == .shieldedPool {
                    Text(PlatformWalletManager.isShieldedProverReady
                        ? "Building the shielded proofs can take up to a minute."
                        : "Preparing the prover, then building the shielded proofs. This can take a minute or more.")
                        .font(.caption)
                        .foregroundColor(.secondary)
                }
            }
        }
        // Switching the funding source mid-proof would only relabel a
        // purchase already under way.
        .disabled(isSubmitting)
        .navigationTitle("Direct Purchase")
        .navigationBarTitleDisplayMode(.inline)
        // Keyed on SDK connectivity so a form opened before the SDK finishes
        // connecting re-fetches once it does (an un-keyed `.task` would stick
        // with the initial run's result forever).
        .task(id: appState.sdk == nil) { await loadPrice() }
        .task { await loadShieldedAvailability() }
        .alert(item: $submitError) { msg in
            Alert(
                title: Text("Purchase failed"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK"))
            )
        }
        .alert(item: $shieldedOutcome) { msg in
            Alert(
                title: Text("Shielded purchase"),
                message: Text(msg.message),
                dismissButton: .default(Text("OK")) { dismiss() }
            )
        }
    }

    // MARK: - Shielded availability

    /// Why the shielded choice is not offered, from Rust's answer.
    @ViewBuilder
    private var shieldedUnavailableNote: some View {
        switch shieldedAvailability {
        case .loading:
            Text("Checking whether this token can be bought from the shielded pool…")
                .font(.caption)
                .foregroundColor(.secondary)
        case .loaded(.noShieldedPool):
            Text("This token has no shielded pool, so it can't be bought from the shielded pool.")
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("tokenPurchase.shieldedUnavailable")
        case .loaded(.configurationUnreadable), .failed:
            Text("Couldn't read this token's configuration, so paying from the shielded pool isn't offered.")
                .font(.caption)
                .foregroundColor(.red)
                .accessibilityIdentifier("tokenPurchase.shieldedUnavailable")
            Button("Retry") {
                Task { await loadShieldedAvailability() }
            }
            .font(.caption)
        case .loaded(.available):
            EmptyView()
        }
    }

    /// Ask Rust whether this token can be bought from the shielded pool. A
    /// funding choice the answer no longer allows falls back to identity
    /// credits.
    private func loadShieldedAvailability() async {
        shieldedAvailability = .loading
        guard let position = UInt16(exactly: token.position) else {
            shieldedAvailability = .loaded(.configurationUnreadable)
            fundingSource = .identityCredits
            return
        }
        do {
            let availability = try await walletManager.shieldedTokenPurchaseAvailability(
                contractId: token.contractId,
                tokenPosition: position
            )
            shieldedAvailability = .loaded(availability)
        } catch {
            shieldedAvailability = .failed
        }
        if shieldedAvailability != .loaded(.available) {
            fundingSource = .identityCredits
        }
    }

    // MARK: - Shielded pool section

    /// What the shielded funding draws on: the wallet's account 0 shielded
    /// balance as Rust pushes it, the consensus fee estimate for a single
    /// note spend, and where the tokens go.
    @ViewBuilder
    private var shieldedPoolSection: some View {
        Section("Shielded pool") {
            LabeledContent("Shielded balance") {
                if let balance = shieldedBalance {
                    Text("\(balance) credits")
                } else {
                    Text("Not known yet")
                        .foregroundColor(.secondary)
                }
            }
            .accessibilityIdentifier("tokenPurchase.shieldedBalance")
            if let fee = shieldedFeeEstimate {
                LabeledContent("Pool fee (estimate)", value: "\(fee) credits")
                if let total = priceWithFee(fee) {
                    LabeledContent("Price + fee", value: "\(total) credits")
                }
                Text("The estimate is for a purchase paid from one or two notes; more notes raise the fee.")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            Text("The purchased tokens go to this wallet's shielded token pool. The app does not show shielded token balances yet, so they will not appear here or in the token list.")
                .font(.caption)
                .foregroundColor(.secondary)
                .accessibilityIdentifier("tokenPurchase.shieldedTokensNote")
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

        case .failed:
            Text("Couldn't load the configured direct-purchase price.")
                .font(.caption)
                .foregroundColor(.red)
            Button("Retry") {
                Task { await loadPrice() }
            }
            .font(.caption)
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
        switch fundingSource {
        case .identityCredits:
            return managedWallet != nil
        case .shieldedPool:
            // Offered only on Rust's answer that the token has a shielded
            // pool. A shielded spend needs a known, non-zero balance, as the
            // send screen's shielded flows do; whether it covers price plus
            // fee is Rust's call (an insufficient-balance error).
            guard shieldedAvailability == .loaded(.available) else { return false }
            guard identity.wallet?.walletId != nil else { return false }
            guard let balance = shieldedBalance else { return false }
            return balance > 0
        }
    }

    /// Account 0's spendable shielded credits of the identity's wallet, or
    /// `nil` while not known. The shielded purchase spends from account 0.
    private var shieldedBalance: UInt64? {
        guard let walletId = identity.wallet?.walletId else { return nil }
        return walletManager.spendableShieldedCredits(walletId: walletId, account: 0)
    }

    /// The consensus fee a purchase paid from one or two notes adds on top
    /// of the price, computed in Rust at the network's protocol version.
    private var shieldedFeeEstimate: UInt64? {
        try? walletManager.estimateShieldedFee(kind: .tokenPurchaseFromPool)
    }

    /// The agreed total plus `fee`, for display, or `nil` without a total or
    /// on overflow.
    private func priceWithFee(_ fee: UInt64) -> UInt64? {
        guard let cost = expectedTotalCost else { return nil }
        let (total, overflow) = cost.addingReportingOverflow(fee)
        return overflow ? nil : total
    }

    // MARK: - Price fetch

    /// Fetch the token's configured direct-purchase price and model it as a
    /// ``TokenDirectPurchasePricing``. The canonical token id — which the price
    /// query is keyed by — is derived the same way `TokenActionPermissionsView`
    /// derives it (`calculateTokenId(contractId:position:)` on the base58
    /// contract id), *not* the persisted `(contractId + position)` composite.
    /// An invalid position resolves to `.loaded(nil)` (nothing a retry can
    /// fix); a failed id-derivation or query resolves to `.failed` so the user
    /// sees a retryable error instead of a false "no price configured"; a
    /// not-yet-connected SDK stays `.loading` — the `.task(id:)` key re-runs
    /// this once the SDK lands.
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
            priceState = .failed
        }
    }

    // MARK: - Submit

    private func submit() {
        if fundingSource == .shieldedPool {
            submitShielded()
            return
        }
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

    /// Buy from the identity's wallet's shielded notes (account 0) at the
    /// same agreed total the identity path submits. Rust selects the notes,
    /// proves, broadcasts and records the activity.
    private func submitShielded() {
        guard let walletId = identity.wallet?.walletId else {
            submitError = .init(message: "This identity is not linked to a loaded wallet.")
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
        let contractId = token.contractId
        let manager = walletManager

        Task {
            do {
                // Per-operation spend authority: the resolver fires for
                // this spend only.
                try await manager.shieldedTokenPurchaseFromPool(
                    walletId: walletId,
                    resolver: MnemonicResolver(),
                    account: 0,
                    contractId: contractId,
                    tokenPosition: position,
                    tokenCount: amount,
                    totalAgreedPrice: totalCost
                )
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.shieldedOutcome = .init(message: "Purchase submitted. Platform accepted it; the next shielded sync confirms it, and it shows as pending in Shielded Activity until then. Do not retry. The tokens go to this wallet's shielded token pool, which the app does not show yet.")
                }
            } catch let unconfirmed as PlatformWalletError
                where SendViewModel.mayHaveGoneThrough(unconfirmed) {
                // The spend may already be on chain and Rust keeps its notes
                // reserved: report it without inviting a retry, which would
                // sign a second purchase.
                await MainActor.run {
                    guard self.submitGeneration == gen else { return }
                    self.isSubmitting = false
                    self.shieldedOutcome = .init(message: "The purchase may have gone through; the next shielded sync confirms it. Do not retry.")
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
