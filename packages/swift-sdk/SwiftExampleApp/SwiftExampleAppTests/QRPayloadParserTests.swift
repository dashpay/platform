import XCTest
import SwiftDashSDK
@testable import SwiftExampleApp

/// Behavioral tests for `QRPayloadParser.parse` — the pure decoder behind
/// the Send screen's QR scanner.
///
/// The scanner UI (camera session, overlays, haptics) isn't unit-testable,
/// but the parser is: it's the piece that decides what a scanned/pasted
/// string actually means. It accepts bare addresses and BIP21-style
/// `dash:` URIs, lifts an optional `amount`, and rejects anything that
/// doesn't resolve to a known `DashAddress` on the given network.
///
/// Address validation is NOT mocked: the Core path runs through
/// `DashAddress.parse` → `Address.validate` (Rust FFI), which links fine in
/// the test bundle (other suites already exercise the SDK). All cases use
/// `Network.testnet`.
final class QRPayloadParserTests: XCTestCase {

    /// Known-valid testnet P2PKH (base58check) Core address.
    private let validCoreAddress = "yMqShkrgjTRuReBGFpQr7FozEF1QcNBBYA"
    private let network: Network = .testnet

    // MARK: - Sanity: the fixture really is valid on testnet

    func test_fixtureAddress_isRecognizedByDashAddress() {
        // If this fails, every other case below is meaningless — the
        // fixture or the FFI link is the problem, not the parser.
        XCTAssertNotEqual(
            DashAddress.parse(validCoreAddress, network: network).type,
            .unknown,
            "Fixture testnet address should validate via the FFI"
        )
    }

    // MARK: - Bare address

    func test_bareCoreAddress_isAccepted() {
        let result = QRPayloadParser.parse(validCoreAddress, network: network)
        XCTAssertEqual(result, ScannedPayment(address: validCoreAddress, amount: nil))
    }

    // MARK: - Scheme stripping

    func test_dashSchemePrefix_isStripped() {
        let result = QRPayloadParser.parse("dash:\(validCoreAddress)", network: network)
        XCTAssertEqual(result?.address, validCoreAddress)
        XCTAssertNil(result?.amount)
    }

    func test_dashSchemeIsCaseInsensitive() {
        let result = QRPayloadParser.parse("DASH:\(validCoreAddress)", network: network)
        XCTAssertEqual(result?.address, validCoreAddress)
    }

    func test_dashSchemeWithDoubleSlash_isStripped() {
        let result = QRPayloadParser.parse("dash://\(validCoreAddress)", network: network)
        XCTAssertEqual(result?.address, validCoreAddress)
    }

    // MARK: - Amount parsing

    func test_amountQueryParam_isExtracted() {
        let result = QRPayloadParser.parse(
            "dash:\(validCoreAddress)?amount=0.5&label=x",
            network: network
        )
        XCTAssertEqual(result?.address, validCoreAddress)
        XCTAssertEqual(result?.amount, "0.5")
    }

    func test_amountBeforeLabelOrder_isIrrelevant() {
        let result = QRPayloadParser.parse(
            "dash:\(validCoreAddress)?label=Coffee&amount=1.23",
            network: network
        )
        XCTAssertEqual(result?.amount, "1.23")
    }

    func test_nonNumericAmount_isDroppedButAddressKept() {
        let result = QRPayloadParser.parse(
            "dash:\(validCoreAddress)?amount=abc",
            network: network
        )
        XCTAssertEqual(result?.address, validCoreAddress)
        XCTAssertNil(result?.amount)
    }

    func test_negativeAmount_isDroppedButAddressKept() {
        let result = QRPayloadParser.parse(
            "dash:\(validCoreAddress)?amount=-1.0",
            network: network
        )
        XCTAssertEqual(result?.address, validCoreAddress)
        XCTAssertNil(result?.amount)
    }

    func test_zeroAmount_isDropped() {
        // Zero is not a positive amount.
        let result = QRPayloadParser.parse(
            "dash:\(validCoreAddress)?amount=0",
            network: network
        )
        XCTAssertEqual(result?.address, validCoreAddress)
        XCTAssertNil(result?.amount)
    }

    // MARK: - Whitespace handling

    func test_whitespaceWrappedAddress_isTrimmedAndValid() {
        let result = QRPayloadParser.parse("  \n\(validCoreAddress)\t \n", network: network)
        XCTAssertEqual(result?.address, validCoreAddress)
    }

    // MARK: - Rejections

    func test_garbageString_isRejected() {
        XCTAssertNil(QRPayloadParser.parse("not-an-address-at-all", network: network))
    }

    func test_emptyString_isRejected() {
        XCTAssertNil(QRPayloadParser.parse("", network: network))
    }

    func test_whitespaceOnly_isRejected() {
        XCTAssertNil(QRPayloadParser.parse("   \n\t  ", network: network))
    }

    func test_schemeWithNoAddress_isRejected() {
        XCTAssertNil(QRPayloadParser.parse("dash:", network: network))
    }

    func test_addressCasingIsPreserved_notLowercased() {
        // The parser must not lowercase the candidate — base58check is
        // case-sensitive, so a mangled-case address must be rejected
        // rather than "fixed" into a different (or invalid) address.
        let mangled = validCoreAddress.lowercased()
        XCTAssertNotEqual(mangled, validCoreAddress)
        XCTAssertNil(QRPayloadParser.parse(mangled, network: network))
    }
}
