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
        let config = """
            {"$formatVersion":"0","canBeDeleted":false,"readonly":false,"keepsHistory":false}
            """
        let exported = """
            {"$formatVersion":"1","id":"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",\
            "ownerId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF","version":3,\
            "config":\(config),\
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
        XCTAssertEqual(fields.config, try canonical(config))
    }

    /// The config is written as given, every member kept (a `null` member
    /// too), so an export that changes it is not dropped.
    func testShouldImportAnExportedContractsConfigVerbatim() throws {
        let config = """
            {"$formatVersion":"1","canBeDeleted":false,"readonly":false,"keepsHistory":false,\
            "documentsKeepHistoryContractDefault":false,"documentsMutableContractDefault":true,\
            "documentsCanBeDeletedContractDefault":true,"requiresIdentityEncryptionBoundedKey":null,\
            "requiresIdentityDecryptionBoundedKey":null,"sizedIntegerTypes":true}
            """
        let exported = """
            {"$formatVersion":"1","id":"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",\
            "ownerId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF","version":2,\
            "config":\(config),"schemaDefs":null,\
            "documentSchemas":{"note":{"type":"object","properties":{}}},"tokens":{},"groups":{},\
            "keywords":[],"description":null}
            """

        let fields = try UpdateContractImport.fields(from: exported)

        XCTAssertEqual(fields.config, try canonical(config))
        XCTAssertEqual(try UpdateContractImport.WalletArguments(fields).config, try canonical(config))
    }

    /// A `null` config and no config both keep the live one.
    func testShouldLeaveTheConfigEmptyWhenItIsNullOrAbsent() throws {
        for text in [
            #"{"config":null,"documentSchemas":{"note":{"type":"object","properties":{}}}}"#,
            #"{"documentSchemas":{"note":{"type":"object","properties":{}}}}"#,
        ] {
            let fields = try UpdateContractImport.fields(from: text)

            XCTAssertEqual(fields.config, "", text)
            XCTAssertNil(try UpdateContractImport.WalletArguments(fields).config, text)
        }
    }

    /// A contract without a description exports `"description": null`.
    func testShouldImportAnExportedContractWithoutADescription() throws {
        let exported = """
            {"$formatVersion":"1","id":"GWRSAVFMjXx8HpQFaNJMqBV7MBgMK4br5UESsB4S31Ec",\
            "ownerId":"4EfA9Jrvv3nnCFdSf7fad59851iiTRZ6Wcu6YVJ4iSeF","version":1,\
            "config":{"$formatVersion":"0","canBeDeleted":false},"schemaDefs":null,\
            "documentSchemas":{"note":{"type":"object","properties":{}}},"tokens":{},"groups":{},\
            "keywords":[],"description":null}
            """

        let fields = try UpdateContractImport.fields(from: exported)

        XCTAssertEqual(fields.description, "")
        XCTAssertNil(try UpdateContractImport.WalletArguments(fields).description)
        XCTAssertEqual(fields.documents, #"{"note":{"properties":{},"type":"object"}}"#)
    }

    func testShouldStillRefuseADescriptionThatIsNotAString() {
        XCTAssertThrowsError(try UpdateContractImport.fields(from: #"{"description":5}"#))
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
                description: "",
                config: ""
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

    /// Platform allows document types named like a contract section; a bare
    /// map of them is still read as document schemas.
    func testShouldImportABareMapWhoseTypeNamesMatchSectionNames() throws {
        let map = """
            {"documents":{"type":"object","properties":{"title":{"type":"string","position":0}}},\
            "description":{"type":"object","properties":{"text":{"type":"string","position":0}}}}
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

    // MARK: - Shared definitions

    /// An exported contract without definitions writes `"schemaDefs": null`.
    func testShouldNotReportSharedDefinitionsForANullSchemaDefs() throws {
        let imported = try UpdateContractImport.read(
            from: #"{"schemaDefs":null,"documentSchemas":{"note":{"type":"object","properties":{}}}}"#
        )

        XCTAssertFalse(imported.carriesSharedDefinitions)
    }

    func testShouldReportANonEmptySchemaDefsObject() throws {
        let text = """
            {"schemaDefs":{"label":{"type":"string","maxLength":32}},\
            "documentSchemas":{"note":{"type":"object","properties":{}}}}
            """

        let imported = try UpdateContractImport.read(from: text)

        XCTAssertTrue(imported.carriesSharedDefinitions)
        // The definitions are not written to any field.
        XCTAssertEqual(imported.fields.documents, #"{"note":{"properties":{},"type":"object"}}"#)
        XCTAssertEqual(imported.fields, try UpdateContractImport.fields(from: text))
    }

    func testShouldReportANonEmptyDollarDefsObject() throws {
        let imported = try UpdateContractImport.read(
            from: #"{"$defs":{"label":{"type":"string"}},"documentSchemas":{"note":{"type":"object","properties":{}}}}"#
        )

        XCTAssertTrue(imported.carriesSharedDefinitions)
    }

    func testShouldNotReportAnEmptyDefinitionsObject() throws {
        for key in ["schemaDefs", "$defs"] {
            let imported = try UpdateContractImport.read(
                from: #"{"\#(key)":{},"documentSchemas":{"note":{"type":"object","properties":{}}}}"#
            )

            XCTAssertFalse(imported.carriesSharedDefinitions, key)
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
