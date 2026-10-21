import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `DataContractParser`'s token trade mode parsing.
///
/// rs-dpp serialises `TokenMarketplaceRules` flat, tagged with its format
/// version, and `TokenTradeMode` as a bare string:
///
/// ```json
/// "marketplaceRules": {
///   "$formatVersion": "0",
///   "tradeMode": "Tradeable",
///   "tradeModeChangeRules": { "$formatVersion": "0", ... }
/// }
/// ```
///
/// "Tradeable" is valid from protocol version 15. The Swift enum used to
/// have only `notTradeable`, so the parser's `TokenTradeMode(rawValue:)`
/// returned nil for it and the token kept the `notTradeable` default, which
/// made `PersistentToken.isTradeable` report false for a tradeable token.
///
/// These tests drive the public `parseDataContract` entry point against an
/// in-memory `ModelContainer` (no network, no FFI handle), save, and read the
/// token back through a fresh context so the stored value is what is checked.
@MainActor
final class DataContractParserTradeModeTests: XCTestCase {

    private let contractId = Data(repeating: 0xAB, count: 32)

    /// The parser's `parseTokens` bails out early unless a
    /// `PersistentDataContract` row already exists for `contractId`
    /// (tokens hang off the contract relationship). Seed that row, run the
    /// parser, save, and pass `body` the single token read through a new
    /// context on the same container. The context lives until `body` returns.
    private func withParsedToken(
        _ tokenDict: [String: Any],
        _ body: (PersistentToken) throws -> Void
    ) throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        let contract = PersistentDataContract(
            id: contractId,
            name: "Fixture",
            serializedContract: Data(),
            network: .testnet
        )
        context.insert(contract)
        try context.save()

        let contractData: [String: Any] = [
            "tokens": ["0": tokenDict]
        ]

        try DataContractParser.parseDataContract(
            contractData: contractData,
            contractId: contractId,
            modelContext: context
        )
        try context.save()

        let id = contractId
        let descriptor = FetchDescriptor<PersistentToken>(
            predicate: #Predicate { $0.contractId == id }
        )
        let readContext = ModelContext(container)
        let tokens = try readContext.fetch(descriptor)
        XCTAssertEqual(tokens.count, 1)
        let token = try XCTUnwrap(tokens.first, "parser should have persisted one token")
        try withExtendedLifetime(readContext) {
            try body(token)
        }
    }

    /// A token configuration whose `marketplaceRules` carries `tradeMode`
    /// in the real rs-dpp wire shape.
    private func tokenDict(tradeMode: String) -> [String: Any] {
        return [
            "$formatVersion": "0",
            "baseSupply": 0,
            "marketplaceRules": [
                "$formatVersion": "0",
                "tradeMode": tradeMode,
                "tradeModeChangeRules": [
                    "$formatVersion": "0",
                    "authorizedToMakeChange": [
                        "$type": AuthorizedActionTakers.WireType.contractOwner
                    ],
                    "adminActionTakers": [
                        "$type": AuthorizedActionTakers.WireType.noOne
                    ]
                ]
            ]
        ]
    }

    func testParsesTradeableTradeModeFromContractJSON() throws {
        try withParsedToken(tokenDict(tradeMode: "Tradeable")) { token in
            XCTAssertEqual(token.tradeMode, .tradeable)
            XCTAssertTrue(token.isTradeable)
            XCTAssertEqual(token.tradeMode.displayName, "Tradeable")
            // The change rules next to it still parse.
            XCTAssertEqual(
                token.tradeModeChangeRules?.authorizedToMakeChange,
                AuthorizedActionTakers.contractOwner.rawValue
            )
        }
    }

    func testNotTradeableTradeModeStaysNotTradeable() throws {
        try withParsedToken(tokenDict(tradeMode: "NotTradeable")) { token in
            XCTAssertEqual(token.tradeMode, .notTradeable)
            XCTAssertFalse(token.isTradeable)
            XCTAssertEqual(token.tradeMode.displayName, "Not Tradeable")
        }
    }

    /// The raw values are the strings rs-dpp writes; a rename on either side
    /// would silently fall back to the default again.
    func testTradeModeRawValuesMatchTheRustSerdeNames() {
        XCTAssertEqual(TokenTradeMode.notTradeable.rawValue, "NotTradeable")
        XCTAssertEqual(TokenTradeMode.tradeable.rawValue, "Tradeable")
    }
}
