import XCTest
@testable import SwiftDashSDK
@testable import SwiftExampleApp

/// Pins the catalog's removal records preview: it stops at its budget, says
/// how many records it shows, and drops the full page's cursor when it cuts,
/// as paging from it would skip the records left out.
final class RemovalsPreviewFormatterTests: XCTestCase {
    private func page(records: Int, cursor: String?) throws -> ContractDocumentRemovalsPage {
        let hash = String(repeating: "ab", count: 32)
        let removals = (0..<records).map { index in
            #"{"documentId":"doc\#(index)","documentOwnerId":"owner","moderatorId":"moderator","reason":{"code":null,"text":"off topic"},"removedAt":1,"documentHash":"\#(hash)","keptFields":{}}"#
        }
        let next = cursor.map { #","nextStartAfter":"\#($0)""# } ?? ""
        let json = #"{"removals":[\#(removals.joined(separator: ","))]\#(next)}"#
        return try JSONDecoder().decode(ContractDocumentRemovalsPage.self, from: Data(json.utf8))
    }

    private func parse(_ text: String) throws -> [Swift.String: Any] {
        try XCTUnwrap(JSONSerialization.jsonObject(with: Data(text.utf8)) as? [Swift.String: Any])
    }

    func testShouldShowAWholePageWithItsCursor() throws {
        let text = try RemovalsPreviewFormatter.format(
            inputs: ["contractId": "c"], page: page(records: 3, cursor: "doc2"))
        let result = try XCTUnwrap(try parse(text)["result"] as? [Swift.String: Any])

        XCTAssertEqual((result["removals"] as? [Any])?.count, 3)
        XCTAssertEqual(result["nextStartAfter"] as? Swift.String, "doc2")
        XCTAssertNil(result["preview"])
    }

    func testShouldStopAtTheBudgetAndDropTheFullPagesCursor() throws {
        let text = try RemovalsPreviewFormatter.format(
            inputs: ["contractId": "c"],
            page: page(records: 50, cursor: "doc49"),
            maxBytes: 2_000
        )
        XCTAssertLessThanOrEqual(text.utf8.count, 2_000)
        let parsed = try parse(text)
        XCTAssertEqual((parsed["inputs"] as? [Swift.String: Swift.String])?["contractId"], "c")
        let result = try XCTUnwrap(parsed["result"] as? [Swift.String: Any])
        let shown = try XCTUnwrap(result["removals"] as? [Any]).count

        XCTAssertLessThan(shown, 50)
        XCTAssertNil(result["nextStartAfter"])
        let note = try XCTUnwrap(result["preview"] as? Swift.String)
        XCTAssertTrue(note.contains("\(shown) of 50 records shown"))
        XCTAssertTrue(note.contains("rerun from the same Start After"))
    }
}
