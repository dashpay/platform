import Foundation
import SwiftData
import XCTest
@testable import SwiftDashSDK

/// An outbound DashPay account (`accountType == 13`) registered again under
/// another xpub drops its stored pool, and keeps the digest of the contact
/// request it was built from beside its xpub.
@MainActor
final class OutboundAccountReplacementPersistenceTests: XCTestCase {
    private let walletId = Data(repeating: 0x0A, count: 32)
    private let outboundTypeTag: UInt32 = 13
    private let owner = Data(repeating: 0x01, count: 32)
    private let contact = Data(repeating: 0x02, count: 32)

    private var key: PlatformWalletPersistenceHandler.AccountLookupKey {
        .init(typeTag: outboundTypeTag, index: 0, standardTag: 0, registrationIndex: 0,
              keyClass: 0, userIdentityId: owner, friendIdentityId: contact)
    }

    /// A wallet with the outbound account, two pool rows of the previous
    /// xpub, and a TXO linked to one of them.
    private func seed(_ container: ModelContainer) throws {
        let context = ModelContext(container)
        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet, accountType: outboundTypeTag, accountIndex: 0, accountTypeName: "Contact")
        account.userIdentityId = owner
        account.friendIdentityId = contact
        context.insert(account)
        for index in 0..<2 {
            let row = PersistentCoreAddress(
                address: "yOldXpubAddress\(index)", poolTypeTag: 0, addressIndex: UInt32(index),
                derivationPath: "m/\(index)", isUsed: index == 0)
            row.account = account
            context.insert(row)
        }
        let tx = PersistentTransaction(txid: Data(repeating: 7, count: 32), transactionData: Data())
        context.insert(tx)
        context.insert(PersistentTxo(transaction: tx, vout: 0, amount: 1_000, address: "yOldXpubAddress0"))
        try context.save()
        let txo = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentTxo>()).first)
        txo.coreAddress = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentCoreAddress>(
            predicate: #Predicate { $0.address == "yOldXpubAddress0" })).first)
        try context.save()
    }

    func testResetDropsThePreviousXpubsRowsAndKeepsTheirTxos() throws {
        let container = try DashModelContainer.createInMemory()
        try seed(container)
        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)

        XCTAssertThrowsError(try handler.resetAccountAddressPools(walletId: walletId, keys: [key]),
                             "a reset outside a changeset could not roll back with its round")
        handler.beginChangeset(walletId: walletId)
        try handler.resetAccountAddressPools(walletId: walletId, keys: [key])
        XCTAssertTrue(handler.persistAccountAddresses(walletId: walletId, accountKey: key, entries: [
            .init(address: "yNewXpubAddress0", publicKey: Data(count: 33), keyType: 0, poolTypeTag: 0,
                  addressIndex: 0, isUsed: false, balance: 0, derivationPath: "m/0")
        ]))
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))

        let context = ModelContext(container)
        XCTAssertEqual(try context.fetch(FetchDescriptor<PersistentCoreAddress>()).map(\.address),
                       ["yNewXpubAddress0"])
        let txos = try context.fetch(FetchDescriptor<PersistentTxo>())
        XCTAssertEqual(txos.count, 1, "the reset keeps funds history")
        XCTAssertNil(txos.first?.coreAddress)
    }

    func testDigestSurvivesStoreReopenOnTheAccountRowAndFailsClosed() throws {
        let directory = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: directory) }
        let url = directory.appendingPathComponent("wallet.store")
        let digest = Data(repeating: 0xD1, count: 32)
        try autoreleasepool {
            let container = try DashModelContainer.create(url: url)
            try seed(container)
            let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
            XCTAssertNil(try handler.loadAccountXpubDigest(walletId: walletId, key: key))
            XCTAssertThrowsError(try handler.persistAccountXpubDigest(walletId: walletId, key: key, digest: digest))
            handler.beginChangeset(walletId: walletId)
            XCTAssertThrowsError(try handler.persistAccountXpubDigest(
                walletId: walletId, key: key, digest: Data(repeating: 0xD1, count: 31)))
            try handler.persistAccountXpubDigest(walletId: walletId, key: key, digest: digest)
            XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        }

        let reopened = try DashModelContainer.create(url: url)
        let handler = PlatformWalletPersistenceHandler(modelContainer: reopened, network: .testnet)
        XCTAssertEqual(try handler.loadAccountXpubDigest(walletId: walletId, key: key), digest)

        // An account registered again without a digest clears the previous one.
        handler.beginChangeset(walletId: walletId)
        try handler.persistAccountXpubDigest(walletId: walletId, key: key, digest: nil)
        XCTAssertTrue(handler.endChangeset(walletId: walletId, success: true))
        XCTAssertNil(try handler.loadAccountXpubDigest(walletId: walletId, key: key))
        let missing = PlatformWalletPersistenceHandler.AccountLookupKey(typeTag: outboundTypeTag, index: 0, standardTag: 0, registrationIndex: 0,
                        keyClass: 0, userIdentityId: owner, friendIdentityId: Data(repeating: 9, count: 32))
        XCTAssertThrowsError(try handler.loadAccountXpubDigest(walletId: walletId, key: missing),
                             "a missing account row is a failed read")
    }
}
