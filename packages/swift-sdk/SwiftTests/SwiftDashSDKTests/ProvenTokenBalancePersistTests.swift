import XCTest
import SwiftData
@testable import SwiftDashSDK

/// Coverage for `SDK.persistTokenBalances` — the persist half of the
/// MW-02 reworked balance refresh.
///
/// A token transfer / burn's broadcast result already carries the
/// proof-verified post-action balances; the FFI surfaces them as
/// `[Data: UInt64]` and the example app hands them straight to this
/// upsert helper. These tests pin the upsert / relationship-link
/// behavior against an in-memory `ModelContainer` so the rows land
/// exactly like the periodic-sync persister's
/// (`PlatformWalletPersistenceHandler.persistTokenBalances`).
///
/// The static helper takes the canonical token id directly (the
/// `calculateTokenId` derivation is exercised separately and needs the
/// FFI), so these tests are pure SwiftData with no SDK handle.
@MainActor
final class ProvenTokenBalancePersistTests: XCTestCase {

    // A canonical (32-byte → base58) token id stand-in. The helper
    // treats it opaquely as the `PersistentTokenBalance.tokenId` key.
    private let canonicalTokenId = "TokenIdCanonicalBase58Fixture"

    private let contractId = Data(repeating: 0xAB, count: 32)
    private let position = 0

    // 40-byte PersistentToken.id composite (contractId + bigEndian
    // position), the relationship-link key. Built via the model's own
    // init so the composite formula stays the single source of truth.
    private var tokenRelationshipKey: Data {
        PersistentToken(
            contractId: contractId,
            position: position,
            name: "Fixture",
            baseSupply: "0"
        ).id
    }

    private func makeContainer() throws -> ModelContainer {
        try DashModelContainer.createInMemory()
    }

    private func balanceRow(
        for identityId: Data,
        in context: ModelContext
    ) throws -> PersistentTokenBalance? {
        let tokenId = canonicalTokenId
        let descriptor = FetchDescriptor<PersistentTokenBalance>(
            predicate: #Predicate {
                $0.tokenId == tokenId && $0.identityId == identityId
            }
        )
        return try context.fetch(descriptor).first
    }

    /// Persisting a `[Data: UInt64]` for a sender + recipient creates one
    /// row per identity, each carrying the supplied balance.
    func testPersistsSenderAndRecipientRows() throws {
        let container = try makeContainer()
        let context = ModelContext(container)

        let sender = Data(repeating: 0x01, count: 32)
        let recipient = Data(repeating: 0x02, count: 32)

        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [sender: 70, recipient: 30],
            in: context
        )

        let senderRow = try balanceRow(for: sender, in: context)
        let recipientRow = try balanceRow(for: recipient, in: context)

        XCTAssertEqual(senderRow?.balance, 70)
        XCTAssertEqual(recipientRow?.balance, 30)
        XCTAssertEqual(senderRow?.tokenId, canonicalTokenId)
        XCTAssertEqual(recipientRow?.tokenId, canonicalTokenId)
        // markAsSynced stamped both rows.
        XCTAssertNotNil(senderRow?.lastSyncedAt)
        XCTAssertNotNil(recipientRow?.lastSyncedAt)

        // Exactly two rows total — no duplicates inserted.
        let all = try context.fetch(FetchDescriptor<PersistentTokenBalance>())
        XCTAssertEqual(all.count, 2)
    }

    /// Upserting an existing row updates its balance in place (no
    /// duplicate row) and preserves its `identity` relationship link.
    func testUpsertPreservesIdentityRelationship() throws {
        let container = try makeContainer()
        let context = ModelContext(container)

        let identityId = Data(repeating: 0x03, count: 32)

        // A local identity row exists for this id → first persist links
        // the relationship.
        let identity = PersistentIdentity(identityId: identityId, network: .testnet)
        context.insert(identity)
        try context.save()

        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [identityId: 100],
            in: context
        )

        let firstRow = try XCTUnwrap(try balanceRow(for: identityId, in: context))
        XCTAssertEqual(firstRow.balance, 100)
        XCTAssertNotNil(firstRow.identity, "first persist should link the local identity")
        let linkedIdentityId = firstRow.identity?.identityId

        // Second persist (a later transfer/burn) for the same identity:
        // the existing row is updated, not duplicated, and the identity
        // link survives.
        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [identityId: 55],
            in: context
        )

        let all = try context.fetch(FetchDescriptor<PersistentTokenBalance>())
        XCTAssertEqual(all.count, 1, "upsert must not create a duplicate row")

        let secondRow = try XCTUnwrap(try balanceRow(for: identityId, in: context))
        XCTAssertEqual(secondRow.balance, 55)
        XCTAssertNotNil(secondRow.identity, "identity relationship preserved across upsert")
        XCTAssertEqual(secondRow.identity?.identityId, linkedIdentityId)
    }

    /// A zero balance (a sender that drained to empty) zeroes the row
    /// rather than leaving the prior figure.
    func testZeroBalanceZeroesTheRow() throws {
        let container = try makeContainer()
        let context = ModelContext(container)

        let identityId = Data(repeating: 0x04, count: 32)

        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [identityId: 500],
            in: context
        )
        XCTAssertEqual(try balanceRow(for: identityId, in: context)?.balance, 500)

        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [identityId: 0],
            in: context
        )
        XCTAssertEqual(try balanceRow(for: identityId, in: context)?.balance, 0)
    }

    /// When a local `PersistentToken` row exists (its `id` ==
    /// `tokenRelationshipKey`), the persist links the balance row's
    /// `token` relationship, and the link survives a later upsert.
    func testUpsertLinksAndPreservesTokenRelationship() throws {
        let container = try makeContainer()
        let context = ModelContext(container)

        let identityId = Data(repeating: 0x05, count: 32)

        // A local token row exists for this contract+position → the first
        // persist should stitch in the `token` relationship.
        let token = PersistentToken(
            contractId: contractId,
            position: position,
            name: "Fixture",
            baseSupply: "0"
        )
        context.insert(token)
        try context.save()
        XCTAssertEqual(
            token.id, tokenRelationshipKey,
            "fixture token id must equal the relationship key the helper looks up"
        )

        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [identityId: 42],
            in: context
        )

        let firstRow = try XCTUnwrap(try balanceRow(for: identityId, in: context))
        XCTAssertEqual(firstRow.balance, 42)
        XCTAssertNotNil(firstRow.token, "first persist should link the local token")
        XCTAssertEqual(firstRow.token?.id, tokenRelationshipKey)

        // Second persist (a later transfer/burn): the row is updated, not
        // duplicated, and the token link survives.
        try SDK.persistTokenBalances(
            canonicalTokenId: canonicalTokenId,
            tokenRelationshipKey: tokenRelationshipKey,
            network: .testnet,
            balances: [identityId: 7],
            in: context
        )

        let all = try context.fetch(FetchDescriptor<PersistentTokenBalance>())
        XCTAssertEqual(all.count, 1, "upsert must not create a duplicate row")

        let secondRow = try XCTUnwrap(try balanceRow(for: identityId, in: context))
        XCTAssertEqual(secondRow.balance, 7)
        XCTAssertNotNil(secondRow.token, "token relationship preserved across upsert")
        XCTAssertEqual(secondRow.token?.id, tokenRelationshipKey)
    }
}
