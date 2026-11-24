import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the rules of a token pool spend form's fee state: Send and Unshield
/// are allowed only once Rust has quoted exactly the typed amount, and a
/// re-ask for the same amount keeps the answer already shown.
final class ShieldedTokenSpendFeeStateTests: XCTestCase {

    private let preview = ShieldedTokenSpendPreview(tokenNotes: 1, feeNotes: 2, fee: 1_000)

    // MARK: - allowsSubmit

    func testShouldAllowSubmitForAQuoteOfTheSameAmount() {
        let state = ShieldedTokenSpendFeeState.quoted(amount: 5, preview: preview)
        XCTAssertTrue(state.allowsSubmit(amount: 5))
    }

    func testShouldNotAllowSubmitForAQuoteOfAnotherAmount() {
        let state = ShieldedTokenSpendFeeState.quoted(amount: 5, preview: preview)
        XCTAssertFalse(state.allowsSubmit(amount: 6))
        XCTAssertFalse(state.allowsSubmit(amount: nil))
    }

    func testShouldNotAllowSubmitWhileLoading() {
        XCTAssertFalse(ShieldedTokenSpendFeeState.loading.allowsSubmit(amount: 5))
    }

    func testShouldNotAllowSubmitWhileIdle() {
        XCTAssertFalse(ShieldedTokenSpendFeeState.idle.allowsSubmit(amount: 5))
    }

    func testShouldNotAllowSubmitAfterARefusalOfTheSameAmount() {
        let state = ShieldedTokenSpendFeeState.refused(amount: 5, message: "refused")
        XCTAssertFalse(state.allowsSubmit(amount: 5))
    }

    // MARK: - startingQuote

    func testShouldKeepAHeldQuoteWhenAQuoteForTheSameAmountStarts() {
        let state = ShieldedTokenSpendFeeState.quoted(amount: 5, preview: preview)
        XCTAssertEqual(state.startingQuote(for: 5), state)
    }

    func testShouldKeepAHeldRefusalWhenAQuoteForTheSameAmountStarts() {
        let state = ShieldedTokenSpendFeeState.refused(amount: 5, message: "refused")
        XCTAssertEqual(state.startingQuote(for: 5), state)
    }

    func testShouldShowLoadingWhenAQuoteForAnotherAmountStarts() {
        XCTAssertEqual(
            ShieldedTokenSpendFeeState.quoted(amount: 5, preview: preview).startingQuote(for: 6),
            .loading
        )
        XCTAssertEqual(
            ShieldedTokenSpendFeeState.refused(amount: 5, message: "refused").startingQuote(for: 6),
            .loading
        )
    }

    func testShouldShowLoadingWhenAQuoteStartsFromIdleOrLoading() {
        XCTAssertEqual(ShieldedTokenSpendFeeState.idle.startingQuote(for: 5), .loading)
        XCTAssertEqual(ShieldedTokenSpendFeeState.loading.startingQuote(for: 5), .loading)
    }

    func testShouldShowIdleWhenAQuoteStartsWithoutAnAmount() {
        XCTAssertEqual(
            ShieldedTokenSpendFeeState.quoted(amount: 5, preview: preview).startingQuote(for: nil),
            .idle
        )
        XCTAssertEqual(ShieldedTokenSpendFeeState.loading.startingQuote(for: nil), .idle)
    }
}
