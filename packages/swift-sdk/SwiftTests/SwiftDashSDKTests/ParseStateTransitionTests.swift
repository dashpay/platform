import XCTest

@testable import SwiftDashSDK

/// `ManagedPlatformWallet.parseStateTransition` decodes any DPP state
/// transition kind and returns a typed summary. The fixtures under
/// `Fixtures/StateTransitions/` are serialized on the Rust side by
/// `parse_state_transition::tests::fixture_bytes_are_pinned_for_the_client_suites`
/// (one hex line each), so the bytes decoded here are exactly the ones the
/// Rust tests decode.
@MainActor
final class ParseStateTransitionTests: XCTestCase {

    private let owner = Data(repeating: 0x21, count: 32)
    private let contract = Data(repeating: 0x42, count: 32)
    private let token = Data(repeating: 0x77, count: 32)
    private let recipient = Data(repeating: 0x22, count: 32)

    /// The parse is a pure FFI call over the bytes; the wallet handle is
    /// never touched, so a null handle is fine.
    private let wallet = ManagedPlatformWallet(
        handle: NULL_HANDLE, walletId: Data(repeating: 0x07, count: 32))

    private func fixture(_ name: String) throws -> Data {
        let url = try XCTUnwrap(
            Bundle.module.url(
                forResource: name, withExtension: "hex",
                subdirectory: "Fixtures/StateTransitions"),
            "missing fixture \(name)")
        let hex = try String(contentsOf: url, encoding: .utf8)
            .trimmingCharacters(in: .whitespacesAndNewlines)
        return try XCTUnwrap(Data(hexString: hex), "fixture \(name) is not hex")
    }

    func testParsesAMixedBatchWithOneEntryPerTransition() throws {
        let bytes = try fixture("mixed_batch")
        let parsed = try wallet.parseStateTransition(bytes)

        XCTAssertEqual(
            parsed.kindName, "DocumentsBatch([Create, Transfer, TokenTransfer, TokenDirectPurchase])")
        XCTAssertEqual(parsed.ownerId, owner)
        XCTAssertFalse(parsed.isSigned)
        XCTAssertEqual(parsed.serialized, bytes, "tagged input decodes as-is")

        guard case .batch(let batch) = parsed.kind else {
            return XCTFail("expected .batch, got \(parsed.kind)")
        }
        XCTAssertEqual(batch.ownerId, owner)
        XCTAssertEqual(batch.transitions.count, 4)

        XCTAssertEqual(
            batch.transitions[0],
            .document(
                dataContractId: contract, documentType: "post",
                documentId: Data(repeating: 0x0D, count: 32), action: "Create",
                amount: nil, recipientId: nil))
        XCTAssertEqual(
            batch.transitions[1],
            .document(
                dataContractId: contract, documentType: "profile",
                documentId: Data(repeating: 0x0D, count: 32), action: "Transfer",
                amount: nil, recipientId: recipient))
        XCTAssertEqual(
            batch.transitions[2],
            .token(
                dataContractId: contract, tokenId: token, tokenContractPosition: 3,
                action: "Transfer", amount: 250, recipientId: recipient))
        XCTAssertEqual(
            batch.transitions[3],
            .token(
                dataContractId: contract, tokenId: token, tokenContractPosition: 3,
                action: "DirectPurchase", amount: 100_000_000, recipientId: nil))
        XCTAssertEqual(batch.transitions.map(\.action), ["Create", "Transfer", "Transfer", "DirectPurchase"])
    }

    func testParsesATaglessBatchAndReturnsTaggedBytes() throws {
        let tagged = try fixture("mixed_batch")
        XCTAssertEqual(tagged.first, 2, "StateTransition::Batch variant tag")
        let parsed = try wallet.parseStateTransition(tagged.dropFirst())

        guard case .batch = parsed.kind else {
            return XCTFail("expected .batch, got \(parsed.kind)")
        }
        XCTAssertEqual(parsed.serialized, tagged, "the variant tag is restored")
    }

