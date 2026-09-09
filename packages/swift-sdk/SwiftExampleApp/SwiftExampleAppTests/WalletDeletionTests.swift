import SwiftData
import XCTest
@testable import SwiftDashSDK

final class WalletDeletionTests: XCTestCase {

    func testDeleteWalletDataRemovesWalletFootprintAndLastNetworkSyncState() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let walletId = Data(repeating: 0x44, count: 32)
        let identityId = Data(repeating: 0x55, count: 32)

        let wallet = PersistentWallet(walletId: walletId, network: .testnet)
        let identity = PersistentIdentity(identityId: identityId, network: .testnet)
        identity.wallet = wallet
        wallet.identities.append(identity)

        let balance = PersistentTokenBalance(
            tokenId: "token-a",
            identityId: identityId,
            balance: 10,
            network: .testnet
        )
        balance.identity = identity
        identity.tokenBalances.append(balance)

        let pendingTx = PersistentTransaction(
            txid: Data(repeating: 0x66, count: 32),
            transactionData: Data([0x01])
        )
        let pending = PersistentPendingInput(
            outpoint: Data(repeating: 0x67, count: 36),
            inputIndex: 0,
            spendingTxid: pendingTx.txid,
            spendingTransaction: pendingTx,
            walletId: walletId
        )
        pendingTx.pendingInputs.append(pending)

        let orphanTx = PersistentTransaction(
            txid: Data(repeating: 0x77, count: 32),
            transactionData: Data([0x02])
        )

        let liveTx = PersistentTransaction(
            txid: Data(repeating: 0x88, count: 32),
            transactionData: Data([0x03])
        )
        let liveTxo = PersistentTxo(
            transaction: liveTx,
            vout: 0,
            amount: 1,
            address: "yTest"
        )
        liveTx.outputs.append(liveTxo)

        let syncState = PersistentPlatformAddressesSyncState(
            walletId: Self.syncStateScopeId(for: .testnet),
            network: .testnet,
            syncHeight: 10,
            syncTimestamp: 20,
            lastKnownRecentBlock: 30
        )

        context.insert(wallet)
        context.insert(identity)
        context.insert(balance)
        context.insert(pendingTx)
        context.insert(pending)
        context.insert(orphanTx)
        context.insert(liveTx)
        context.insert(liveTxo)
        context.insert(syncState)
        try context.save()

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try handler.deleteWalletData(walletId: walletId)
        try handler.deleteWalletData(walletId: walletId)

        XCTAssertTrue(try fetch(PersistentWallet.self, in: container).isEmpty)
        XCTAssertTrue(try fetch(PersistentIdentity.self, in: container).isEmpty)
        XCTAssertTrue(try fetch(PersistentTokenBalance.self, in: container).isEmpty)
        XCTAssertTrue(try fetch(PersistentPendingInput.self, in: container).isEmpty)
        XCTAssertTrue(try fetch(PersistentPlatformAddressesSyncState.self, in: container).isEmpty)

