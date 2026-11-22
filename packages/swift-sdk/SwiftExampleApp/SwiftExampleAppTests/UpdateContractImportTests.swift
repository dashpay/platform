import XCTest
@testable import SwiftExampleApp

/// `UpdateContractImport` fills the Update Contract screen from pasted or
/// imported JSON without rewriting or dropping a supplied section, and
/// `UpdateContractImport.WalletArguments` hands the fields to the wallet.
final class UpdateContractImportTests: XCTestCase {

    /// `text` parsed and written back with sorted keys, as the importer
    /// writes a section.
    private func canonical(_ text: String) throws -> String {
        let value = try JSONSerialization.jsonObject(with: Data(text.utf8), options: [.fragmentsAllowed])
        let data = try JSONSerialization.data(
            withJSONObject: value,
            options: [.sortedKeys, .fragmentsAllowed, .withoutEscapingSlashes]
        )
        return try XCTUnwrap(String(data: data, encoding: .utf8))
    }

    // MARK: - Import

    /// The shape `dash_sdk_data_contract_fetch_with_serialization` returns:
    /// every section is imported as given, and the other keys are ignored.
    func testShouldImportEverySectionOfAnExportedContract() throws {
        let documents = """
            {"note":{"type":"object","properties":{"message":{"type":"string","position":0,"maxLength":63}},\
            "additionalProperties":false,"indices":[{"name":"byMessage","properties":[{"message":"asc"}]}]}}
            """
        let tokens = """
            {"0":{"$format_version":"0","baseSupply":1000,"conventions":{"$format_version":"0",\
            "decimals":2,"localizations":{"en":{"$format_version":"0","pluralForm":"Coins","shouldCapitalize":true,"singularForm":"Coin"}}}}}
            """
        let groups = """
            {"0":{"$format_version":"0","members":{"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF":1},"requiredPower":1}}
            """
        let exported = """
            {"$formatVersion":"1","id":"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",\
            "ownerId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF","version":3,\
            "config":{"$formatVersion":"0","canBeDeleted":false,"readonly":false,"keepsHistory":false},\
            "schemaDefs":null,\
            "documentSchemas":\(documents),"tokens":\(tokens),"groups":\(groups),\
            "keywords":["alpha","beta"],"description":"A contract"}
            """

        let fields = try UpdateContractImport.fields(from: exported)

        XCTAssertEqual(fields.documents, try canonical(documents))
        XCTAssertEqual(fields.tokens, try canonical(tokens))
        XCTAssertEqual(fields.groups, try canonical(groups))
        XCTAssertEqual(fields.keywords, #"["alpha","beta"]"#)
        XCTAssertEqual(fields.description, "A contract")
    }

    func testShouldKeepAKeywordThatContainsAComma() throws {
        let fields = try UpdateContractImport.fields(from: #"{"keywords":["alpha,beta","gamma"]}"#)

        XCTAssertEqual(fields.keywords, #"["alpha,beta","gamma"]"#)
        let arguments = try UpdateContractImport.WalletArguments(fields)
        XCTAssertEqual(arguments.keywords, #"["alpha,beta","gamma"]"#)
    }

    func testShouldKeepADescriptionsLeadingAndTrailingSpaces() throws {
        let fields = try UpdateContractImport.fields(from: #"{"description":"  spaced out  "}"#)

        XCTAssertEqual(fields.description, "  spaced out  ")
        XCTAssertEqual(try UpdateContractImport.WalletArguments(fields).description, "  spaced out  ")
    }

    func testShouldImportAGroupsOnlyFile() throws {
        let fields = try UpdateContractImport.fields(from: #"{"groups":{"1":{"members":{},"requiredPower":2}}}"#)

        XCTAssertEqual(
            fields,
            UpdateContractImport.Fields(
                documents: "{}",
                tokens: "",
                groups: #"{"1":{"members":{},"requiredPower":2}}"#,
                keywords: "",
                description: ""
            )
        )
    }

    func testShouldImportADescriptionOnlyFile() throws {
        let fields = try UpdateContractImport.fields(from: #"{"description":"Only this"}"#)

        var expected = UpdateContractImport.Fields.empty
        expected.description = "Only this"
        XCTAssertEqual(fields, expected)
    }

    /// The wallet refuses a malformed section; the import does not drop it.
    func testShouldPassAMalformedSectionThrough() throws {
        let fields = try UpdateContractImport.fields(from: #"{"groups":{"0":null},"tokens":"not tokens"}"#)

        XCTAssertEqual(fields.groups, #"{"0":null}"#)
        XCTAssertEqual(fields.tokens, #""not tokens""#)
    }

    func testShouldImportABareDocumentSchemasMap() throws {
        let map = """
            {"note":{"type":"object","properties":{"message":{"type":"string","position":0}}},\
            "tag":{"properties":{"name":{"type":"string","position":0}}}}
            """

        let fields = try UpdateContractImport.fields(from: map)

        var expected = UpdateContractImport.Fields.empty
        expected.documents = try canonical(map)
        XCTAssertEqual(fields, expected)
    }

    func testShouldRefuseANonObject() {
        for text in ["[1, 2]", "\"text\"", "42", "not json"] {
            XCTAssertThrowsError(try UpdateContractImport.fields(from: text), text)
        }
    }

    func testShouldRefuseAnUnrecognisedObjectNamingItsKeys() {
        XCTAssertThrowsError(try UpdateContractImport.fields(from: #"{"foo":1,"bar":{"baz":true}}"#)) { error in
            XCTAssertEqual(
                error as? UpdateContractImport.ImportError,
                .noSections(topLevelKeys: ["bar", "foo"])
            )
            XCTAssertTrue(error.localizedDescription.contains("bar, foo"), error.localizedDescription)
        }
    }

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
}
