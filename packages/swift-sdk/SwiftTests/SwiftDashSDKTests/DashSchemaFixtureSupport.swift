import CoreData
import Foundation
import SQLite3
import SwiftData
import XCTest

@testable import SwiftDashSDK

/// Shared synthetic release evidence; never reads a user's wallet or keychain.
enum DashSchemaFixtureSupport {
    struct Description: Codable, Equatable {
        let schema_version: String
        let model_checksum: String
        let entity_hashes: [String: String]
        let indexes: [String]
    }

    static func version(_ version: Schema.Version) -> String {
        "\(version.major).\(version.minor).\(version.patch)"
    }

    static func describeStore(at url: URL, version: Schema.Version) throws -> Description {
        let metadata = try NSPersistentStoreCoordinator.metadataForPersistentStore(type: .sqlite, at: url)
        let identifiers = try XCTUnwrap(metadata["NSStoreModelVersionIdentifiers"] as? [String])
        guard identifiers == [self.version(version)] else {
            throw NSError(domain: "DashSchemaFixtureVersionMismatch", code: 1)
        }
        let checksum = try XCTUnwrap(metadata["NSStoreModelVersionChecksumKey"] as? String)
        let hashes = try XCTUnwrap(metadata["NSStoreModelVersionHashes"] as? [String: Data])
        return Description(
            schema_version: self.version(version), model_checksum: checksum,
            entity_hashes: hashes.mapValues { data in data.map { String(format: "%02x", $0) }.joined() },
            indexes: try indexes(at: url))
    }

    static func indexes(at url: URL) throws -> [String] {
        var database: OpaquePointer?
        guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READONLY, nil) == SQLITE_OK else {
            sqlite3_close(database)
            throw NSError(domain: "DashSchemaFixture", code: 1)
        }
        defer { sqlite3_close(database) }
        var statement: OpaquePointer?
        let query = "SELECT tbl_name, name, sql FROM sqlite_master WHERE type = 'index'"
        guard sqlite3_prepare_v2(database, query, -1, &statement, nil) == SQLITE_OK else {
            throw NSError(domain: "DashSchemaFixture", code: 2)
        }
        defer { sqlite3_finalize(statement) }
        var result: [String] = []
        var step = sqlite3_step(statement)
        while step == SQLITE_ROW {
            let table = String(cString: sqlite3_column_text(statement, 0))
            let name = String(cString: sqlite3_column_text(statement, 1))
            let sql = sqlite3_column_text(statement, 2).map { String(cString: $0) } ?? "(auto)"
            result.append("\(table) \(name): \(sql)")
            step = sqlite3_step(statement)
        }
        guard step == SQLITE_DONE else { throw NSError(domain: "DashSchemaFixture", code: Int(step)) }
        return result.sorted()
    }

    @MainActor
    static func writeLiveStore(at url: URL) throws {
        guard !FileManager.default.fileExists(atPath: url.path) else {
            throw NSError(domain: "DashSchemaFixtureAlreadyExists", code: 1)
        }
        try autoreleasepool { try populateStore(at: url) }
        var database: OpaquePointer?
        guard sqlite3_open_v2(url.path, &database, SQLITE_OPEN_READWRITE, nil) == SQLITE_OK else {
            sqlite3_close(database)
            throw NSError(domain: "DashSchemaFixture", code: 3)
        }
        defer { sqlite3_close(database) }
        guard sqlite3_wal_checkpoint_v2(database, nil, SQLITE_CHECKPOINT_TRUNCATE, nil, nil) == SQLITE_OK,
            sqlite3_exec(database, "PRAGMA journal_mode=DELETE", nil, nil, nil) == SQLITE_OK
        else { throw NSError(domain: "DashSchemaFixture", code: 4) }
        guard !FileManager.default.fileExists(atPath: url.path + "-wal"),
            !FileManager.default.fileExists(atPath: url.path + "-shm")
        else { throw NSError(domain: "DashSchemaFixtureSidecars", code: 1) }
    }

    @MainActor
    private static func populateStore(at url: URL) throws {
        let container = try DashModelContainer.create(url: url)
        let context = container.mainContext
        let wallet = PersistentWallet(
            walletId: Data(repeating: 0x31, count: 32), network: .testnet, name: "fixture wallet",
            syncedHeight: 120)
        context.insert(wallet)
        let account = PersistentAccount(
            wallet: wallet, accountType: 0, accountIndex: 0, accountTypeName: "standard")
        context.insert(account)
        let address = PersistentCoreAddress(
            address: "yFixtureAddress", poolTypeTag: 0, addressIndex: 0, derivationPath: "m/0")
        address.account = account
        context.insert(address)
        let funding = PersistentTransaction(
            txid: Data(repeating: 0x34, count: 32), transactionData: Data([3, 0]), context: 2,
            blockHeight: 100)
        let spend = PersistentTransaction(
            txid: Data(repeating: 0x32, count: 32), transactionData: Data([3, 0]), context: 2,
            blockHeight: 110)
        context.insert(funding)
        context.insert(spend)
        account.involvedTransactions = [funding, spend]
        let txo = PersistentTxo(
            transaction: funding, vout: 0, amount: 1_000, address: "yFixtureAddress",
            height: 100)
        txo.walletId = Data(repeating: 0x31, count: 32)
        txo.isSpent = true
        txo.spendingTransaction = spend
        txo.coreAddress = address
        txo.account = account
        context.insert(txo)
        context.insert(PersistentPendingInput(
            outpoint: Data(repeating: 0x11, count: 36), inputIndex: 0,
            spendingTxid: Data(repeating: 0x32, count: 32), spendingTransaction: spend,
            walletId: Data(repeating: 0x31, count: 32)))
        let identity = PersistentIdentity(
            identityId: Data(repeating: 0x35, count: 32), balance: 5, network: .testnet)
        identity.wallet = wallet
        context.insert(identity)
        let key = PersistentPublicKey(
            keyId: 3, purpose: .authentication, securityLevel: .high,
            keyType: .ecdsaSecp256k1, publicKeyData: Data(repeating: 0x02, count: 33),
            totalBudget: 100_000, expiresAt: 1_800_000_000_000,
            identityId: identity.identityIdString)
        key.identity = identity
        context.insert(key)
        context.insert(PersistentKeyword(keyword: "preserved", contractId: "contract"))
        let lock = PersistentAssetLock(
            outPointHex: String(repeating: "ab", count: 32) + ":0",
            walletId: Data(repeating: 0x31, count: 32), transactionBytes: Data([1, 2, 3]),
            fundingTypeRaw: 4, identityIndexRaw: -1, amountDuffs: 100_000, statusRaw: 4)
        lock.recipientIsExternal = true
        context.insert(lock)
        context.insert(PersistentTrackedMasternode(
            networkRaw: Network.testnet.rawValue, proTxHash: Data(repeating: 7, count: 32),
            label: "fixture", addedAt: 1, snapshotJSON: "{}"))
        try context.save()
        let storedKey = try XCTUnwrap(context.fetch(FetchDescriptor<PersistentPublicKey>()).first)
        XCTAssertEqual(storedKey.totalBudgetCredits, 100_000)
        XCTAssertEqual(storedKey.expiresAtMillis, 1_800_000_000_000)
        XCTAssertEqual(storedKey.identity?.identityId, identity.identityId)
    }
}
