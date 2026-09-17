import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// Coverage for the `ContractBounds` variant surviving a persist ->
/// restart -> restore round trip.
///
/// The FFI carries the bound as a `(kind, id, document_type)` trio:
/// 0 none, 1 SingleContract, 2 SingleContractDocumentType, 3
/// ContractGroup. Kinds 1 and 3 look identical in the two columns the
/// SwiftData row used to have (an id, no document-type name), so the
/// variant used to be INFERRED on the way back out and a group-bound
/// AUTHENTICATION key restored unbounded, quietly changing its
/// authorization metadata. `PersistentPublicKey.contractBoundsKind`
/// records the kind instead; a row written before that column reads
/// `nil` and keeps the old inference, which is what these tests pin.
@MainActor
final class IdentityKeyContractBoundsTests: XCTestCase {

    private let walletId = Data(repeating: 0x07, count: 32)
    private let identityId = Data(repeating: 0x35, count: 32)
    private let boundId = Data(repeating: 0x6B, count: 32)
    private let publicKeyData = Data(repeating: 0x02, count: 33)

    private func makeHandler() throws -> (PlatformWalletPersistenceHandler, ModelContainer) {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(
            modelContainer: container, network: .testnet)
        return (handler, container)
    }

    /// A wallet the restore path will actually emit: it needs one
    /// account carrying an xpub, which is what Rust rebuilds the
    /// watch-only wallet from.
    private func seedWallet(in container: ModelContainer) throws {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "Standard")
        account.accountExtendedPubKeyBytes = Data(repeating: 0x30, count: 78)
        context.insert(account)
        let identity = PersistentIdentity(identityId: identityId, network: .testnet)
        identity.wallet = wallet
        context.insert(identity)
        try context.save()
    }

    /// Attach one key row to the seeded identity, writing the columns
    /// directly so a legacy shape (no stored kind) can be modelled.
    private func insertKeyRow(
        in container: ModelContainer,
        keyId: Int32,
        boundIds: [Data]?,
        documentTypeName: String?,
        kind: Int?
    ) throws {
        let context = ModelContext(container)
        let identity = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentIdentity>()).first)
        let row = PersistentPublicKey(
            keyId: keyId,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKeyData,
            contractBounds: boundIds,
            contractBoundsDocumentTypeName: documentTypeName,
            contractBoundsKind: kind,
            identityId: identityId.toBase58String())
        context.insert(row)
        row.identity = identity
        identity.addPublicKey(row)
        try context.save()
    }

    /// One `IdentityKeyEntrySnapshot`, the shape the Rust persist
    /// callback hands over.
    private func snapshot(
        keyId: UInt32,
        bounds: ManagedPlatformWallet.ContractBounds?
    ) -> PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot {
        .init(
            identityId: identityId,
            keyId: keyId,
            purpose: KeyPurpose.authentication.rawValue,
            securityLevel: SecurityLevel.high.rawValue,
            keyType: KeyType.ecdsaSecp256k1.rawValue,
            readOnly: false,
            disabledAt: nil,
            publicKeyData: publicKeyData,
            publicKeyHash: Data(repeating: 0x09, count: 20),
            walletId: nil,
            derivationIndices: nil,
            contractBounds: bounds)
    }

    /// Run one persist round the way the Rust store() round does:
    /// `persistIdentityKeys` stages rows and leaves the save to
    /// `endChangeset`, so an unbracketed call would never reach the
    /// store.
    private func persist(
        _ handler: PlatformWalletPersistenceHandler,
        _ upserts: [PlatformWalletPersistenceHandler.IdentityKeyEntrySnapshot]
    ) {
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentityKeys(walletId: walletId, upserts: upserts, removed: [])
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
    }

    /// The restored `(kind, id, documentType)` trio for every key the
    /// load path emits, read out of the real FFI buffer.
    private func restoredBounds(
        _ handler: PlatformWalletPersistenceHandler
    ) throws -> [(kind: UInt8, id: Data, documentType: String?)] {
        let loaded = handler.loadWalletList()
        XCTAssertFalse(loaded.errored, "the load must not fail")
        let entries = try XCTUnwrap(loaded.entries, "the seeded wallet must be restorable")
        defer { handler.loadWalletListFree(entries: UnsafeRawPointer(entries)) }
        XCTAssertEqual(loaded.count, 1)

        var out: [(kind: UInt8, id: Data, documentType: String?)] = []
        let identities = try XCTUnwrap(entries[0].identities)
        for i in 0..<Int(entries[0].identities_count) {
            let identity = identities[i]
            guard let keys = identity.keys else { continue }
            for k in 0..<Int(identity.keys_count) {
                var idTuple = keys[k].contract_bounds_id
                out.append((
                    kind: keys[k].contract_bounds_kind,
                    id: Swift.withUnsafeBytes(of: &idTuple) { Data($0) },
                    documentType: keys[k].contract_bounds_document_type.map {
                        String(cString: $0)
                    }
                ))
            }
        }
        return out
    }

    // MARK: - Persist then restore

    /// The defect this change fixes: a group-bound key persisted from
    /// Rust comes back as kind 3 with the same group id and no
    /// document type, instead of restoring unbounded.
    func testGroupBoundKeyRoundTripsAsKindThree() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)

        persist(handler, [snapshot(keyId: 1, bounds: .contractGroup(id: boundId))])

        let context = ModelContext(container)
        let row = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentPublicKey>()).first)
        XCTAssertEqual(row.contractBoundsKind, 3, "the group bound is stored as kind 3")
        XCTAssertEqual(row.contractBounds?.first, boundId)
        XCTAssertNil(row.contractBoundsDocumentTypeName, "a group bound has no document type")

        let restored = try restoredBounds(handler)
        XCTAssertEqual(restored.map(\.kind), [3])
        XCTAssertEqual(restored.first?.id, boundId)
        XCTAssertNil(restored.first?.documentType)
    }

    /// The two variants that already round-tripped keep doing so, and
    /// their stored kinds match what the FFI handed over.
    func testSingleContractAndDocumentTypeBoundsStillRoundTrip() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)

        persist(handler, [
            snapshot(keyId: 1, bounds: .singleContract(id: boundId)),
            snapshot(
                keyId: 2,
                bounds: .singleContractDocumentType(
                    id: boundId, documentTypeName: "contactRequest")),
            snapshot(keyId: 3, bounds: nil)
        ])

        let context = ModelContext(container)
        let rows = try context.fetch(
            FetchDescriptor<PersistentPublicKey>(sortBy: [SortDescriptor(\.keyId)]))
        XCTAssertEqual(rows.map(\.contractBoundsKind), [1, 2, 0])

        let restored = try restoredBounds(handler)
        XCTAssertEqual(restored.map(\.kind), [1, 2, 0])
        XCTAssertEqual(restored[1].documentType, "contactRequest")
        XCTAssertNil(restored[0].documentType)
        XCTAssertEqual(restored[2].id, Data(repeating: 0, count: 32), "kind 0 zeroes the id")
    }

    /// A row written before the kind column keeps the inference it
    /// restored with: a document-type name means kind 2, a bare id
    /// means kind 1, neither means kind 0.
    func testLegacyRowsWithoutAStoredKindKeepTheOldInference() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try insertKeyRow(
            in: container, keyId: 1, boundIds: [boundId], documentTypeName: nil, kind: nil)
        try insertKeyRow(
            in: container, keyId: 2, boundIds: [boundId],
            documentTypeName: "contactRequest", kind: nil)
        try insertKeyRow(
            in: container, keyId: 3, boundIds: nil, documentTypeName: nil, kind: nil)

        let context = ModelContext(container)
        let rows = try context.fetch(
            FetchDescriptor<PersistentPublicKey>(sortBy: [SortDescriptor(\.keyId)]))
        XCTAssertEqual(rows.map(\.contractBoundsKind), [nil, nil, nil])
        XCTAssertEqual(rows.map(\.effectiveContractBoundsKind), [1, 2, 0])

        let restored = try restoredBounds(handler)
        XCTAssertEqual(restored.map(\.kind), [1, 2, 0])
        XCTAssertEqual(restored[1].documentType, "contactRequest")
    }

    /// Rows the restore path must not take at face value: a kind this
    /// build does not know, a kind 2 whose document type went missing,
    /// and a bound id that is not 32 bytes.
    func testUnusableRowsRestoreAsUnboundedOrDemote() throws {
        let (handler, container) = try makeHandler()
        try seedWallet(in: container)
        try insertKeyRow(
            in: container, keyId: 1, boundIds: [boundId], documentTypeName: nil, kind: 4)
        try insertKeyRow(
            in: container, keyId: 2, boundIds: [boundId], documentTypeName: nil, kind: 2)
        try insertKeyRow(
            in: container, keyId: 3, boundIds: [Data(repeating: 0x6B, count: 31)],
            documentTypeName: nil, kind: 3)

        let restored = try restoredBounds(handler)
        XCTAssertEqual(
            restored.map(\.kind), [0, 1, 0],
            "an unknown kind and a short id restore unbounded; a kind 2 with no document "
                + "type demotes to kind 1, the same demotion Rust performs")
        XCTAssertNil(restored[1].documentType)
    }

    // MARK: - Model projections

    /// The stored kind wins over the inference, which is the whole
    /// point of the column: kind 3 and kind 1 are indistinguishable
    /// from the id and document-type columns alone.
    func testEffectiveKindPrefersTheStoredValue() throws {
        let row = PersistentPublicKey(
            keyId: 1,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKeyData,
            contractBounds: [boundId],
            contractBoundsKind: 3,
            identityId: identityId.toBase58String())
        XCTAssertEqual(row.effectiveContractBoundsKind, 3)

        // Rewriting the ids through the setter resets the kind with
        // them, so a stale 3 cannot outlive the id it described.
        row.contractBounds = [Data(repeating: 0x11, count: 32)]
        XCTAssertEqual(row.contractBoundsKind, 1)
        row.contractBounds = nil
        XCTAssertEqual(row.contractBoundsKind, 0)
    }

    /// The DPP-layer `ContractBounds` has no group variant and a group
    /// id is not a contract id, so a kind 3 row reports no bounds
    /// there rather than claiming a whole-contract bound.
    func testGroupBoundRowProjectsNoDPPContractBounds() throws {
        let row = PersistentPublicKey(
            keyId: 1,
            purpose: .authentication,
            securityLevel: .high,
            keyType: .ecdsaSecp256k1,
            publicKeyData: publicKeyData,
            contractBounds: [boundId],
            contractBoundsKind: 3,
            identityId: identityId.toBase58String())
        XCTAssertNil(try XCTUnwrap(row.toIdentityPublicKey()).contractBounds)

        row.contractBoundsKind = 1
        let projected = try XCTUnwrap(row.toIdentityPublicKey()).contractBounds
        XCTAssertEqual(projected, ContractBounds.singleContract(id: boundId))
    }

    // MARK: - FFI marshalling

    /// The outbound half: pinning a group bound emits kind 3, the
    /// group id, and a null document-type pointer.
    func testPinnedGroupBoundEmitsKindThreeWithNoDocumentType() throws {
        let pubkey = ManagedPlatformWallet.IdentityPubkey(
            keyId: 4,
            keyType: .ecdsaSecp256k1,
            purpose: .authentication,
            securityLevel: .high,
            pubkeyBytes: publicKeyData,
            contractBounds: .contractGroup(id: boundId))

        let observed: (kind: UInt8, id: Data, hasDocumentType: Bool) =
            ManagedPlatformWallet.withPubkeyFFIArray(
                [pubkey], buffers: [publicKeyData]
            ) { rows, count in
                guard let rows = rows, count == 1 else {
                    return (kind: UInt8.max, id: Data(), hasDocumentType: false)
                }
                let id = rows[0].contract_bounds_id.map { Data(bytes: $0, count: 32) } ?? Data()
                return (
                    kind: rows[0].contract_bounds_kind,
                    id: id,
                    hasDocumentType: rows[0].contract_bounds_document_type != nil
                )
            }

        XCTAssertEqual(observed.kind, 3)
        XCTAssertEqual(observed.id, boundId)
        XCTAssertFalse(observed.hasDocumentType)
    }

    /// The inbound half: an `IdentityUpdateTransition` key row tagged
    /// kind 3 parses back into `.contractGroup`, and a kind this build
    /// does not know is still rejected rather than guessed at.
    func testParsedContractBoundsDecodesKindThreeAndRejectsUnknownKinds() throws {
        func entry(kind: UInt8) -> ParsedIdentityUpdatePublicKeyFFI {
            var entry = ParsedIdentityUpdatePublicKeyFFI()
            entry.contract_bounds_kind = kind
            withUnsafeMutableBytes(of: &entry.contract_bounds_id) { dst in
                boundId.copyBytes(to: dst.bindMemory(to: UInt8.self).baseAddress!, count: 32)
            }
            entry.contract_bounds_document_type = nil
            return entry
        }

        let group = try ManagedPlatformWallet.parsedContractBounds(
            from: entry(kind: 3), index: 0)
        XCTAssertEqual(group, ManagedPlatformWallet.ContractBounds.contractGroup(id: boundId))

        let contract = try ManagedPlatformWallet.parsedContractBounds(
            from: entry(kind: 1), index: 0)
        XCTAssertEqual(contract, ManagedPlatformWallet.ContractBounds.singleContract(id: boundId))

        XCTAssertNil(
            try ManagedPlatformWallet.parsedContractBounds(from: entry(kind: 0), index: 0))
        XCTAssertThrowsError(
            try ManagedPlatformWallet.parsedContractBounds(from: entry(kind: 4), index: 0),
            "an unknown kind must not decode as some neighbouring variant")
        XCTAssertThrowsError(
            try ManagedPlatformWallet.parsedContractBounds(from: entry(kind: 2), index: 0),
            "kind 2 still needs a document type")
    }
}
