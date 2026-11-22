import Foundation
import XCTest
@testable import SwiftDashSDK

/// Hermetic tests for the JSON `platform_wallet_preview_data_contract_update`
/// writes, decoded by `DataContractUpdatePreview.decode(fromJSON:)` exactly as
/// `ManagedPlatformWallet.previewDataContractUpdate` decodes it.
///
/// The payloads mirror the shape the FFI's `preview_to_json` in
/// `packages/rs-platform-wallet-ffi/src/data_contract.rs` writes (and its
/// tests pin). No wallet handle, no network.
final class DataContractUpdatePreviewDecodingTests: XCTestCase {
    func testShouldDecodeAPreviewWithEveryField() throws {
        let json = """
            {"currentVersion":1,"newVersion":2,"hasChanges":true,\
            "documentTypes":{"added":["comment"],"changed":["note"]},\
            "tokens":{"added":["2","10"],"changed":[]},\
            "groups":{"added":[],"changed":["0"]},\
            "keywordsChanged":true,"descriptionChanged":false,"configChanged":true}
            """

        let preview = try DataContractUpdatePreview.decode(fromJSON: json)

        XCTAssertEqual(
            preview,
            DataContractUpdatePreview(
                currentVersion: 1,
                newVersion: 2,
                hasChanges: true,
                documentTypes: .init(added: ["comment"], changed: ["note"]),
                tokens: .init(added: ["2", "10"], changed: []),
                groups: .init(added: [], changed: ["0"]),
                keywordsChanged: true,
                descriptionChanged: false,
                configChanged: true
            )
        )
    }

    /// `hasChanges` is read as Rust wrote it, never recomputed from the
    /// sections.
    func testShouldTakeHasChangesAsWritten() throws {
        let json = """
            {"currentVersion":4,"newVersion":5,"hasChanges":false,\
            "documentTypes":{"added":[],"changed":[]},\
            "tokens":{"added":[],"changed":[]},\
            "groups":{"added":[],"changed":[]},\
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":false}
            """

        let preview = try DataContractUpdatePreview.decode(fromJSON: json)

        XCTAssertFalse(preview.hasChanges)
        XCTAssertEqual(preview.currentVersion, 4)
        XCTAssertEqual(preview.newVersion, 5)
    }

    func testShouldRejectMalformedJSON() {
        let malformed = [
            "",
            "not json",
            // A section is missing.
            """
            {"currentVersion":1,"newVersion":2,"hasChanges":true,\
            "documentTypes":{"added":[],"changed":[]},\
            "tokens":{"added":[],"changed":[]},\
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":false}
            """,
            // A version is not a number.
            """
            {"currentVersion":"1","newVersion":2,"hasChanges":true,\
            "documentTypes":{"added":[],"changed":[]},\
            "tokens":{"added":[],"changed":[]},\
            "groups":{"added":[],"changed":[]},\
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":false}
            """,
        ]

        for json in malformed {
            XCTAssertThrowsError(try DataContractUpdatePreview.decode(fromJSON: json), json) { error in
                guard case PlatformWalletError.deserialization = error else {
                    return XCTFail("expected a deserialization error, got \(error)")
                }
            }
        }
    }
}
