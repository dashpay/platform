import XCTest
import SwiftData
import DashSDKFFI
@testable import SwiftDashSDK

/// The private contact request scan state (read positions and private
/// contacts' shielded addresses) round-trips through SwiftData and the
/// persistence extension's C shims.
final class PrivateContactScanStatePersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 42, count: 32)
    private let owner = Data(repeating: 7, count: 32)
    private let contact = Data(repeating: 5, count: 32)
    private let otherContact = Data(repeating: 6, count: 32)

    private typealias Position = PlatformWalletPersistenceHandler.PrivateContactRequestsPositionRow
    private typealias Address = PlatformWalletPersistenceHandler.PrivateShieldedAddressRow
    private typealias AddressKey = PlatformWalletPersistenceHandler.PrivateShieldedAddressKey

    private func makeHandler() throws -> (ModelContainer, PlatformWalletPersistenceHandler) {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        return (container, PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet))
    }

    private func round(
        _ handler: PlatformWalletPersistenceHandler,
        positions: [Position] = [],
        addresses: [Address] = [],
        removed: [AddressKey] = [],
        cleared: [Data] = [],
        success: Bool = true
    ) throws {
        handler.beginChangeset(walletId: walletId)
        try handler.persistPrivateContactScanState(
            walletId: walletId, positions: positions, addresses: addresses,
            removedAddresses: removed, clearedIdentities: cleared)
        XCTAssertEqual(handler.endChangeset(walletId: walletId, success: success), success)
    }

    func testShouldStoreReplaceAndRemoveTheScanStateWithItsRound() throws {
        let (_, handler) = try makeHandler()
        let fingerprint = Data(repeating: 3, count: 32)
        try round(
            handler,
            positions: [.init(identityId: owner, position: 12, keyFingerprint: fingerprint)],
            addresses: [
                .init(ownerId: owner, contactId: contact, address: Data(repeating: 9, count: 43)),
                .init(ownerId: owner, contactId: otherContact, address: Data(repeating: 8, count: 43)),
            ])
        var stored = try handler.loadPrivateContactScanState(walletId: walletId)
        XCTAssertEqual(stored.positions, [.init(identityId: owner, position: 12, keyFingerprint: fingerprint)])
        XCTAssertEqual(Set(stored.addresses.map(\.contactId)), [contact, otherContact])

        // A rewind writes the lower position; a tombstone removes one address.
        try round(
            handler,
            positions: [.init(identityId: owner, position: 0, keyFingerprint: fingerprint)],
            removed: [.init(ownerId: owner, contactId: otherContact)])
        stored = try handler.loadPrivateContactScanState(walletId: walletId)
        XCTAssertEqual(stored.positions.map(\.position), [0])
        XCTAssertEqual(stored.addresses, [
            .init(ownerId: owner, contactId: contact, address: Data(repeating: 9, count: 43)),
        ])

        // A rolled-back round leaves the stored state as it was.
        try round(
            handler,
            positions: [.init(identityId: owner, position: 40, keyFingerprint: fingerprint)],
            success: false)
        XCTAssertEqual(try handler.loadPrivateContactScanState(walletId: walletId).positions.map(\.position), [0])

        // The unresolved-payment marker is stored with the position and
        // cleared by a later round.
        try round(handler, positions: [.init(
            identityId: owner, position: 0, keyFingerprint: fingerprint, privatePaymentsUnresolved: true)])
        XCTAssertEqual(
            try handler.loadPrivateContactScanState(walletId: walletId).positions.map(\.privatePaymentsUnresolved),
            [true])
        try round(handler, positions: [.init(identityId: owner, position: 0, keyFingerprint: fingerprint)])
        XCTAssertEqual(
            try handler.loadPrivateContactScanState(walletId: walletId).positions.map(\.privatePaymentsUnresolved),
            [false])

        // A full-width position survives the signed SQLite column.
        try round(handler, positions: [.init(identityId: owner, position: .max, keyFingerprint: fingerprint)])
        XCTAssertEqual(try handler.loadPrivateContactScanState(walletId: walletId).positions.map(\.position), [.max])
    }

    func testShouldClearTheScanStateOfARemovedIdentity() throws {
        let (_, handler) = try makeHandler()
        let survivor = Data(repeating: 1, count: 32)
        try round(
            handler,
            positions: [
                .init(identityId: owner, position: 12, keyFingerprint: Data(repeating: 3, count: 32)),
                .init(identityId: survivor, position: 4, keyFingerprint: Data(repeating: 2, count: 32)),
            ],
            addresses: [
                .init(ownerId: owner, contactId: contact, address: Data(repeating: 9, count: 43)),
                .init(ownerId: survivor, contactId: contact, address: Data(repeating: 1, count: 43)),
            ])
        try round(handler, cleared: [owner])
        let stored = try handler.loadPrivateContactScanState(walletId: walletId)
        XCTAssertEqual(stored.positions.map(\.identityId), [survivor])
        XCTAssertEqual(stored.addresses.map(\.ownerId), [survivor])
    }

    func testShouldScopeTheScanStateByWalletAndNetworkAndPurgeItWithTheWallet() throws {
        let (container, handler) = try makeHandler()
        try round(
            handler,
            positions: [.init(identityId: owner, position: 12, keyFingerprint: Data(repeating: 3, count: 32))],
            addresses: [.init(ownerId: owner, contactId: contact, address: Data(repeating: 9, count: 43))])
        XCTAssertTrue(try handler.loadPrivateContactScanState(walletId: Data(repeating: 43, count: 32))
            .positions.isEmpty)
        let mainnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .mainnet)
        XCTAssertTrue(try mainnet.loadPrivateContactScanState(walletId: walletId).positions.isEmpty)

        try handler.deleteWalletData(walletId: walletId)
        let context = ModelContext(container)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentPrivateContactRequestsPosition>()), 0)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentPrivateShieldedAddress>()), 0)
    }

    /// A position record marked unresolved survives its wallet's purge and
    /// is listed for the load to hand back as retained; an unmarked record
    /// and every address go with the wallet; a cleared record stored later
    /// for the same wallet and identity replaces it, so the next purge
    /// removes it.
    func testShouldKeepAMarkedRecordThroughItsWalletsPurgeUntilAClearedRecordReplacesIt() throws {
        let (container, handler) = try makeHandler()
        let unmarked = Data(repeating: 1, count: 32)
        let fingerprint = Data(repeating: 3, count: 32)
        try round(
            handler,
            positions: [
                .init(identityId: owner, position: 12, keyFingerprint: fingerprint,
                      privatePaymentsUnresolved: true),
                .init(identityId: unmarked, position: 4, keyFingerprint: fingerprint),
            ],
            addresses: [.init(ownerId: owner, contactId: contact, address: Data(repeating: 9, count: 43))])
        XCTAssertEqual(try handler.loadPrivateContactScanStateWalletIds(), [walletId])

        try handler.deleteWalletData(walletId: walletId)
        let context = ModelContext(container)
        XCTAssertEqual(try context.fetchCount(FetchDescriptor<PersistentPrivateShieldedAddress>()), 0)
        var kept = try handler.loadPrivateContactScanState(walletId: walletId)
        XCTAssertEqual(kept.positions, [.init(
            identityId: owner, position: 12, keyFingerprint: fingerprint, privatePaymentsUnresolved: true)],
            "only the marked record outlives the wallet")
        XCTAssertTrue(kept.addresses.isEmpty)
        XCTAssertEqual(try handler.loadPrivateContactScanStateWalletIds(), [walletId],
                       "the removed wallet is still listed for its kept record")

        // The wallet registered again stores the record cleared.
        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        try context.save()
        try round(handler, positions: [.init(identityId: owner, position: 0, keyFingerprint: fingerprint)])
        kept = try handler.loadPrivateContactScanState(walletId: walletId)
        XCTAssertEqual(kept.positions.map(\.privatePaymentsUnresolved), [false])
        try handler.deleteWalletData(walletId: walletId)
        XCTAssertEqual(try ModelContext(container).fetchCount(
            FetchDescriptor<PersistentPrivateContactRequestsPosition>()), 0)
        XCTAssertTrue(try handler.loadPrivateContactScanStateWalletIds().isEmpty)
    }

    /// A handler configured with no network, as `configure(sdkPointer:
    /// modelContainer:network:)` makes by default: a marked record of a
    /// mainnet wallet, stored under mainnet, still reads back after the
    /// wallet's removal, through the wallet list and the per-wallet load and
    /// through their C shims, rather than reading as absent under a guessed
    /// testnet scope. A testnet-scoped handler still does not see it.
    func testShouldHandBackAKeptMainnetRecordToAHandlerWithNoNetwork() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        context.insert(PersistentWallet(walletId: walletId, network: .mainnet))
        try context.save()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container)
        let fingerprint = Data(repeating: 3, count: 32)
        let marked = Position(
            identityId: owner, position: 12, keyFingerprint: fingerprint, privatePaymentsUnresolved: true)
        try round(handler, positions: [marked])
        XCTAssertEqual(
            try ModelContext(container).fetch(FetchDescriptor<PersistentPrivateContactRequestsPosition>())
                .map(\.networkRaw),
            [Network.mainnet.rawValue],
            "stored under the wallet's own network")

        try handler.deleteWalletData(walletId: walletId)
        XCTAssertEqual(try handler.loadPrivateContactScanStateWalletIds(), [walletId])
        XCTAssertEqual(try handler.loadPrivateContactScanState(walletId: walletId).positions, [marked])

        let ext = handler.makePersistenceCallbacksExtension()
        let list = try XCTUnwrap(ext.on_load_private_contact_scan_state_wallets_fn)
        let freeList = try XCTUnwrap(ext.on_load_private_contact_scan_state_wallets_free_fn)
        let load = try XCTUnwrap(ext.on_load_private_contact_scan_state_fn)
        let free = try XCTUnwrap(ext.on_load_private_contact_scan_state_free_fn)
        let shimContext = Unmanaged.passUnretained(handler).toOpaque()
        var ids: UnsafePointer<FFIByteTuple32>?
        var idsCount: UInt = 0
        XCTAssertEqual(list(shimContext, &ids, &idsCount), 0)
        XCTAssertEqual(idsCount, 1)
        freeList(shimContext, ids, idsCount)
        var positions: UnsafePointer<PrivateContactRequestsPositionFFI>?
        var positionsCount: UInt = 0
        var addresses: UnsafePointer<PrivateShieldedAddressFFI>?
        var addressesCount: UInt = 0
        let rc = walletId.withUnsafeBytes { wallet in
            load(shimContext, wallet.bindMemory(to: UInt8.self).baseAddress,
                 &positions, &positionsCount, &addresses, &addressesCount)
        }
        XCTAssertEqual(rc, 0)
        XCTAssertEqual(positionsCount, 1)
        XCTAssertTrue(try XCTUnwrap(positions).pointee.private_payments_unresolved)
        free(shimContext, positions, positionsCount, addresses, addressesCount)

        let testnet = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        XCTAssertTrue(try testnet.loadPrivateContactScanStateWalletIds().isEmpty)
        XCTAssertTrue(try testnet.loadPrivateContactScanState(walletId: walletId).positions.isEmpty)
    }

    /// A handler with no network and no wallet row to take one from refuses
    /// to store scan state rather than file it under a guessed network.
    func testShouldRefuseToStoreScanStateWithNoNetworkToStoreItUnder() throws {
        let container = try DashModelContainer.createInMemory()
        let handler = PlatformWalletPersistenceHandler(modelContainer: container)
        handler.beginChangeset(walletId: walletId)
        XCTAssertThrowsError(try handler.persistPrivateContactScanState(
            walletId: walletId,
            positions: [.init(identityId: owner, position: 1, keyFingerprint: Data(repeating: 3, count: 32))],
            addresses: [], removedAddresses: [], clearedIdentities: []))
        _ = handler.endChangeset(walletId: walletId, success: false)
    }

    /// The wallet-list shim hands Rust the listed ids as 32-byte rows and
    /// its free shim releases them; an empty list crosses as null.
    func testShouldListTheWalletsWithScanStateThroughTheCallbackShims() throws {
        let (_, handler) = try makeHandler()
        let ext = handler.makePersistenceCallbacksExtension()
        let list = try XCTUnwrap(ext.on_load_private_contact_scan_state_wallets_fn)
        let free = try XCTUnwrap(ext.on_load_private_contact_scan_state_wallets_free_fn)
        let context = Unmanaged.passUnretained(handler).toOpaque()

        var ids: UnsafePointer<FFIByteTuple32>?
        var count: UInt = 7
        XCTAssertEqual(list(context, &ids, &count), 0)
        XCTAssertNil(ids)
        XCTAssertEqual(count, 0)

        try round(handler, positions: [.init(
            identityId: owner, position: 1, keyFingerprint: Data(repeating: 3, count: 32),
            privatePaymentsUnresolved: true)])
        try handler.deleteWalletData(walletId: walletId)
        XCTAssertEqual(list(context, &ids, &count), 0)
        XCTAssertEqual(count, 1)
        var first = try XCTUnwrap(ids).pointee
        XCTAssertEqual(Swift.withUnsafeBytes(of: &first) { Data($0) }, walletId)
        free(context, ids, count)
    }

    /// A stored row that cannot cross the ABI fails the load instead of
    /// handing back the rest without it: a valid position beside a 42-byte
    /// address would otherwise resume past the entry that carried it.
    func testShouldFailTheLoadOnAStoredRowThatCannotCrossTheABI() throws {
        let (container, handler) = try makeHandler()
        try round(
            handler,
            positions: [.init(identityId: owner, position: 12, keyFingerprint: Data(repeating: 3, count: 32))])
        let context = ModelContext(container)
        context.insert(PersistentPrivateShieldedAddress(
            networkRaw: Network.testnet.rawValue, walletId: walletId, ownerId: owner,
            contactId: contact, address: Data(repeating: 9, count: 42)))
        try context.save()

        let ext = handler.makePersistenceCallbacksExtension()
        let load = try XCTUnwrap(ext.on_load_private_contact_scan_state_fn)
        let context_ = Unmanaged.passUnretained(handler).toOpaque()
        var positions: UnsafePointer<PrivateContactRequestsPositionFFI>?
        var positionsCount: UInt = 0
        var addresses: UnsafePointer<PrivateShieldedAddressFFI>?
        var addressesCount: UInt = 0
        let rc = walletId.withUnsafeBytes { wallet in
            load(context_, wallet.bindMemory(to: UInt8.self).baseAddress,
                 &positions, &positionsCount, &addresses, &addressesCount)
        }
        XCTAssertNotEqual(rc, 0)
        XCTAssertNil(positions)
        XCTAssertNil(addresses)
        XCTAssertEqual(positionsCount + addressesCount, 0)
    }

    func testShouldRefuseAScanStateWriteOutsideAChangeset() throws {
        let (_, handler) = try makeHandler()
        XCTAssertThrowsError(try handler.persistPrivateContactScanState(
            walletId: walletId,
            positions: [.init(identityId: owner, position: 1, keyFingerprint: Data(repeating: 3, count: 32))],
            addresses: [], removedAddresses: [], clearedIdentities: []))
    }

    /// The C shims Rust calls: a persist through the extension's slot is
    /// read back through its load slot, and the free slot returns the
    /// arrays. The capability is declared alongside them.
    func testShouldDeclareTheCapabilityAndRoundTripThroughTheCallbackShims() throws {
        let (_, handler) = try makeHandler()
        XCTAssertNotEqual(
            handler.makePersistenceCapabilities().bits
                & PlatformWalletPersistenceCapabilities.privateContactRequestScanState, 0)
        // Bit 15: bit 14 is the asset-lock shielded recipient's.
        XCTAssertEqual(PlatformWalletPersistenceCapabilities.privateContactRequestScanState, 1 << 15)
        XCTAssertEqual(
            PlatformWalletPersistenceCapabilities.privateContactRequestScanState,
            UInt64(PLATFORM_WALLET_PERSISTENCE_CAPABILITY_PRIVATE_CONTACT_REQUEST_SCAN_STATE))
        let ext = handler.makePersistenceCallbacksExtension()
        let persist = try XCTUnwrap(ext.on_persist_private_contact_scan_state_fn)
        let load = try XCTUnwrap(ext.on_load_private_contact_scan_state_fn)
        let free = try XCTUnwrap(ext.on_load_private_contact_scan_state_free_fn)
        XCTAssertNotNil(ext.on_load_private_contact_scan_state_wallets_fn)
        XCTAssertNotNil(ext.on_load_private_contact_scan_state_wallets_free_fn)
        let context = Unmanaged.passUnretained(handler).toOpaque()

        var position = PrivateContactRequestsPositionFFI()
        Swift.withUnsafeMutableBytes(of: &position.identity_id) { _ = owner.copyBytes(to: $0) }
        position.position = 77
        position.private_payments_unresolved = true
        Swift.withUnsafeMutableBytes(of: &position.key_fingerprint) {
            _ = Data(repeating: 3, count: 32).copyBytes(to: $0)
        }
        var address = PrivateShieldedAddressFFI()
        Swift.withUnsafeMutableBytes(of: &address.owner_id) { _ = owner.copyBytes(to: $0) }
        Swift.withUnsafeMutableBytes(of: &address.contact_id) { _ = contact.copyBytes(to: $0) }
        Swift.withUnsafeMutableBytes(of: &address.address) { _ = Data(repeating: 9, count: 43).copyBytes(to: $0) }

        handler.beginChangeset(walletId: walletId)
        let rc = walletId.withUnsafeBytes { wallet in
            persist(context, wallet.bindMemory(to: UInt8.self).baseAddress,
                    &position, 1, &address, 1, nil, 0, nil, 0)
        }
        XCTAssertEqual(rc, 0)
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        var positions: UnsafePointer<PrivateContactRequestsPositionFFI>?
        var positionsCount: UInt = 0
        var addresses: UnsafePointer<PrivateShieldedAddressFFI>?
        var addressesCount: UInt = 0
        let loadRc = walletId.withUnsafeBytes { wallet in
            load(context, wallet.bindMemory(to: UInt8.self).baseAddress,
                 &positions, &positionsCount, &addresses, &addressesCount)
        }
        XCTAssertEqual(loadRc, 0)
        XCTAssertEqual(positionsCount, 1)
        XCTAssertEqual(addressesCount, 1)
        let loadedPosition = try XCTUnwrap(positions).pointee
        XCTAssertEqual(loadedPosition.position, 77)
        XCTAssertTrue(loadedPosition.private_payments_unresolved, "the unresolved marker round-trips")
        var loadedOwner = loadedPosition.identity_id
        XCTAssertEqual(Swift.withUnsafeBytes(of: &loadedOwner) { Data($0) }, owner)
        var loadedAddress = try XCTUnwrap(addresses).pointee.address
        XCTAssertEqual(Swift.withUnsafeBytes(of: &loadedAddress) { Data($0) }, Data(repeating: 9, count: 43))
        free(context, positions, positionsCount, addresses, addressesCount)

        // A wallet with nothing stored hands back two empty arrays.
        let empty = Data(repeating: 43, count: 32)
        let emptyRc = empty.withUnsafeBytes { wallet in
            load(context, wallet.bindMemory(to: UInt8.self).baseAddress,
                 &positions, &positionsCount, &addresses, &addressesCount)
        }
        XCTAssertEqual(emptyRc, 0)
        XCTAssertNil(positions)
        XCTAssertNil(addresses)
        XCTAssertEqual(positionsCount + addressesCount, 0)
    }
}
