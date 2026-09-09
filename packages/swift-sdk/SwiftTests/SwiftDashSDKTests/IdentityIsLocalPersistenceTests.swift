import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

// MARK: - `isLocal` = mine-or-tracked, and wallet-linkage hygiene
//
// Owner-decided semantics: `isLocal` is `true` for every identity
// that is YOURS or deliberately tracked on this device — wallet-derived
// identities always, manual adds (LoadIdentityView) always — and
// `false` only for incidental rows (observed foreign identities
// materialized by sync). Promote-only: no sync path ever writes
// `false` over a `true`.
//
// Historically the persister wrote a constant `false` and nothing
// promoted, so the wallet's own identity (correct `wallet`
// relationship, index 0) showed as not-local on a mainnet device,
// hiding the identity-key refresh affordances in dashwallet-ios.
// These tests pin the promotion, the promote-only discipline, the
// startup heal, and the wallet-relationship hygiene fixed alongside
// (observed entries must not inherit the changeset's scope wallet).

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

    // MARK: Promotion

    /// A wallet-owned entry links the wallet relationship AND
    /// promotes `isLocal` — things from the wallet are always local
    /// (the mainnet field bug was exactly this row showing `false`).
    func testWalletOwnedEntryIsLocal() throws {
        try insertWalletRow()
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: walletId)
        ])

        let row = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(row.wallet?.walletId, walletId)
        XCTAssertTrue(row.isLocal, "wallet-derived identities are always local")
    }

    /// A wallet-derived entry without its own `wallet_id` but WITH an
    /// identity index (create-flow corner case) falls back to the
    /// changeset's scope wallet and promotes.
    func testIndexOnlyEntryFallsBackToScopeWalletAndPromotes() throws {
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
    /// shape — is an incidental row: not linked to the scope wallet
    /// (the old unconditional fallback mislinked these) and not
    /// local.
    func testObservedEntryStaysIncidental() throws {
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

    // MARK: Promote-only discipline

    /// A manually-added row (`isLocal == true`, no wallet) keeps its
    /// mark when sync later flows over it as an observed entry — and
    /// a scope-wallet mislink is cleared WITHOUT demoting.
    func testSyncNeverDemotesManualAdds() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let wallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        // Manual add, later mislinked by the old fallback.
        let manual = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        manual.wallet = wallet
        context.insert(manual)
        try context.save()

        applyIdentities([
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertNil(row.wallet, "the fabricated scope-wallet link is cleared")
        XCTAssertTrue(row.isLocal, "sync must never erase a manual mark")
    }

    // MARK: Wallet-relationship hygiene

    /// An entry declaring an owner whose wallet row misses the
    /// network-scoped fetch keeps a MATCHING existing link…
    func testDeclaredOwnerFetchMissKeepsMatchingLink() throws {
        let otherWalletId = Data(repeating: 0xCC, count: 32)
        let context = ModelContext(container)
        let mainnetWallet = PersistentWallet(walletId: otherWalletId, network: .mainnet)
        context.insert(mainnetWallet)
        let row = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: true,
            network: .testnet
        )
        row.wallet = mainnetWallet
        context.insert(row)
        try context.save()

        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: otherWalletId)
        ])

        let fetched = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertEqual(fetched.wallet?.walletId, otherWalletId)
    }

    /// …while a link CONTRADICTING the declared owner is cleared
    /// (the declaration is Rust's current truth) — without demoting.
    func testDeclaredOwnerMismatchClearsContradictingLink() throws {
        try insertWalletRow()
        let context = ModelContext(container)
        let scopeWallet = try XCTUnwrap(
            try context.fetch(FetchDescriptor<PersistentWallet>()).first
        )
        let row = PersistentIdentity(
            identityId: ownIdentityId,
            isLocal: true,
            network: .testnet
        )
        row.wallet = scopeWallet
        context.insert(row)
        try context.save()

        let unresolvedOwnerId = Data(repeating: 0xDD, count: 32)
        applyIdentities([
            makeEntry(identityId: ownIdentityId, identityIndex: 0, walletId: unresolvedOwnerId)
        ])

        let fetched = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertNil(fetched.wallet)
        XCTAssertTrue(fetched.isLocal, "clearing a link never demotes")
    }

    /// Wallet B's valid relationship survives wallet A's manager
    /// emitting the identity as observed (out-of-wallet is relative
    /// to the EMITTING manager; the row is globally keyed).
    func testObservedEntryPreservesAnotherWalletsLinkage() throws {
        try insertWalletRow()
        let otherWalletId = Data(repeating: 0xBB, count: 32)
        let context = ModelContext(container)
        let otherWallet = PersistentWallet(walletId: otherWalletId, network: .testnet)
        context.insert(otherWallet)
        let ownedByOther = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        ownedByOther.wallet = otherWallet
        context.insert(ownedByOther)
        try context.save()

        applyIdentities([
            makeEntry(identityId: observedIdentityId, identityIndex: nil, walletId: nil)
        ])

        let row = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertEqual(row.wallet?.walletId, otherWalletId)
        XCTAssertTrue(row.isLocal)
    }

    // MARK: Startup heal

    /// `loadWalletList()` promotes wallet-linked rows still carrying
    /// the constant-`false` of the pre-fix persister; unlinked rows
    /// (manual adds) are untouched in both directions.
    func testLoadWalletListHealsWalletLinkedRows() throws {
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
        let manualAdd = PersistentIdentity(
            identityId: observedIdentityId,
            isLocal: true,
            network: .testnet
        )
        context.insert(manualAdd)
        try context.save()

        // The fixture wallet has no restorable accounts, so the
        // returned list is empty — the heal runs regardless.
        let result = handler.loadWalletList()
        XCTAssertNil(result.entries)
        XCTAssertFalse(result.errored)

        let healed = try XCTUnwrap(try fetchIdentity(ownIdentityId))
        XCTAssertTrue(healed.isLocal, "wallet-linked rows heal to local")

        let manual = try XCTUnwrap(try fetchIdentity(observedIdentityId))
        XCTAssertTrue(manual.isLocal, "manual adds are untouched by the heal")
    }
}
