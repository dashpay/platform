import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - `PersistentIdentity.isLocal` promotion invariants
//
// `isLocal` = the user can act as this identity from this device.
// Wallet linkage implies local (`wallet != nil` ⟹ `isLocal`), but not
// the converse — an identity can be local via imported key material
// (masternode voting/owner/payout keys, pasted user keys) with no
// wallet row. Writers therefore PROMOTE and never demote.
//
// Historically the persister wrote a constant `false`, so a wallet's
// own identity (correct `wallet` relationship, identityIndex 0)
// persisted as `isLocal == false` — observed on a mainnet device
// 2026-08-12, where it hid the identity-key refresh affordances in
// dashwallet-ios. These tests pin the promotion on the upsert path,
// the gating of the scope-wallet fallback, the preservation of
// walletless-local rows, and the load-time heal.

final class IdentityIsLocalPersistenceTests: XCTestCase {

    private var container: ModelContainer!
    private var handler: PlatformWalletPersistenceHandler!

    private let walletId = Data(repeating: 0xAA, count: 32)
    private let ownIdentityId = Data(repeating: 0x01, count: 32)
    private let observedIdentityId = Data(repeating: 0x02, count: 32)

    override func setUpWithError() throws {
        try super.setUpWithError()
        container = try DashModelContainer.createInMemory()
        handler = PlatformWalletPersistenceHandler(
            modelContainer: container,
            network: .testnet
        )
    }

    override func tearDown() {
        handler = nil
        container = nil
        super.tearDown()
    }

    // MARK: Fixtures

    private func insertWalletRow() throws {
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
    }

    private func makeEntry(
        identityId: Data,
        identityIndex: UInt32?,
        walletId: Data?
    ) -> PlatformWalletPersistenceHandler.IdentityEntrySnapshot {
        .init(
            identityId: identityId,
            balance: 100,
            revision: 1,
            identityIndex: identityIndex,
            label: nil,
            status: 0,
            walletId: walletId,
            dpnsNames: [],
            dashpayProfile: nil,
            contactProfiles: []
        )
    }

    /// Apply one identity persister round the way the FFI does —
    /// bracketed by `beginChangeset` / `endChangeset(success: true)`.
    private func applyIdentities(
        _ upserts: [PlatformWalletPersistenceHandler.IdentityEntrySnapshot]
    ) {
        handler.beginChangeset(walletId: walletId)
        handler.persistIdentities(
            walletId: walletId,
            upserts: upserts,
            removed: []
        )
        handler.endChangeset(walletId: walletId, success: true)
    }

    private func fetchIdentity(_ identityId: Data) throws -> PersistentIdentity? {
        let context = ModelContext(container)
        return try context.fetch(
            FetchDescriptor<PersistentIdentity>(
                predicate: #Predicate { $0.identityId == identityId }
            )
        ).first
    }

    // MARK: Upsert derivation

    /// A wallet-owned entry (Rust stamps `wallet_id` on every
    /// `add_identity` / load / discovery emission) links the wallet
    /// relationship AND persists `isLocal == true`.
    func testWalletOwnedEntryPersistsIsLocalTrue() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId)
        XCTAssertTrue(
            row.isLocal,
            "the wallet's own identity must persist isLocal == true (2026-08-12 field bug)"
        )
    }

    /// A wallet-derived entry that arrives without its own
    /// `wallet_id` but WITH an identity index (create-flow corner
    /// case) falls back to the changeset's scope wallet.
    func testIndexOnlyEntryFallsBackToScopeWallet() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId)
        XCTAssertTrue(row.isLocal)
    }

    /// An out-of-wallet (observed) entry — `identity_index == nil`
    /// and `wallet_id == nil`, the `add_out_of_wallet_identity`
    /// shape — must NOT be linked to the scope wallet by the
    /// fallback, and stays `isLocal == false`.
    func testObservedEntryDoesNotLinkScopeWallet() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertNil(
            row.wallet,
            "observed identities must not inherit the changeset's scope wallet"
        )
        XCTAssertFalse(row.isLocal)
    }

    /// Re-emitting an entry heals a stale row in place: a
    /// wallet-linked row persisted with `isLocal == false` (written
    /// before the flag tracked linkage) flips to `true`, and a row
    /// mislinked by the old unconditional fallback is unlinked when
    /// its entry re-arrives as out-of-wallet. The unlink must NOT
    /// demote `isLocal` — the flag may be `true` because the user
    /// imported key material (masternode voting keys, pasted user
    /// keys), which the persister cannot see.
    func testUpsertPromotesStaleRowsAndPreservesWalletlessLocal() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let staleOwn = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: false,
            network: .testnet
        )
        staleOwn.wallet = wallet
        context.insert(staleOwn)
        // Mislinked by the old fallback AND marked local (as an
        // imported-keys identity would be).
        let mislinkedLocal = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        mislinkedLocal.wallet = wallet
        context.insert(mislinkedLocal)
        try context.save()

        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId),
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil),
        ])

        let ownRow = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertTrue(ownRow.isLocal)
        XCTAssertEqual(ownRow.wallet?.walletId, walletId)

        let observedRow = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertNil(observedRow.wallet, "the wallet mislink is cleared")
        XCTAssertTrue(
            observedRow.isLocal,
            "losing the wallet link must not demote isLocal — imported-key identities are local without a wallet"
        )
    }

    // MARK: Load-time heal

    /// `loadWalletList()` promotes wallet-linked stale-`false` rows
    /// to `isLocal == true`. UPWARD ONLY: a walletless row carrying
    /// `true` (imported-key identity — masternode voting keys, pasted
    /// user keys) is left alone.
    func testLoadWalletListPromotesButNeverDemotesIsLocal() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let staleOwn = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: false,
            network: .testnet
        )
        staleOwn.wallet = wallet
        context.insert(staleOwn)
        let walletlessLocal = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        context.insert(walletlessLocal)
        try context.save()

        // The fixture wallet has no restorable accounts, so the
        // returned list is empty — the heal runs regardless.
        let result = handler.loadWalletList()
        XCTAssertNil(result.entries)
        XCTAssertFalse(result.errored)

        let ownRow = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertTrue(ownRow.isLocal, "wallet-linked row heals to isLocal == true")

        let walletlessRow = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertTrue(
            walletlessRow.isLocal,
            "walletless local row (imported keys) must survive the heal"
        )
    }
}
