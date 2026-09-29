import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Coverage for the protocol-version-14 `propertyConstraints` bridge: the
/// decoding of what `dash_sdk_data_contract_get_property_constraints` and
/// `dash_sdk_data_contract_check_property_constraints` return, and the
/// wrappers' round trip through the FFI.
///
/// Rust parses and evaluates the rules (rs-sdk-ffi's own tests cover every
/// rule family and violation); the Swift side only marshals, so these tests pin
/// the JSON shapes, which match wasm-dpp2's key for key, and the marshalling.
/// The round trips run on the FFI mock SDK, which sits at a protocol version
/// below 14, where no document type carries a rule.
@MainActor
final class DocumentPropertyConstraintsTests: XCTestCase {

    /// The rules of an `offer` type as the FFI reports them (rs-sdk-ffi's
    /// `should_list_every_rule_in_name_order_with_what_it_reads`), abridged to
    /// three rules.
    private let rulesJSON = """
        [
          {
            "name": "closedNeedsClosedAt",
            "readsOwner": false,
            "readsSystem": [],
            "reads": [
              { "kind": "text", "path": "status" },
              { "kind": "presence", "path": "closedAt" }
            ],
            "rule": {
              "ifThen": [
                { "equal": ["status", { "const": "closed" }] },
                { "present": "closedAt" }
              ]
            }
          },
          {
            "name": "perUnitFee",
            "readsOwner": false,
            "readsSystem": [],
            "reads": [
              { "kind": "value", "path": "price" },
              { "kind": "value", "path": "fee" }
            ],
            "rule": { "greaterThanOrEqual": [{ "divide": ["price", "fee"] }, 1] }
          },
          {
            "name": "sellerIsOwner",
            "readsOwner": true,
            "readsSystem": [],
            "reads": [
              { "kind": "presence", "path": "sellerId" },
              { "kind": "identifier", "path": "sellerId" }
            ],
            "rule": {
              "anyOf": [{ "absent": "sellerId" }, { "equal": ["sellerId", "$ownerId"] }]
            }
          }
        ]
        """

    /// Rules measuring sizes and looking among an array's elements, as the FFI
    /// reports them: the `length`, `count` and `elements` read kinds.
    private let sizeAndElementRulesJSON = """
        [
          {
            "name": "notUsed",
            "readsOwner": false,
            "readsSystem": [],
            "reads": [{ "kind": "elements", "path": "labels" }],
            "rule": { "not": { "contains": ["labels", { "const": "used" }] } }
          },
          {
            "name": "shortTitle",
            "readsOwner": false,
            "readsSystem": [],
            "reads": [
              { "kind": "length", "path": "title" },
              { "kind": "length", "path": "title" }
            ],
            "rule": {
              "allOf": [
                { "lessThanOrEqual": [{ "length": "title" }, 20] },
                { "lessThanOrEqual": [{ "byteLength": "title" }, 40] }
              ]
            }
          },
          {
            "name": "tagsFitSlots",
            "readsOwner": false,
            "readsSystem": [],
            "reads": [
              { "kind": "count", "path": "tags" },
              { "kind": "value", "path": "slots" }
            ],
            "rule": { "lessThanOrEqual": [{ "count": "tags" }, "slots"] }
          }
        ]
        """

    /// Rules reading system times and heights, as the FFI reports them: the
    /// `listing` type of rs-sdk-ffi's
    /// `should_read_the_clock_for_system_times_and_skip_block_heights`, and a
    /// rule comparing the last update with the last transfer.
    private let systemRulesJSON = """
        [
          {
            "name": "endsAfterCreation",
            "readsOwner": false,
            "readsSystem": ["$createdAt"],
            "reads": [{ "kind": "value", "path": "endsAt" }],
            "rule": { "greaterThan": ["endsAt", "$createdAt"] }
          },
          {
            "name": "listedAfterHeight10",
            "readsOwner": false,
            "readsSystem": ["$createdAtBlockHeight"],
            "reads": [],
            "rule": { "greaterThanOrEqual": ["$createdAtBlockHeight", 10] }
          },
          {
            "name": "repricedAfterTransfer",
            "readsOwner": false,
            "readsSystem": [
              "$updatedAt",
              "$transferredAt",
              "$updatedAtCoreBlockHeight",
              "$transferredAtCoreBlockHeight"
            ],
            "reads": [],
            "rule": {
              "allOf": [
                { "greaterThanOrEqual": ["$updatedAt", "$transferredAt"] },
                { "greaterThanOrEqual": ["$updatedAtCoreBlockHeight", "$transferredAtCoreBlockHeight"] }
              ]
            }
          }
        ]
        """

