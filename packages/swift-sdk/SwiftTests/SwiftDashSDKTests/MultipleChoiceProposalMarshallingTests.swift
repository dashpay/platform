import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the C structs the multiple choice proposal wallet
/// wrappers hand to `platform_wallet_multiple_choice_proposal_*`
/// (`MultipleChoiceProposalParamsFFI`, `MultipleChoiceProposalEditFFI`,
/// `ChoiceParamsFFI`, `DescriptionDocumentFFI`). Each string goes through
/// `withDecisionProposalCString`, so one holding U+0000 is refused rather
/// than handed over cut short; each id must be the 32 bytes the Rust side
/// reads. No FFI call is made.
final class MultipleChoiceProposalMarshallingTests: XCTestCase {
    private let documentId = Data(repeating: 5, count: 32)
    private let contractId = Data(repeating: 6, count: 32)

    private var descriptionDocument: DescriptionDocumentParams {
        DescriptionDocumentParams(id: documentId, contractId: contractId, documentType: "post")
    }

    /// The string `pointer` holds up to its NUL terminator; `nil` for null.
    private func string(_ pointer: UnsafePointer<CChar>?) -> String? {
        pointer.map { String(cString: $0) }
    }

    /// The 32 bytes `pointer` holds; `nil` for null.
    private func bytes32(_ pointer: UnsafePointer<UInt8>?) -> Data? {
        pointer.map { Data(bytes: $0, count: 32) }
    }

    /// The description document `pointer` holds, read as the Rust side reads
    /// it; `nil` for null.
    private func readDescriptionDocument(
        _ pointer: UnsafePointer<DescriptionDocumentFFI>?
    ) -> DescriptionDocumentParams? {
        guard let document = pointer?.pointee else { return nil }
        return DescriptionDocumentParams(
            id: bytes32(document.id) ?? Data(),
            contractId: bytes32(document.contract_id) ?? Data(),
            documentType: string(document.document_type) ?? ""
        )
    }

    /// `body` refuses with `PlatformWalletError.invalidParameter` naming
    /// `field`, without running the marshalled body.
    private func assertRefused(
        field: String,
        file: StaticString = #filePath,
        line: UInt = #line,
        _ body: (_ bodyRan: inout Bool) throws -> Void
    ) {
        var bodyRan = false
        XCTAssertThrowsError(try body(&bodyRan), file: file, line: line) { error in
            guard case PlatformWalletError.invalidParameter(let message) = error else {
                return XCTFail("expected PlatformWalletError.invalidParameter, got \(error)", file: file, line: line)
            }
            XCTAssertTrue(message.hasPrefix("\(field) "), message, file: file, line: line)
        }
        XCTAssertFalse(bodyRan, "a refused value must not reach C", file: file, line: line)
    }

    // MARK: - Create params

    func testShouldHandOverEveryCreateParam() throws {
        let params = MultipleChoiceProposalParams(
            title: "Which block size? 是 ✓",
            descriptionDocument: descriptionDocument,
            url: "https://forum.dash.org/t/block-size",
            open: true
        )
        try params.withFFI { ffi in
            XCTAssertEqual(string(ffi.pointee.title), "Which block size? 是 ✓")
            XCTAssertEqual(readDescriptionDocument(ffi.pointee.description_document), descriptionDocument)
            XCTAssertEqual(string(ffi.pointee.url), "https://forum.dash.org/t/block-size")
            XCTAssertTrue(ffi.pointee.open)
        }
    }

    func testShouldHandOverAbsentCreateParamsAsNullPointers() throws {
        let params = MultipleChoiceProposalParams(title: "t", open: false)
        try params.withFFI { ffi in
            XCTAssertEqual(string(ffi.pointee.title), "t")
            XCTAssertNil(ffi.pointee.description_document)
            XCTAssertNil(ffi.pointee.url)
            XCTAssertFalse(ffi.pointee.open)
        }
    }

    func testShouldRefuseACreateParamHoldingANUL() {
        assertRefused(field: "title") { bodyRan in
            try MultipleChoiceProposalParams(title: "Which\0", open: true).withFFI { _ in bodyRan = true }
        }
        assertRefused(field: "url") { bodyRan in
            try MultipleChoiceProposalParams(title: "t", url: "https://\0", open: true)
                .withFFI { _ in bodyRan = true }
        }
        var document = descriptionDocument
        document.documentType = "po\0st"
        assertRefused(field: "descriptionDocument.documentType") { bodyRan in
            try MultipleChoiceProposalParams(title: "t", descriptionDocument: document, open: true)
                .withFFI { _ in bodyRan = true }
        }
    }

