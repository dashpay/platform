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
            "keywordsChanged":true,"descriptionChanged":false,"configChanged":true,\
            "configChangedFields":["moderation","readonly"],\
            "moderatorsAdded":["4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"],\
            "moderatorsRemoved":["GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"]}
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
                configChanged: true,
                configChangedFields: ["moderation", "readonly"],
                moderatorsAdded: ["4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF"],
                moderatorsRemoved: ["GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec"]
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
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":false,\
            "configChangedFields":[],"moderatorsAdded":[],"moderatorsRemoved":[]}
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
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":false,\
            "configChangedFields":[],"moderatorsAdded":[],"moderatorsRemoved":[]}
            """,
            // A version is not a number.
            """
            {"currentVersion":"1","newVersion":2,"hasChanges":true,\
            "documentTypes":{"added":[],"changed":[]},\
            "tokens":{"added":[],"changed":[]},\
            "groups":{"added":[],"changed":[]},\
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":false,\
            "configChangedFields":[],"moderatorsAdded":[],"moderatorsRemoved":[]}
            """,
            // A moderator list is missing: it is not read as empty.
            """
            {"currentVersion":1,"newVersion":2,"hasChanges":true,\
            "documentTypes":{"added":[],"changed":[]},\
            "tokens":{"added":[],"changed":[]},\
            "groups":{"added":[],"changed":[]},\
            "keywordsChanged":false,"descriptionChanged":false,"configChanged":true,\
            "configChangedFields":["moderation"],"moderatorsRemoved":[]}
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

    // MARK: - Arguments that cannot cross the FFI

    /// A string with a NUL would reach Rust cut at the NUL, so the update
    /// and its preview refuse it, naming the argument, before any FFI call.
    func testShouldRefuseAnUpdateArgumentThatHoldsANUL() {
        func check(
            documents: String = "{}",
            tokens: String? = nil,
            groups: String? = nil,
            keywords: String? = nil,
            description: String? = nil,
            config: String? = nil
        ) throws {
            try ManagedPlatformWallet.requireNoNULInDataContractUpdateArguments(
                documentSchemasJSON: documents,
                tokenSchemasJSON: tokens,
                groupsJSON: groups,
                keywordsJSON: keywords,
                description: description,
                contractConfigJSON: config
            )
        }

        let cases: [(name: String, run: () throws -> Void)] = [
            ("documentSchemasJSON", { try check(documents: "{\"note\":{}}\u{0}{}") }),
            ("tokenSchemasJSON", { try check(tokens: "\u{0}") }),
            ("groupsJSON", { try check(groups: "{}\u{0}") }),
            ("keywordsJSON", { try check(keywords: "[\"a\u{0}b\"]") }),
            ("description", { try check(description: "short\u{0} and the rest") }),
            ("contractConfigJSON", { try check(config: "{}\u{0}") }),
        ]
        for testCase in cases {
            XCTAssertThrowsError(try testCase.run(), testCase.name) { error in
                guard case PlatformWalletError.invalidParameter(let message) = error else {
                    return XCTFail("expected invalidParameter, got \(error)")
                }
                XCTAssertTrue(message.contains(testCase.name), message)
            }
        }

        XCTAssertNoThrow(
            try check(
                documents: "{}",
                tokens: "{}",
                groups: "{}",
                keywords: "[\"alpha\"]",
                description: "A description",
                config: "{}"
            )
        )
    }

    /// `Identifier` is `Data` of any length and `withFFIBytes` traps on a
    /// length other than 32, so the update and its preview refuse a wrong
    /// length, naming the argument, before any FFI call.
    func testShouldRefuseAnUpdateIdentifierThatIsNot32Bytes() {
        let valid = Data(repeating: 1, count: 32)
        let cases: [(name: String, owner: Data, contract: Data)] = [
            ("ownerIdentityId", Data(repeating: 1, count: 31), valid),
            ("contractId", valid, Data(repeating: 2, count: 33)),
            ("ownerIdentityId", Data(), valid),
        ]
        for testCase in cases {
            XCTAssertThrowsError(
                try ManagedPlatformWallet.requireDataContractUpdateIdentifiers(
                    ownerIdentityId: testCase.owner,
                    contractId: testCase.contract
                ),
                "\(testCase.name): owner \(testCase.owner.count) bytes, contract \(testCase.contract.count) bytes"
            ) { error in
                guard case PlatformWalletError.invalidParameter(let message) = error else {
                    return XCTFail("expected invalidParameter, got \(error)")
                }
                XCTAssertTrue(message.contains(testCase.name), message)
            }
        }

        XCTAssertNoThrow(
            try ManagedPlatformWallet.requireDataContractUpdateIdentifiers(
                ownerIdentityId: valid,
                contractId: Data(repeating: 2, count: 32)
            )
        )
    }
}