    /// Rules reading `countOf` and `sumOf` totals, as the FFI reports them:
    /// the `listing` type of rs-sdk-ffi's
    /// `should_list_the_totals_a_rule_reads_and_leave_it_unjudged`, whose
    /// trees keep its count, each owner's count and each category's total
    /// price, and a rule, `priceCap`, reading only the price.
    private let totalsRulesJSON = """
        [
          {
            "name": "allListings",
            "rule": { "lessThan": [{ "countOf": ["listing"] }, 1000] },
            "reads": [],
            "readsOwner": false,
            "readsSystem": [],
            "readsTotals": [
              { "kind": "countOf", "documentType": "listing", "filter": [] }
            ]
          },
          {
            "name": "atMostTwoPerOwner",
            "rule": {
              "lessThanOrEqual": [
                { "countOf": ["listing", { "$ownerId": "$ownerId" }] },
                2
              ]
            },
            "reads": [],
            "readsOwner": true,
            "readsSystem": [],
            "readsTotals": [
              { "kind": "countOf", "documentType": "listing", "filter": ["$ownerId"] }
            ]
          },
          {
            "name": "categoryBudget",
            "rule": {
              "lessThanOrEqual": [
                { "sumOf": ["listing", "price", { "category": "category" }] },
                250
              ]
            },
            "reads": [{ "path": "category", "kind": "value" }],
            "readsOwner": false,
            "readsSystem": [],
            "readsTotals": [
              {
                "kind": "sumOf",
                "documentType": "listing",
                "property": "price",
                "filter": ["category"]
              }
            ]
          },
          {
            "name": "priceCap",
            "rule": { "lessThanOrEqual": ["price", 1000] },
            "reads": [{ "path": "price", "kind": "value" }],
            "readsOwner": false,
            "readsSystem": [],
            "readsTotals": []
          }
        ]
        """

    /// The platform serialization of a contract created at protocol version
    /// 13, owned by `[7; 32]`, declaring one `note` type with a string
    /// `message` and no rules (generated by rs-sdk-ffi's
    /// `serialize_to_bytes_with_platform_version`). It reads at every later
    /// protocol version too.
    private let noteContractHex =
        "013ac40651a4bcb91c3f5c1e458a1c95e48dc2c8259003dd580cab11a5f02ed4850100000000010100000101070707070707"
        + "07070707070707070707070707070707070707070707070707070001046e6f7465160312047479706512066f626a65637412"
        + "0a70726f70657274696573160112076d65737361676516031204747970651206737472696e6712096d61784c656e67746805"
        + "801208706f736974696f6e050012146164646974696f6e616c50726f70657274696573130000000000000000000000"

    private let ownerId = Data(repeating: 7, count: 32)

    // MARK: - Rules

