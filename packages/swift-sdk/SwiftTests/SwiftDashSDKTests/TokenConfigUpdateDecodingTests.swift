import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for what `ManagedPlatformWallet.tokenUpdateConfig` sends to
/// and reads from the `rs-platform-wallet-ffi`
/// `platform_wallet_token_update_config_and_refresh` function: the change
/// item tag and payload, and the outcome JSON, decoded exactly as the wrapper
/// decodes it.
///
/// The payloads are the ones the FFI's
/// `packages/rs-platform-wallet-ffi/src/tokens/update_config.rs` tests pin.
/// No wallet handle, no network.
final class TokenConfigUpdateDecodingTests: XCTestCase {
    /// The tags must agree with `TAG_MAX_SUPPLY` and
    /// `TAG_MARKETPLACE_TRADE_MODE` in the FFI; any other tag is refused
    /// there.
    func testShouldEncodeTheChangeItemsAsTheFFITags() {
        XCTAssertEqual(TokenConfigChange.maxSupply(newValue: 1).ffiTag, 0)
        XCTAssertEqual(TokenConfigChange.marketplaceTradeMode(.tradeable).ffiTag, 1)
    }

    func testShouldRenderTheMaxSupplyPayload() {
        XCTAssertEqual(
            TokenConfigChange.maxSupply(newValue: 1_000_000).payloadJSON(),
            #"{"newMaxSupply":"1000000"}"#
        )
        XCTAssertEqual(
            TokenConfigChange.maxSupply(newValue: .max).payloadJSON(),
            #"{"newMaxSupply":"18446744073709551615"}"#
        )
        XCTAssertEqual(
            TokenConfigChange.maxSupply(newValue: nil).payloadJSON(),
            #"{"newMaxSupply":null}"#
        )
    }

    /// The names are the ones rs-dpp gives the modes in a contract's JSON,
    /// which the FFI decodes with rs-dpp's own type.
    func testShouldRenderTheMarketplaceTradeModePayload() {
        XCTAssertEqual(
            TokenConfigChange.marketplaceTradeMode(.tradeable).payloadJSON(),
            #"{"tradeMode":"Tradeable"}"#
        )
        XCTAssertEqual(
            TokenConfigChange.marketplaceTradeMode(.notTradeable).payloadJSON(),
            #"{"tradeMode":"NotTradeable"}"#
        )
    }

    func testShouldDecodeAPendingGroupAction() throws {
        XCTAssertEqual(
            try TokenConfigUpdateOutcome.decode(fromJSON: #"{"outcome":"pending"}"#),
            .pending
        )
    }

    func testShouldDecodeAnAppliedUpdateWithTheContractItProduced() throws {
        let outcome = try TokenConfigUpdateOutcome.decode(
            fromJSON: #"{"outcome":"applied","contract":{"id":"abc","version":4},"serializationHex":"ab01"}"#
        )
        guard case .applied(let contractJSON, let serialization) = outcome else {
            return XCTFail("expected an applied outcome, got \(outcome)")
        }
        let contract = try XCTUnwrap(
            JSONSerialization.jsonObject(with: contractJSON) as? [String: Any]
        )
        XCTAssertEqual(contract["id"] as? String, "abc")
        XCTAssertEqual(contract["version"] as? Int, 4)
        XCTAssertEqual(serialization, Data([0xab, 0x01]))
    }

    func testShouldDecodeAnAppliedUpdateThatWasNotRefreshed() throws {
        XCTAssertEqual(
            try TokenConfigUpdateOutcome.decode(
                fromJSON: #"{"outcome":"appliedNotRefreshed","minimumVersion":3,"reason":"behind"}"#
            ),
            .appliedNotRefreshed(minimumVersion: 3, reason: "behind")
        )
    }

    func testShouldRefuseAnOutcomeItDoesNotKnow() {
        for json in [
            #"{"outcome":"reverted"}"#,
            #"{"outcome":"applied"}"#,
            #"{"outcome":"appliedNotRefreshed","reason":"behind"}"#,
            #"{}"#,
            "not json",
        ] {
            XCTAssertThrowsError(try TokenConfigUpdateOutcome.decode(fromJSON: json), json) { error in
                guard case PlatformWalletError.deserialization = error else {
                    return XCTFail("expected a deserialization error for \(json), got \(error)")
                }
            }
        }
    }
}