    func testShouldRefuseADescriptionDocumentIdThatIsNot32Bytes() {
        var shortId = descriptionDocument
        shortId.id = Data(repeating: 5, count: 31)
        assertRefused(field: "descriptionDocument.id") { bodyRan in
            try MultipleChoiceProposalParams(title: "t", descriptionDocument: shortId, open: true)
                .withFFI { _ in bodyRan = true }
        }
        var longContractId = descriptionDocument
        longContractId.contractId = Data(repeating: 6, count: 33)
        assertRefused(field: "descriptionDocument.contractId") { bodyRan in
            try MultipleChoiceProposalParams(title: "t", descriptionDocument: longContractId, open: true)
                .withFFI { _ in bodyRan = true }
        }
    }

    // MARK: - Edit

    func testShouldHandOverAnEditThatKeepsEverythingAsNullsWithoutFlags() throws {
        try MultipleChoiceProposalEdit().withFFI { ffi in
            XCTAssertNil(ffi.pointee.title)
            XCTAssertNil(ffi.pointee.description_document)
            XCTAssertFalse(ffi.pointee.remove_description_document)
            XCTAssertNil(ffi.pointee.url)
            XCTAssertFalse(ffi.pointee.remove_url)
        }
    }

    func testShouldHandOverAnEditThatSetsEveryField() throws {
        let edit = MultipleChoiceProposalEdit(
            title: "New title",
            descriptionDocument: .set(descriptionDocument),
            url: .set("https://dash.org")
        )
        try edit.withFFI { ffi in
            XCTAssertEqual(string(ffi.pointee.title), "New title")
            XCTAssertEqual(readDescriptionDocument(ffi.pointee.description_document), descriptionDocument)
            XCTAssertFalse(ffi.pointee.remove_description_document)
            XCTAssertEqual(string(ffi.pointee.url), "https://dash.org")
            XCTAssertFalse(ffi.pointee.remove_url)
        }
    }

    /// A removal is a null value beside a raised flag; the FFI refuses a
    /// value together with its flag.
    func testShouldHandOverARemovalAsANullBesideItsFlag() throws {
        try MultipleChoiceProposalEdit(descriptionDocument: .remove).withFFI { ffi in
            XCTAssertNil(ffi.pointee.description_document)
            XCTAssertTrue(ffi.pointee.remove_description_document)
            XCTAssertNil(ffi.pointee.url)
            XCTAssertFalse(ffi.pointee.remove_url)
        }
        try MultipleChoiceProposalEdit(url: .remove).withFFI { ffi in
            XCTAssertNil(ffi.pointee.description_document)
            XCTAssertFalse(ffi.pointee.remove_description_document)
            XCTAssertNil(ffi.pointee.url)
            XCTAssertTrue(ffi.pointee.remove_url)
        }
    }

    func testShouldRefuseAnEditHoldingANULOrAShortId() {
        assertRefused(field: "title") { bodyRan in
            try MultipleChoiceProposalEdit(title: "\0").withFFI { _ in bodyRan = true }
        }
        assertRefused(field: "url") { bodyRan in
            try MultipleChoiceProposalEdit(url: .set("a\0b")).withFFI { _ in bodyRan = true }
        }
        var shortId = descriptionDocument
        shortId.id = Data()
        assertRefused(field: "descriptionDocument.id") { bodyRan in
            try MultipleChoiceProposalEdit(descriptionDocument: .set(shortId)).withFFI { _ in bodyRan = true }
        }
    }

    // MARK: - Choice

    func testShouldHandOverAChoice() throws {
        try ProposalChoiceParams(title: "Double it", description: "Twice the size.").withFFI { ffi in
            XCTAssertEqual(string(ffi.pointee.title), "Double it")
            XCTAssertEqual(string(ffi.pointee.description), "Twice the size.")
        }
        try ProposalChoiceParams(title: "Keep it").withFFI { ffi in
            XCTAssertEqual(string(ffi.pointee.title), "Keep it")
            XCTAssertNil(ffi.pointee.description)
        }
    }

    func testShouldRefuseAChoiceHoldingANUL() {
        assertRefused(field: "title") { bodyRan in
            try ProposalChoiceParams(title: "Double\0it").withFFI { _ in bodyRan = true }
        }
        assertRefused(field: "description") { bodyRan in
            try ProposalChoiceParams(title: "t", description: "\0").withFFI { _ in bodyRan = true }
        }
    }

    func testShouldPassTheBodysErrorThroughUnchanged() {
        struct BodyError: Error, Equatable {}
        XCTAssertThrowsError(
            try ProposalChoiceParams(title: "t").withFFI { _ in throw BodyError() }
        ) { error in
            XCTAssertEqual(error as? BodyError, BodyError())
        }
    }
}