    func testRulesDecodeInOrderWithWhatTheyRead() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: rulesJSON)

        XCTAssertEqual(rules.map(\.name), ["closedNeedsClosedAt", "perUnitFee", "sellerIsOwner"])
        XCTAssertEqual(
            rules[0].reads,
            [
                PropertyConstraintRead(path: "status", kind: .text),
                PropertyConstraintRead(path: "closedAt", kind: .presence)
            ]
        )
        XCTAssertEqual(rules[1].reads.map(\.kind), [.value, .value])
        XCTAssertEqual(rules[2].reads.map(\.kind), [.presence, .identifier])
        XCTAssertEqual(rules.map(\.readsOwner), [false, false, true])
        XCTAssertEqual(rules.map(\.readsSystem), [[], [], []])
    }

    /// The rule is kept as the JSON the schema declares, compact with sorted
    /// keys: array order, and so operand order, is untouched.
    func testRuleIsKeptAsDeclaredJSON() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: rulesJSON)

        XCTAssertEqual(
            rules[0].ruleJSON,
            #"{"ifThen":[{"equal":["status",{"const":"closed"}]},{"present":"closedAt"}]}"#
        )
        XCTAssertEqual(rules[1].ruleJSON, #"{"greaterThanOrEqual":[{"divide":["price","fee"]},1]}"#)
        XCTAssertEqual(
            rules[2].ruleJSON,
            #"{"anyOf":[{"absent":"sellerId"},{"equal":["sellerId","$ownerId"]}]}"#
        )
    }

    /// An `ifThenElse` lists what every branch reads, whichever one a document
    /// takes, the owner and the system times included (rs-sdk-ffi's
    /// `should_report_every_branch_of_an_if_then_else_and_judge_the_one_taken`).
    func testIfThenElseReadsEveryBranch() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: """
            [
              {
                "name": "openEndedSoldByOwner",
                "rule": {
                  "ifThenElse": [
                    { "absent": "endsAt" },
                    { "equal": ["sellerId", "$ownerId"] },
                    { "greaterThan": ["endsAt", "$createdAt"] }
                  ]
                },
                "reads": [
                  { "path": "endsAt", "kind": "presence" },
                  { "path": "sellerId", "kind": "identifier" },
                  { "path": "endsAt", "kind": "value" }
                ],
                "readsOwner": true,
                "readsSystem": ["$createdAt"]
              }
            ]
            """)

        XCTAssertEqual(rules.count, 1)
        let rule = try XCTUnwrap(rules.first)
        XCTAssertEqual(rule.name, "openEndedSoldByOwner")
        XCTAssertEqual(
            rule.reads,
            [
                PropertyConstraintRead(path: "endsAt", kind: .presence),
                PropertyConstraintRead(path: "sellerId", kind: .identifier),
                PropertyConstraintRead(path: "endsAt", kind: .value)
            ]
        )
        XCTAssertTrue(rule.readsOwner)
        XCTAssertEqual(rule.readsSystem, ["$createdAt"])
        XCTAssertEqual(
            rule.ruleJSON,
            #"{"ifThenElse":[{"absent":"endsAt"},{"equal":["sellerId","$ownerId"]},{"greaterThan":["endsAt","$createdAt"]}]}"#
        )
    }

    func testPrettyRuleJSONIndentsTheSameRule() throws {
        let rule = try XCTUnwrap(DocumentPropertyConstraint.list(fromJSON: rulesJSON).first)

        let pretty = rule.prettyRuleJSON
        XCTAssertTrue(pretty.contains("\n"), pretty)
        let reparsed = try JSONSerialization.jsonObject(with: Data(pretty.utf8)) as? NSDictionary
        let original = try JSONSerialization.jsonObject(with: Data(rule.ruleJSON.utf8)) as? NSDictionary
        XCTAssertEqual(reparsed, original)
    }

    func testNoRulesDecodeToAnEmptyList() throws {
        XCTAssertEqual(try DocumentPropertyConstraint.list(fromJSON: "[]"), [])
    }

    /// A kind added by a later protocol version is kept by name rather than
    /// failing the whole list.
    func testUnknownReadKindIsKeptByName() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: """
            [{ "name": "r", "rule": { "present": "a" }, "readsOwner": false,
               "reads": [{ "path": "a", "kind": "somethingNew" }] }]
            """)

        XCTAssertEqual(rules.first?.reads.first?.kind, .other("somethingNew"))
        XCTAssertEqual(rules.first?.reads.first?.kind.name, "somethingNew")
    }

    func testMalformedRulesAreRefused() {
        let malformed = [
            "not json",
            #"{"name": "r"}"#,
            #"[{"name": "r", "rule": {"present": "a"}, "reads": []}]"#,
            // A number is not a boolean, although NSNumber would cast it
            #"[{"name": "r", "rule": {"present": "a"}, "reads": [], "readsOwner": 1}]"#,
            #"[{"name": "r", "rule": {"present": "a"}, "reads": [{"path": "a"}], "readsOwner": false}]"#
        ]
        for json in malformed {
            XCTAssertThrowsError(try DocumentPropertyConstraint.list(fromJSON: json), json) { error in
                guard case SDKError.serializationError = error else {
                    return XCTFail("\(json): expected a serialization error, got \(error)")
                }
            }
        }
    }

    // MARK: - Read kinds

    func testSizeAndElementReadsDecodeToTheirKinds() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: sizeAndElementRulesJSON)

        XCTAssertEqual(rules.map(\.name), ["notUsed", "shortTitle", "tagsFitSlots"])
        XCTAssertEqual(rules[0].reads, [PropertyConstraintRead(path: "labels", kind: .elements)])
        // `length` and `byteLength` both read a string's size
        XCTAssertEqual(
            rules[1].reads,
            [
                PropertyConstraintRead(path: "title", kind: .length),
                PropertyConstraintRead(path: "title", kind: .length)
            ]
        )
        XCTAssertEqual(
            rules[2].reads,
            [
                PropertyConstraintRead(path: "tags", kind: .count),
                PropertyConstraintRead(path: "slots", kind: .value)
            ]
        )
        XCTAssertEqual(
            rules[1].ruleJSON,
            #"{"allOf":[{"lessThanOrEqual":[{"length":"title"},20]},{"lessThanOrEqual":[{"byteLength":"title"},40]}]}"#
        )
    }

    func testEveryReadKindNameRoundTrips() {
        let names = ["value", "presence", "text", "identifier", "length", "count", "elements"]
        let kinds: [PropertyConstraintRead.Kind] = [
            .value, .presence, .text, .identifier, .length, .count, .elements
        ]
        XCTAssertEqual(names.map(PropertyConstraintRead.Kind.init(name:)), kinds)
        XCTAssertEqual(kinds.map(\.name), names)
        XCTAssertEqual(PropertyConstraintRead.Kind(name: "Length"), .other("Length"))
    }

    // MARK: - System reads

    func testSystemReadsDecodeInDeclaredOrder() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: systemRulesJSON)

        XCTAssertEqual(
            rules.map(\.readsSystem),
            [
                ["$createdAt"],
                ["$createdAtBlockHeight"],
                [
                    "$updatedAt",
                    "$transferredAt",
                    "$updatedAtCoreBlockHeight",
                    "$transferredAtCoreBlockHeight"
                ]
            ]
        )
        // A system time or height is no property, and not the owner
        XCTAssertEqual(rules[0].reads, [PropertyConstraintRead(path: "endsAt", kind: .value)])
        XCTAssertEqual(rules[1].reads, [])
        XCTAssertEqual(rules[2].reads, [])
        XCTAssertEqual(rules.map(\.readsOwner), [false, false, false])
    }

    /// Every name Rust reports is kept as it is, in order, a system value read
    /// twice listed twice.
    func testEverySystemNameIsKeptVerbatimWithRepeats() throws {
        let names = [
            "$createdAt",
            "$updatedAt",
            "$transferredAt",
            "$createdAtBlockHeight",
            "$updatedAtBlockHeight",
            "$transferredAtBlockHeight",
            "$createdAtCoreBlockHeight",
            "$updatedAtCoreBlockHeight",
            "$transferredAtCoreBlockHeight"
        ]
        let quoted = names.map { "\"\($0)\"" }.joined(separator: ", ")

        let rules = try DocumentPropertyConstraint.list(fromJSON: """
            [{ "name": "r", "readsOwner": false, "reads": [],
               "readsSystem": [\(quoted), "$createdAt"],
               "rule": { "greaterThan": [{ "add": [\(quoted)] }, "$createdAt"] } }]
            """)

        XCTAssertEqual(rules.first?.readsSystem, names + ["$createdAt"])
    }

    /// A library built before `readsSystem` leaves the key out.
    func testMissingSystemReadsDecodeAsEmpty() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: """
            [{ "name": "r", "rule": { "present": "a" }, "readsOwner": false,
               "reads": [{ "path": "a", "kind": "presence" }] }]
            """)

        XCTAssertEqual(rules.first?.readsSystem, [])
    }

    func testMalformedSystemReadsAreRefused() {
        let rule = #""name": "r", "rule": {"present": "a"}, "reads": [], "readsOwner": false"#
        let malformed = [
            // Not an array
            #"[{\#(rule), "readsSystem": "$createdAt"}]"#,
            #"[{\#(rule), "readsSystem": {"$createdAt": true}}]"#,
            // A null is no missing key
            #"[{\#(rule), "readsSystem": null}]"#,
            // An element that is not a string
            #"[{\#(rule), "readsSystem": ["$createdAt", 1]}]"#,
            #"[{\#(rule), "readsSystem": [["$createdAt"]]}]"#
        ]
        for json in malformed {
            XCTAssertThrowsError(try DocumentPropertyConstraint.list(fromJSON: json), json) { error in
                guard case SDKError.serializationError = error else {
                    return XCTFail("\(json): expected a serialization error, got \(error)")
                }
            }
        }
    }

    /// The initializer still builds a rule without naming `readsSystem`, as it
    /// did before the field existed.
    func testInitializerDefaultsToNoSystemReads() {
        let rule = DocumentPropertyConstraint(
            name: "r",
            ruleJSON: #"{"present":"a"}"#,
            reads: [PropertyConstraintRead(path: "a", kind: .presence)],
            readsOwner: false
        )

        XCTAssertEqual(rule.readsSystem, [])
        XCTAssertNotEqual(
            rule,
            DocumentPropertyConstraint(
                name: "r",
                ruleJSON: #"{"present":"a"}"#,
                reads: [PropertyConstraintRead(path: "a", kind: .presence)],
                readsOwner: false,
                readsSystem: ["$createdAt"]
            )
        )
    }

    // MARK: - Total reads

    func testTotalReadsDecodeWithTheirTypeFilterAndProperty() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: totalsRulesJSON)

        XCTAssertEqual(
            rules.map(\.name),
            ["allListings", "atMostTwoPerOwner", "categoryBudget", "priceCap"]
        )
        XCTAssertEqual(
            rules.map(\.readsTotals),
            [
                [PropertyConstraintTotalRead(kind: .countOf, documentType: "listing", filter: [])],
                [PropertyConstraintTotalRead(kind: .countOf, documentType: "listing", filter: ["$ownerId"])],
                [
                    PropertyConstraintTotalRead(
                        kind: .sumOf,
                        documentType: "listing",
                        property: "price",
                        filter: ["category"]
                    )
                ],
                []
            ]
        )
        // A `countOf` sums nothing
        XCTAssertNil(rules[0].readsTotals.first?.property)
        XCTAssertNil(rules[1].readsTotals.first?.property)
        // A total is no property of the document; the property its filter
        // takes a value from is
        XCTAssertEqual(rules[0].reads, [])
        XCTAssertEqual(rules[1].reads, [])
        XCTAssertEqual(rules[2].reads, [PropertyConstraintRead(path: "category", kind: .value)])
        XCTAssertEqual(rules[3].reads, [PropertyConstraintRead(path: "price", kind: .value)])
        // Counting the owner's listings depends on the owner
        XCTAssertEqual(rules.map(\.readsOwner), [false, true, false, false])
        XCTAssertEqual(rules.map(\.readsSystem), [[], [], [], []])
        XCTAssertEqual(
            rules[1].ruleJSON,
            #"{"lessThanOrEqual":[{"countOf":["listing",{"$ownerId":"$ownerId"}]},2]}"#
        )
    }

    /// Totals are kept in declared order, one read twice listed twice, and a
    /// total added by a later protocol version is kept by name rather than
    /// failing the whole list.
    func testTotalReadsKeepRepeatsAndUnknownKinds() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: """
            [{ "name": "r", "readsOwner": false, "reads": [], "readsSystem": [],
               "readsTotals": [
                 { "kind": "sumOf", "documentType": "pledge", "property": "amount", "filter": [] },
                 { "kind": "averageOf", "documentType": "pledge", "property": "amount",
                   "filter": ["campaignId", "$ownerId"] },
                 { "kind": "sumOf", "documentType": "pledge", "property": "amount", "filter": [] }
               ],
               "rule": { "lessThan": [{ "sumOf": ["pledge", "amount"] }, 10] } }]
            """)

        let sum = PropertyConstraintTotalRead(
            kind: .sumOf,
            documentType: "pledge",
            property: "amount",
            filter: []
        )
        XCTAssertEqual(
            rules.first?.readsTotals,
            [
                sum,
                PropertyConstraintTotalRead(
                    kind: .other("averageOf"),
                    documentType: "pledge",
                    property: "amount",
                    filter: ["campaignId", "$ownerId"]
                ),
                sum
            ]
        )
        XCTAssertEqual(rules.first?.readsTotals[1].kind.name, "averageOf")
    }

    func testEveryTotalKindNameRoundTrips() {
        let names = ["countOf", "sumOf"]
        let kinds: [PropertyConstraintTotalRead.Kind] = [.countOf, .sumOf]
        XCTAssertEqual(names.map(PropertyConstraintTotalRead.Kind.init(name:)), kinds)
        XCTAssertEqual(kinds.map(\.name), names)
        XCTAssertEqual(PropertyConstraintTotalRead.Kind(name: "CountOf"), .other("CountOf"))
        XCTAssertEqual(PropertyConstraintTotalRead.Kind(name: "averageOf").name, "averageOf")
    }

    /// A library built before `readsTotals` leaves the key out.
    func testMissingTotalReadsDecodeAsEmpty() throws {
        let rules = try DocumentPropertyConstraint.list(fromJSON: """
            [{ "name": "r", "rule": { "present": "a" }, "readsOwner": false,
               "readsSystem": [], "reads": [{ "path": "a", "kind": "presence" }] }]
            """)

        XCTAssertEqual(rules.first?.readsTotals, [])
    }

    func testMalformedTotalReadsAreRefused() {
        let rule = #""name": "r", "rule": {"present": "a"}, "reads": [], "readsOwner": false"#
        let count = #""kind": "countOf", "documentType": "listing""#
        let malformed = [
            // Not an array
            #"[{\#(rule), "readsTotals": "countOf"}]"#,
            #"[{\#(rule), "readsTotals": {\#(count), "filter": []}}]"#,
            // A null is no missing key
            #"[{\#(rule), "readsTotals": null}]"#,
            // An element that is not an object
            #"[{\#(rule), "readsTotals": ["countOf"]}]"#,
            #"[{\#(rule), "readsTotals": [[{\#(count), "filter": []}]]}]"#,
            // A missing or mistyped kind
            #"[{\#(rule), "readsTotals": [{"documentType": "listing", "filter": []}]}]"#,
            #"[{\#(rule), "readsTotals": [{"kind": 1, "documentType": "listing", "filter": []}]}]"#,
            // A missing or mistyped document type
            #"[{\#(rule), "readsTotals": [{"kind": "countOf", "filter": []}]}]"#,
            #"[{\#(rule), "readsTotals": [{"kind": "countOf", "documentType": null, "filter": []}]}]"#,
            // A missing or mistyped filter
            #"[{\#(rule), "readsTotals": [{\#(count)}]}]"#,
            #"[{\#(rule), "readsTotals": [{\#(count), "filter": null}]}]"#,
            #"[{\#(rule), "readsTotals": [{\#(count), "filter": "$ownerId"}]}]"#,
            #"[{\#(rule), "readsTotals": [{\#(count), "filter": {"$ownerId": "$ownerId"}}]}]"#,
            // A filter key that is not a string
            #"[{\#(rule), "readsTotals": [{\#(count), "filter": ["$ownerId", 1]}]}]"#,
            #"[{\#(rule), "readsTotals": [{\#(count), "filter": [null]}]}]"#,
            // A property that is present but not a string
            #"[{\#(rule), "readsTotals": [{"kind": "sumOf", "documentType": "listing", "property": 1, "filter": []}]}]"#,
            #"[{\#(rule), "readsTotals": [{"kind": "sumOf", "documentType": "listing", "property": null, "filter": []}]}]"#,
            #"[{\#(rule), "readsTotals": [{"kind": "sumOf", "documentType": "listing", "property": ["price"], "filter": []}]}]"#
        ]
        // The same rule with well-formed totals decodes
        XCTAssertNoThrow(try DocumentPropertyConstraint.list(fromJSON: """
            [{\(rule), "readsTotals": [{\(count), "filter": ["$ownerId"]},
              {"kind": "sumOf", "documentType": "listing", "property": "price", "filter": []}]}]
            """))
        for json in malformed {
            // Each is JSON, so what is refused is its readsTotals
            XCTAssertNoThrow(try JSONSerialization.jsonObject(with: Data(json.utf8)), json)
            XCTAssertThrowsError(try DocumentPropertyConstraint.list(fromJSON: json), json) { error in
                guard case SDKError.serializationError = error else {
                    return XCTFail("\(json): expected a serialization error, got \(error)")
                }
            }
        }
    }

    /// The initializer still builds a rule without naming `readsTotals`, as it
    /// did before the field existed.
    func testInitializerDefaultsToNoTotalReads() {
        let rule = DocumentPropertyConstraint(
            name: "r",
            ruleJSON: #"{"lessThan":["price",1000]}"#,
            reads: [PropertyConstraintRead(path: "price", kind: .value)],
            readsOwner: false,
            readsSystem: []
        )

        XCTAssertEqual(rule.readsTotals, [])
        XCTAssertNotEqual(
            rule,
            DocumentPropertyConstraint(
                name: "r",
                ruleJSON: #"{"lessThan":["price",1000]}"#,
                reads: [PropertyConstraintRead(path: "price", kind: .value)],
                readsOwner: false,
                readsSystem: [],
                readsTotals: [PropertyConstraintTotalRead(kind: .countOf, documentType: "listing", filter: [])]
            )
        )
    }

    // MARK: - Violations

    func testViolationDecodes() throws {
        let violation = try XCTUnwrap(PropertyConstraintViolation.decode(fromJSON: """
            { "rule": "perUnitFee", "violation": "DivisionByZero", "message": "it divides by zero" }
            """))

        XCTAssertEqual(
            violation,
            PropertyConstraintViolation(
                rule: "perUnitFee",
                violation: .divisionByZero,
                message: "it divides by zero"
            )
        )
        XCTAssertEqual(
            violation.localizedDescription,
            "The document breaks the propertyConstraints rule \"perUnitFee\" (DivisionByZero): it divides by zero."
        )
    }

    func testEveryViolationNameRoundTrips() {
        let names = ["NotMet", "Overflow", "DivisionByZero", "NegativeExponent", "NotAnInteger"]
        let kinds: [PropertyConstraintViolation.Kind] = [
            .notMet, .overflow, .divisionByZero, .negativeExponent, .notAnInteger
        ]
        XCTAssertEqual(names.map(PropertyConstraintViolation.Kind.init(name:)), kinds)
        XCTAssertEqual(kinds.map(\.name), names)
        XCTAssertEqual(PropertyConstraintViolation.Kind(name: "Later"), .other("Later"))
    }

    func testNullMeansEveryRuleHolds() throws {
        XCTAssertNil(try PropertyConstraintViolation.decode(fromJSON: "null"))
    }

    func testMalformedViolationsAreRefused() {
        for json in ["", "[]", #"{"rule": "r", "violation": "NotMet"}"#] {
            XCTAssertThrowsError(try PropertyConstraintViolation.decode(fromJSON: json), json)
        }
    }

    // MARK: - FFI round trips

    func testWrappersRoundTripThroughTheFFI() throws {
        let sdk = try mockSDK()
        let contract = try noteContract()

        XCTAssertEqual(
            try sdk.documentPropertyConstraints(serializedContract: contract, documentType: "note"),
            []
        )
        XCTAssertNil(
            try sdk.checkDocumentPropertyConstraints(
                serializedContract: contract,
                documentType: "note",
                propertiesJSON: #"{"message":"hi"}"#,
                ownerId: ownerId
            )
        )
    }

    func testFFIErrorsKeepTheirCodes() throws {
        let sdk = try mockSDK()
        let contract = try noteContract()

        assertThrows(
            try sdk.documentPropertyConstraints(serializedContract: contract, documentType: "letter")
        ) { error in
            if case SDKError.notFound = error { return true }
            return false
        }
        assertThrows(
            try sdk.documentPropertyConstraints(serializedContract: Data([0xFF, 0x00, 0x13]), documentType: "note")
        ) { error in
            if case SDKError.serializationError = error { return true }
            return false
        }
        assertThrows(
            try sdk.documentPropertyConstraints(serializedContract: Data(), documentType: "note")
        ) { error in
            if case SDKError.invalidParameter = error { return true }
            return false
        }
        assertThrows(
            try sdk.checkDocumentPropertyConstraints(
                serializedContract: contract,
                documentType: "note",
                propertiesJSON: "[1]",
                ownerId: ownerId
            )
        ) { error in
            if case SDKError.invalidParameter = error { return true }
            return false
        }
    }

    /// The FFI reads 32 bytes behind the owner pointer, so a shorter id is
    /// refused before the call.
    func testOwnerIdMustBe32Bytes() throws {
        let sdk = try mockSDK()

        assertThrows(
            try sdk.checkDocumentPropertyConstraints(
                serializedContract: try noteContract(),
                documentType: "note",
                propertiesJSON: "{}",
                ownerId: Data(repeating: 7, count: 20)
            )
        ) { error in
            if case SDKError.invalidParameter = error { return true }
            return false
        }
    }

    // MARK: - PersistentDocumentType

    func testDocumentTypeReadsThroughItsStoredContract() throws {
        let sdk = try mockSDK()
        let stored = try persistNoteType(binarySerialization: try noteContract())
        let docType = stored.documentType

        XCTAssertFalse(docType.declaresPropertyConstraints)
        XCTAssertEqual(try docType.propertyConstraints(using: sdk), [])
        XCTAssertNil(
            try docType.propertyConstraintViolation(
                propertiesJSON: #"{"message":"hi"}"#,
                ownerId: ownerId,
                using: sdk
            )
        )
        withExtendedLifetime(stored.context) {}
    }

    func testDocumentTypeWithoutAStoredContractCannotBeRead() throws {
        let sdk = try mockSDK()
        let stored = try persistNoteType(binarySerialization: nil)

        assertThrows(try stored.documentType.propertyConstraints(using: sdk)) { error in
            if case SDKError.invalidState = error { return true }
            return false
        }
        withExtendedLifetime(stored.context) {}
    }

    func testDeclaresPropertyConstraintsReadsTheKeyword() {
        let schema: [String: Any] = [
            "type": "object",
            "propertyConstraints": ["r": ["present": "a"]]
        ]
        let docType = PersistentDocumentType(
            contractId: Data(repeating: 1, count: 32),
            name: "offer",
            schemaJSON: (try? JSONSerialization.data(withJSONObject: schema)) ?? Data(),
            propertiesJSON: Data("{}".utf8)
        )

        XCTAssertTrue(docType.declaresPropertyConstraints)
    }

    // MARK: - Helpers

    private func mockSDK() throws -> SDK {
        SDK.initialize()
        return try SDK(mockVectorsDirectory: nil)
    }

    private func noteContract() throws -> Data {
        let contract = try XCTUnwrap(Data(hexString: noteContractHex))
        // `Data(hexString:)` ignores a trailing odd digit: pin the whole fixture
        XCTAssertEqual(contract.count * 2, noteContractHex.count)
        return contract
    }

    /// A persisted document type and the context holding it, which the caller
    /// keeps alive while it reads the type's relationships.
    private struct StoredDocumentType {
        let documentType: PersistentDocumentType
        let context: ModelContext
    }

    /// Persist the `note` contract and its type the way a download does: the
    /// contract row carrying `binarySerialization`, and the parser writing the
    /// type row.
    private func persistNoteType(binarySerialization: Data?) throws -> StoredDocumentType {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let contractId = Data(repeating: 0xC3, count: 32)

        let contract = PersistentDataContract(
            id: contractId,
            name: "Notes",
            serializedContract: Data(),
            network: .testnet
        )
        contract.binarySerialization = binarySerialization
        context.insert(contract)
        try context.save()

        try DataContractParser.parseDataContract(
            contractData: [
                "documents": [
                    "note": [
                        "type": "object",
                        "properties": ["message": ["type": "string", "maxLength": 64, "position": 0]],
                        "additionalProperties": false
                    ]
                ]
            ],
            contractId: contractId,
            modelContext: context
        )

        let descriptor = FetchDescriptor<PersistentDocumentType>(
            predicate: #Predicate { $0.contractId == contractId }
        )
        let docType = try XCTUnwrap(try context.fetch(descriptor).first)
        return StoredDocumentType(documentType: docType, context: context)
    }

    private func assertThrows<T>(
        _ expression: @autoclosure () throws -> T,
        file: StaticString = #filePath,
        line: UInt = #line,
        matching matches: (Error) -> Bool
    ) {
        XCTAssertThrowsError(try expression(), file: file, line: line) { error in
            XCTAssertTrue(matches(error), "unexpected error: \(error)", file: file, line: line)
        }
    }
}