    func testParsesAnIdentityUpdateWithKeyLimitsAndBounds() throws {
        let parsed = try wallet.parseStateTransition(try fixture("identity_update"))

        XCTAssertEqual(parsed.kindName, "IdentityUpdate")
        XCTAssertEqual(parsed.ownerId, Data(repeating: 0x11, count: 32))
        XCTAssertTrue(parsed.isSigned)

        guard case .identityUpdate(let update) = parsed.kind else {
            return XCTFail("expected .identityUpdate, got \(parsed.kind)")
        }
        XCTAssertEqual(update.identityId, Data(repeating: 0x11, count: 32))
        XCTAssertEqual(update.disablePublicKeyIds, [4, 8])
        XCTAssertEqual(update.addPublicKeys.count, 2)

        let plain = update.addPublicKeys[0]
        XCTAssertEqual(plain.keyId, 17)
        XCTAssertNil(plain.totalBudget)
        XCTAssertNil(plain.expiresAt)
        XCTAssertNil(plain.contractBounds)

        // A DashPay Connect session key: HIGH auth key bound to a contract
        // group with a budget and an expiry.
        let session = update.addPublicKeys[1]
        XCTAssertEqual(session.keyId, 18)
        XCTAssertEqual(session.purpose, .authentication)
        XCTAssertEqual(session.securityLevel, .high)
        XCTAssertEqual(session.keyType, .ecdsaSecp256k1)
        XCTAssertEqual(session.pubkeyBytes, Data(repeating: 0x03, count: 33))
        XCTAssertEqual(session.contractBounds, .contractGroup(id: Data(repeating: 0x66, count: 32)))
        XCTAssertEqual(session.totalBudget, 10_000_000_000)
        XCTAssertEqual(session.expiresAt, 1_800_000_000_000)
    }

    func testParsesACreditTransfer() throws {
        let parsed = try wallet.parseStateTransition(try fixture("credit_transfer"))

        XCTAssertEqual(parsed.kindName, "IdentityCreditTransfer")
        XCTAssertFalse(parsed.isSigned)
        XCTAssertEqual(
            parsed.kind,
            .creditTransfer(
                ParsedCreditTransferTransition(
                    identityId: Data(repeating: 0x11, count: 32),
                    recipientId: recipient,
                    amount: 1_000)))
    }

    func testParsesDataContractCreateAndUpdate() throws {
        for (name, kindName) in [
            ("data_contract_create", "DataContractCreate"),
            ("data_contract_update", "DataContractUpdate"),
        ] {
            let parsed = try wallet.parseStateTransition(try fixture(name))
            XCTAssertEqual(parsed.kindName, kindName)
            XCTAssertEqual(parsed.ownerId, owner)

            let contract: ManagedPlatformWallet.ParsedDataContractTransition
            switch parsed.kind {
            case .dataContractCreate(let c) where name == "data_contract_create": contract = c
            case .dataContractUpdate(let c) where name == "data_contract_update": contract = c
            default: return XCTFail("unexpected kind \(parsed.kind) for \(name)")
            }
            XCTAssertEqual(contract.ownerId, owner)
            XCTAssertEqual(contract.contractId.count, 32)
            // The rs-dpp fixture contract's document types, as the contract
            // orders them.
            XCTAssertEqual(
                contract.documentTypeNames,
                [
                    "indexedDocument", "niceDocument", "noTimeDocument",
                    "optionalUniqueIndexedDocument", "prettyDocument", "uniqueDates",
                    "withByteArrays",
                ])
        }
    }

    func testRejectsEmptyAndMalformedBytes() {
        XCTAssertThrowsError(try wallet.parseStateTransition(Data()))
        XCTAssertThrowsError(try wallet.parseStateTransition(Data([0xDE, 0xAD, 0xBE, 0xEF]))) { error in
            guard case PlatformWalletError.deserialization = error else {
                return XCTFail("expected deserialization, got \(error)")
            }
        }
    }

    /// The Swift projection of a hand-built C struct: an unknown kind tag
    /// is an error, not silently `.other`, so a Rust-side addition that
    /// the Swift switch does not know about cannot be mis-described.
    func testUnknownKindTagIsAnError() {
        var ffi = ParsedStateTransitionFFI()
        ffi.kind = 77
        XCTAssertThrowsError(try ManagedPlatformWallet.makeParsedStateTransition(from: ffi))
    }
}

private typealias ParsedCreditTransferTransition = ManagedPlatformWallet.ParsedCreditTransferTransition