        let transactions = try fetch(PersistentTransaction.self, in: container)
        XCTAssertEqual(transactions.count, 1)
        XCTAssertEqual(transactions.first?.txid, liveTx.txid)
    }

    func testDeleteWalletDataRemovesAssetLockRowsAndPreservesOtherWallets() throws {
        // Regression test for the Codex P2 wallet-deletion finding on
        // dashpay/platform#3634: `deleteWalletData` previously left
        // `PersistentAssetLock` rows behind, so the wallet-load path
        // (`loadCachedAssetLocksOnQueue`) would resurrect stale
        // Pending / Resumable asset-lock state on a delete-then-
        // reimport of the same walletId.
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let walletId = Data(repeating: 0xa1, count: 32)
        let siblingId = Data(repeating: 0xb2, count: 32)

        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: siblingId, network: .testnet))

        let doomedLock = PersistentAssetLock(
            outPointHex: "deadbeef:0",
            walletId: walletId,
            transactionBytes: Data([0x01, 0x02, 0x03]),
            fundingTypeRaw: 0,
            identityIndexRaw: 0,
            accountIndexRaw: 0,
            amountDuffs: 100_000_000,
            statusRaw: 1
        )
        let siblingLock = PersistentAssetLock(
            outPointHex: "cafebabe:1",
            walletId: siblingId,
            transactionBytes: Data([0x04, 0x05, 0x06]),
            fundingTypeRaw: 0,
            identityIndexRaw: 0,
            accountIndexRaw: 0,
            amountDuffs: 200_000_000,
            statusRaw: 2
        )
        context.insert(doomedLock)
        context.insert(siblingLock)
        try context.save()

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try handler.deleteWalletData(walletId: walletId)

        let remaining = try fetch(PersistentAssetLock.self, in: container)
        XCTAssertEqual(remaining.count, 1)
        XCTAssertEqual(remaining.first?.walletId, siblingId)
        XCTAssertEqual(remaining.first?.outPointHex, "cafebabe:1")
    }

    func testDeleteWalletDataPreservesSiblingPayloadOnlyTransactions() throws {
        // Regression test for the thepastaclaw blocking finding on
        // dashpay/platform#4108: the post-delete orphan sweep matched
        // any `PersistentTransaction` with empty outputs / inputs /
        // pendingInputs. Payload-only special txs (a ProRegTx matching
        // a provider owner key) have no TXOs anywhere yet belong to a
        // live account through `involvedAccounts`, so deleting an
        // UNRELATED wallet swept the other wallet's payload-only
        // history. The deleted wallet's own payload-only rows must
        // still be swept — its account deletion nullifies their join
        // links before the sweep runs.
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let doomedId = Data(repeating: 0xc3, count: 32)
        let siblingId = Data(repeating: 0xd4, count: 32)

        let doomedWallet = PersistentWallet(walletId: doomedId, network: .testnet)
        let siblingWallet = PersistentWallet(walletId: siblingId, network: .testnet)
        context.insert(doomedWallet)
        context.insert(siblingWallet)

        // Provider Owner Keys account (tag 9) under each wallet.
        let doomedAccount = PersistentAccount(
            wallet: doomedWallet,
            accountType: 9,
            accountIndex: 0,
            accountTypeName: "Provider Owner Keys"
        )
        let siblingAccount = PersistentAccount(
            wallet: siblingWallet,
            accountType: 9,
            accountIndex: 0,
            accountTypeName: "Provider Owner Keys"
        )
        context.insert(doomedAccount)
        context.insert(siblingAccount)

        // Payload-only txs: no TXOs / pending inputs anywhere, linked
        // to their wallet solely through `involvedAccounts`.
        let doomedProRegTx = PersistentTransaction(
            txid: Data(repeating: 0xe5, count: 32),
            transactionData: Data([0x01])
        )
        doomedProRegTx.involvedAccounts.append(doomedAccount)
        let siblingProRegTx = PersistentTransaction(
            txid: Data(repeating: 0xf6, count: 32),
            transactionData: Data([0x02])
        )
        siblingProRegTx.involvedAccounts.append(siblingAccount)
        context.insert(doomedProRegTx)
        context.insert(siblingProRegTx)
        try context.save()

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try handler.deleteWalletData(walletId: doomedId)

        // The sibling's payload-only tx survives with its join intact;
        // the deleted wallet's own payload-only tx is swept.
        let transactions = try fetch(PersistentTransaction.self, in: container)
        XCTAssertEqual(transactions.map(\.txid), [siblingProRegTx.txid])
        XCTAssertEqual(
            transactions.first?.involvedAccounts.map(\.wallet.walletId),
            [siblingId]
        )
    }

    func testDeleteWalletDataKeepsNetworkSyncStateWhenSiblingWalletRemains() throws {
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)
        let walletId = Data(repeating: 0x99, count: 32)
        let siblingId = Data(repeating: 0xaa, count: 32)

        context.insert(PersistentWallet(walletId: walletId, network: .testnet))
        context.insert(PersistentWallet(walletId: siblingId, network: .testnet))
        context.insert(
            PersistentPlatformAddressesSyncState(
                walletId: Self.syncStateScopeId(for: .testnet),
                network: .testnet,
                syncHeight: 10,
                syncTimestamp: 20,
                lastKnownRecentBlock: 30
            )
        )
        try context.save()

        let handler = PlatformWalletPersistenceHandler(modelContainer: container, network: .testnet)
        try handler.deleteWalletData(walletId: walletId)

        let wallets = try fetch(PersistentWallet.self, in: container)
        XCTAssertEqual(wallets.map(\.walletId), [siblingId])
        XCTAssertEqual(try fetch(PersistentPlatformAddressesSyncState.self, in: container).count, 1)
    }

    @MainActor
    func testThrowingKeychainSweepsUseIsolatedService() throws {
        let manager = KeychainManager(serviceName: "org.dash.wallet-delete-tests.\(UUID().uuidString)")
        let identityId = Data(repeating: 0xbb, count: 32)
        let walletId = Data(repeating: 0xcc, count: 32)
        let publicKey = Data(repeating: 0xdd, count: 33).toHexString()

        XCTAssertNotNil(manager.storePrivateKey(Data(repeating: 0x01, count: 32), identityId: identityId, keyIndex: 0))
        XCTAssertNotNil(manager.storeSpecialKey(Data(repeating: 0x02, count: 32), identityId: identityId, keyType: .voting))
        XCTAssertNotNil(
            manager.storeIdentityPrivateKey(
                Data(repeating: 0x03, count: 32),
                derivationPath: "m/9'/5'/3'/1'",
                metadata: .init(
                    identityId: "identity",
                    keyId: 1,
                    walletId: walletId.toHexString(),
                    identityIndex: 0,
                    keyIndex: 0,
                    derivationPath: "m/9'/5'/3'/1'",
                    publicKey: publicKey,
                    publicKeyHash: Data(repeating: 0xee, count: 20).toHexString(),
                    keyType: 0,
                    purpose: 0,
                    securityLevel: 0
                )
            )
        )

        XCTAssertNotNil(manager.retrievePrivateKey(identityId: identityId, keyIndex: 0))
        XCTAssertNotNil(manager.retrieveSpecialKey(identityId: identityId, keyType: .voting))
        XCTAssertNotNil(manager.retrieveIdentityPrivateKey(publicKeyHex: publicKey))

        try manager.deleteAllKeychainItems(forIdentityId: identityId)

        XCTAssertNil(manager.retrievePrivateKey(identityId: identityId, keyIndex: 0))
        XCTAssertNil(manager.retrieveSpecialKey(identityId: identityId, keyType: .voting))
        XCTAssertNotNil(manager.retrieveIdentityPrivateKey(publicKeyHex: publicKey))

        try manager.deleteAllIdentityPrivateKeys(forWalletId: walletId)

        XCTAssertNil(manager.retrieveIdentityPrivateKey(publicKeyHex: publicKey))
    }

    private static func syncStateScopeId(for network: Network) -> Data {
        var data = Data("platform-sync:\(network.networkName)".utf8.prefix(32))
        if data.count < 32 {
            data.append(Data(repeating: 0, count: 32 - data.count))
        }
        return data
    }

    private func fetch<T: PersistentModel>(_ type: T.Type, in container: ModelContainer) throws -> [T] {
        try ModelContext(container).fetch(FetchDescriptor<T>())
    }
}
