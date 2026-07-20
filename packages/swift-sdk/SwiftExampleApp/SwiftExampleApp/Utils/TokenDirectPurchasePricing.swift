import Foundation

/// A token's direct-purchase pricing schedule, parsed from the JSON that
/// `SDK.getTokenDirectPurchasePrices(tokenIds:)` returns (the shape emitted
/// by the FFI's `dash_sdk_token_get_direct_purchase_prices`):
///
/// ```
/// {"<tokenIdBase58>": {"type":"single_price","price":<u64>}}
/// {"<tokenIdBase58>": {"type":"set_prices","prices":[{"amount":<u64>,"price":<u64>}, …]}}
/// {"<tokenIdBase58>": null}
/// ```
///
/// This is the model behind the Direct Purchase form's cost preview. It
/// resolves the total credits a buyer must pay for a given raw token amount
/// using the *same* tier rule Drive applies when it validates the purchase
/// (`token_direct_purchase_transition_action` v0 transformer), so the
/// `expectedTotalCost` the form submits matches the chain's `required_price`:
///
///  - ``singlePrice(_:)``: `price × amount`.
///  - ``setPrices(_:)``: the highest tier threshold `≤ amount` sets the
///    per-token price and the total is `tierPrice × amount`. An amount below
///    the smallest threshold is under the minimum sale amount (not
///    purchasable); an empty schedule means the token isn't for direct sale.
///
/// Prices are `u64` credits on-chain, so multiplication goes through
/// `multipliedReportingOverflow`; a product that overflows `UInt64` resolves
/// to `nil` (Buy stays disabled) rather than wrapping. A resolved cost of `0`
/// (a free tier) is likewise `nil`: the purchase FFI/flow requires a positive
/// `expectedTotalCost`.
enum TokenDirectPurchasePricing: Equatable {

    /// One flat per-token price for any amount.
    case singlePrice(UInt64)

    /// Tiered pricing: an ordered list of `(minimum amount, per-token price)`
    /// tiers. The highest tier whose `amount ≤` the purchased amount sets the
    /// per-token price.
    case setPrices([Tier])

    /// A `(minimum amount, per-token price)` tier in a ``setPrices(_:)``
    /// schedule.
    struct Tier: Equatable {
        let amount: UInt64
        let price: UInt64
    }

    /// The required total cost in credits to buy `amount` raw token units, or
    /// `nil` when the amount isn't purchasable at this schedule (below the
    /// minimum sale amount, a free tier, or a total that overflows the
    /// `UInt64` credits the purchase FFI accepts).
    ///
    /// This mirrors Drive's `token_direct_purchase_transition_action` v0
    /// transformer: `required_price = perTokenPrice × token_count`, where for
    /// a tiered schedule the per-token price is the highest tier threshold
    /// `≤ amount`.
    ///
    /// One deliberate divergence: Drive's single-price branch uses
    /// `saturating_mul`, so an overflowing product passes its price check at
    /// `u64::MAX` — but Drive then debits that `u64::MAX`, which exceeds any
    /// possible credit balance, so the transition is guaranteed to fail at
    /// balance deduction (after charging a processing fee). Broadcasting a
    /// known-doomed purchase is worse than disabling Buy, so this helper
    /// treats overflow as not-purchasable in both branches (as the Kotlin
    /// counterpart does — its `Long`-based FFI can't even represent
    /// `u64::MAX`).
    func cost(forAmount amount: UInt64) -> UInt64? {
        guard amount > 0 else { return nil }

        let perToken: UInt64
        switch self {
        case let .singlePrice(price):
            perToken = price
        case let .setPrices(tiers):
            // Highest tier threshold `≤ amount` — the Rust
            // `set_prices.range(..=token_count).next_back()` lookup. An amount
            // below every threshold (or an empty schedule) is not purchasable.
            guard let tierPrice = tiers
                .filter({ $0.amount <= amount })
                .max(by: { $0.amount < $1.amount })?
                .price
            else { return nil }
            perToken = tierPrice
        }

        let (total, overflow) = perToken.multipliedReportingOverflow(by: amount)
        guard !overflow else { return nil }
        // A resolved total of 0 (a free tier) is not submittable: the purchase
        // FFI/flow requires a positive expected total cost.
        guard total > 0 else { return nil }
        return total
    }

    /// The smallest raw amount that can be purchased — `1` for a single price,
    /// the lowest tier threshold for a tiered schedule. Used for the "minimum
    /// purchase is N" hint.
    var minimumPurchaseAmount: UInt64 {
        switch self {
        case .singlePrice:
            return 1
        case let .setPrices(tiers):
            return tiers.map { $0.amount }.min() ?? 1
        }
    }

    /// Parse the pricing for `canonicalTokenId` out of the dictionary a
    /// `getTokenDirectPurchasePrices` call produces (a `[String: Any]` from
    /// `JSONSerialization`), or `nil` when the token has no usable price (a
    /// `null`/missing entry, an empty tier list) or the entry can't be read.
    ///
    /// `price`/`amount` values are `u64` and can exceed `Int64.max`, so they
    /// are read through `NSNumber.uint64Value` rather than as `Int`.
    static func parse(
        _ response: [String: Any],
        canonicalTokenId: String
    ) -> TokenDirectPurchasePricing? {
        // A missing key or an explicit JSON `null` (bridged to `NSNull`) both
        // mean "no price configured".
        guard let entry = response[canonicalTokenId] as? [String: Any] else {
            return nil
        }

        switch entry["type"] as? String {
        case "single_price":
            guard let price = (entry["price"] as? NSNumber)?.uint64Value else {
                return nil
            }
            return .singlePrice(price)

        case "set_prices":
            guard let rawTiers = entry["prices"] as? [[String: Any]] else {
                return nil
            }
            let tiers: [Tier] = rawTiers.compactMap { tier in
                guard let amount = (tier["amount"] as? NSNumber)?.uint64Value,
                      let price = (tier["price"] as? NSNumber)?.uint64Value
                else { return nil }
                return Tier(amount: amount, price: price)
            }
            // An empty schedule (or one whose every tier failed to parse) means
            // the token isn't for direct sale.
            return tiers.isEmpty ? nil : .setPrices(tiers)

        default:
            return nil
        }
    }
}
