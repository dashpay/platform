import XCTest
@testable import SwiftExampleApp

// The rules that split pasted or imported JSON into the six fields
// (`UpdateContractImport.read(from:)`) are Rust's and are tested in Rust:
// `split_contract_update_source` in
// `packages/rs-platform-wallet/src/wallet/identity/network/contract_update_source.rs`.
// `read` calls the FFI, and these app tests do not run against the real
// framework, so the cases here cover only what Swift owns: the wallet
// arguments, field equality and the input budget.

/// `UpdateContractImport.WalletArguments` hands the Update Contract screen's
/// fields to the wallet; `UpdateContractImport.Fields` equality decides when
/// an edit clears a review; `checkSize` is the screen's input budget.
final class UpdateContractImportTests: XCTestCase {

    // MARK: - Wallet arguments

    func testShouldPassAKeywordsJSONArrayThroughVerbatim() throws {
        let typed = #"  ["alpha,beta", "gamma"]"#
        XCTAssertEqual(try UpdateContractImport.WalletArguments.keywordsJSON(from: typed), typed)
    }

    func testShouldStillReadCommaSeparatedKeywords() throws {
        XCTAssertEqual(
            try UpdateContractImport.WalletArguments.keywordsJSON(from: " alpha, beta ,,gamma "),
            #"["alpha","beta","gamma"]"#
        )
        XCTAssertNil(try UpdateContractImport.WalletArguments.keywordsJSON(from: "  "))
    }

    func testShouldKeepTheLiveDescriptionOnlyForAnEmptyField() throws {
        var fields = UpdateContractImport.Fields.empty
        XCTAssertNil(try UpdateContractImport.WalletArguments(fields).description)
        fields.description = " "
        XCTAssertEqual(try UpdateContractImport.WalletArguments(fields).description, " ")
    }

    func testShouldPassANonEmptyConfigAndKeepTheLiveOneForAnEmptyField() throws {
        var fields = UpdateContractImport.Fields.empty
        XCTAssertNil(try UpdateContractImport.WalletArguments(fields).config)
        fields.config = " \n "
        XCTAssertNil(try UpdateContractImport.WalletArguments(fields).config)
        fields.config = #"  {"readonly":true} "#
        XCTAssertEqual(try UpdateContractImport.WalletArguments(fields).config, #"{"readonly":true}"#)
    }

    // MARK: - Field equality

    /// `Fields` equality is by bytes: the wallet receives the bytes, so two
    /// spellings Swift `String` calls equal must not share a review.
    func testShouldNotEqualFieldsThatDifferOnlyByNormalizationForm() {
        let precomposed = "caf\u{00E9}"
        let decomposed = "cafe\u{0301}"
        // The case this guards: Swift `String` equality calls them equal.
        XCTAssertEqual(precomposed, decomposed)

        var lhs = UpdateContractImport.Fields.empty
        var rhs = UpdateContractImport.Fields.empty
        lhs.description = precomposed
        rhs.description = decomposed
        XCTAssertNotEqual(lhs, rhs)

        lhs = .empty
        rhs = .empty
        lhs.keywords = "\(precomposed), tea"
        rhs.keywords = "\(decomposed), tea"
        XCTAssertNotEqual(lhs, rhs)
    }

    func testShouldEqualFieldsWithIdenticalBytes() {
        var lhs = UpdateContractImport.Fields.empty
        var rhs = UpdateContractImport.Fields.empty
        XCTAssertEqual(lhs, rhs)

        lhs.description = "caf\u{00E9}"
        rhs.description = "caf\u{00E9}"
        lhs.keywords = "cafe\u{0301}, tea"
        rhs.keywords = "cafe\u{0301}, tea"
        lhs.config = #"{"readonly":true}"#
        rhs.config = #"{"readonly":true}"#
        XCTAssertEqual(lhs, rhs)

        rhs.config = #"{"readonly":false}"#
        XCTAssertNotEqual(lhs, rhs)
    }

    // MARK: - Input budget

    func testShouldAcceptAnInputUpToTheBudget() {
        XCTAssertNoThrow(try UpdateContractImport.checkSize(byteCount: 0))
        XCTAssertNoThrow(try UpdateContractImport.checkSize(byteCount: UpdateContractImport.maxInputBytes))
    }

    func testShouldRefuseAnInputOverTheBudgetNamingTheLimit() {
        XCTAssertThrowsError(
            try UpdateContractImport.checkSize(byteCount: UpdateContractImport.maxInputBytes + 1)
        ) { error in
            XCTAssertEqual(error as? UpdateContractImport.ImportError, .tooLarge)
            XCTAssertTrue(error.localizedDescription.contains("limit of 4 MiB."), error.localizedDescription)
        }
    }
}
