import XCTest
@testable import SwiftExampleApp

/// Parsing + cost-resolution tests for `TokenDirectPurchasePricing` — the
/// model behind the Direct Purchase form's cost preview. The `cost(forAmount:)`
/// expectations mirror Drive's `token_direct_purchase_transition_action` v0
/// transformer (`required_price = perTokenPrice × token_count`, highest tier
/// `≤ amount`, under-minimum / empty schedule reject).
final class TokenDirectPurchasePricingTests: XCTestCase {

    private let tokenId = "TokenIdBase58AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"

    /// Parse the pricing out of a raw JSON body the same way the app does at
    /// runtime: `getTokenDirectPurchasePrices` returns a `[String: Any]`
    /// produced by `JSONSerialization`, so route the fixture through it to get
    /// the exact `NSNumber` bridging the parser sees.
    private func parse(_ json: String) -> TokenDirectPurchasePricing? {
        guard let data = json.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data),
              let dictionary = object as? [String: Any]
        else { return nil }
        return TokenDirectPurchasePricing.parse(dictionary, canonicalTokenId: tokenId)
    }

    private func single(_ price: String) -> String {
        #"{"\#(tokenId)":{"type":"single_price","price":\#(price)}}"#
    }

    private func setPrices(_ tiers: [(amount: UInt64, price: UInt64)]) -> String {
        let entries = tiers
            .map { #"{"amount":\#($0.amount),"price":\#($0.price)}"# }
            .joined(separator: ",")
        return #"{"\#(tokenId)":{"type":"set_prices","prices":[\#(entries)]}}"#
    }

    // MARK: - Parsing

    func test_singlePrice_parses() {
        XCTAssertEqual(parse(single("100")), .singlePrice(100))
    }

    func test_setPrices_parsePreservingTiers() {
        let pricing = parse(setPrices([(1, 100), (10, 80), (100, 50)]))
        XCTAssertEqual(
            pricing,
            .setPrices([
                .init(amount: 1, price: 100),
                .init(amount: 10, price: 80),
                .init(amount: 100, price: 50),
            ])
        )
    }

    func test_nullEntry_meansNoPrice() {
        XCTAssertNil(parse(#"{"\#(tokenId)":null}"#))
    }

    func test_missingTokenKey_meansNoPrice() {
        XCTAssertNil(parse(#"{"other":{"type":"single_price","price":5}}"#))
    }

    func test_emptySetPricesSchedule_meansNoPrice() {
        XCTAssertNil(parse(setPrices([])))
    }

    func test_garbageJson_meansNoPrice() {
        XCTAssertNil(parse("not json"))
        XCTAssertNil(parse("[1,2,3]"))
    }

    func test_u64PriceBeyondInt64Range_parses() {
        // 2^63 + 1 — outside signed Int64, must survive parsing via uint64Value.
        let big: UInt64 = (1 << 63) + 1
        XCTAssertEqual(parse(single("\(big)")), .singlePrice(big))
    }

    /// DPP writes a `u64` above the JavaScript safe-integer ceiling
    /// (2^53 - 1) as a decimal STRING, not a number. Reading only numbers
    /// returned nil here, which the form reports as "not for direct sale":
    /// an expensive token looked unpurchasable.
    func test_singlePriceEncodedAsString_parses() {
        let big: UInt64 = 9_007_199_254_740_992  // 2^53, one past the ceiling
        XCTAssertEqual(parse(single("\"\(big)\"")), .singlePrice(big))
        XCTAssertEqual(parse(single("\"100\"")), .singlePrice(100))
        XCTAssertEqual(parse(single("\"\(UInt64.max)\"")), .singlePrice(UInt64.max))
    }

    /// The same for a tier schedule, where either half of a tier can cross
    /// the ceiling on its own.
    func test_setPricesTiersEncodedAsStrings_parse() {
        let bigAmount: UInt64 = 9_007_199_254_740_992
        let bigPrice: UInt64 = 18_014_398_509_481_984  // 2^54
        let json = #"{"\#(tokenId)":{"type":"set_prices","prices":"#
            + #"[{"amount":1,"price":"\#(bigPrice)"},"#
            + #"{"amount":"\#(bigAmount)","price":50}]}}"#
        XCTAssertEqual(
            parse(json),
            .setPrices([
                .init(amount: 1, price: bigPrice),
                .init(amount: bigAmount, price: 50),
            ])
        )
    }

    /// A string that is not a plain decimal is still no price, rather than
    /// being coerced into one.
    func test_unparseableStringPrice_meansNoPrice() {
        XCTAssertNil(parse(single("\"\"")))
        XCTAssertNil(parse(single("\"-1\"")))
        XCTAssertNil(parse(single("\"1.5\"")))
        XCTAssertNil(parse(single("\"free\"")))
        XCTAssertNil(parse(single("true")))
    }

    // MARK: - Cost resolution

    func test_singlePrice_costIsPriceTimesAmount() {
        let pricing = parse(single("250"))!
        XCTAssertEqual(pricing.cost(forAmount: 4), 1_000)
    }

    func test_setPrices_picksHighestTierAtOrBelowAmount() {
        let pricing = parse(setPrices([(1, 100), (10, 80), (100, 50)]))!
        // Buying 50 matches tier 10 (price 80) => 80 * 50.
        XCTAssertEqual(pricing.cost(forAmount: 50), 4_000)
        // Exact boundary hits its own tier.
        XCTAssertEqual(pricing.cost(forAmount: 10), 800)
        // Above the top tier uses the top tier.
        XCTAssertEqual(pricing.cost(forAmount: 1_000), 50_000)
    }

    func test_amountBelowMinimumTier_isNotPurchasable() {
        let pricing = parse(setPrices([(5, 200), (10, 150)]))!
        XCTAssertNil(pricing.cost(forAmount: 2))
        XCTAssertEqual(pricing.minimumPurchaseAmount, 5)
    }

    func test_freeTier_producesZeroExpectedCost() {
        XCTAssertEqual(parse(single("0"))!.cost(forAmount: 10), 0)
    }

    func test_zeroAmount_isNotPurchasable() {
        let pricing = parse(single("100"))!
        XCTAssertNil(pricing.cost(forAmount: 0))
    }

    func test_singlePriceOverflow_saturatesLikeDrive() {
        // price 2^62, amount 4 => 2^64, overflows UInt64.
        let pricing = parse(single("\(UInt64(1) << 62)"))!
        XCTAssertEqual(pricing.cost(forAmount: 4), UInt64.max)
    }

    func test_setPricesOverflow_isNotPurchasableLikeDrive() {
        let pricing = parse(setPrices([(1, UInt64(1) << 62)]))!
        XCTAssertNil(pricing.cost(forAmount: 4))
    }

    func test_amountAboveProtocolU48Maximum_isNotPurchasable() {
        let pricing = parse(single("1"))!
        XCTAssertEqual(pricing.cost(forAmount: 281_474_976_710_655), 281_474_976_710_655)
        XCTAssertNil(pricing.cost(forAmount: 281_474_976_710_656))
    }

    func test_costMayUseHighHalfOfUInt64() {
        let pricing = parse(single("\(UInt64(1) << 62)"))!
        XCTAssertEqual(pricing.cost(forAmount: 2), UInt64(1) << 63)
    }

    func test_singlePrice_minimumIsOne() {
        let pricing = parse(single("100"))!
        XCTAssertEqual(pricing.minimumPurchaseAmount, 1)
    }
}
