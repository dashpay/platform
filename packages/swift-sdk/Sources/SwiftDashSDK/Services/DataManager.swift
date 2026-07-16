import Foundation
import SwiftData


/// Service to manage SwiftData operations for the app.
///
/// Scope has shrunk dramatically: identities, documents, and data
/// contracts are now read and written directly against their
/// `Persistent*` SwiftData models by views and services that own a
/// `ModelContext`. What remains here are:
///
///  - Token balance upserts (still routed via `saveTokenBalance` so
///    DPP → SwiftData conversion stays in one place).
///  - Bulk utility operations that don't have a natural home on a
///    specific view (`clearAllData`, `getDataStatistics`).
///  - The current-network pointer that some services read to scope
///    their queries.
@MainActor
public final class DataManager: ObservableObject {
    private let modelContext: ModelContext
    public var currentNetwork: Network

    public init(modelContext: ModelContext, currentNetwork: Network = .testnet) {
        self.modelContext = modelContext
        self.currentNetwork = currentNetwork
    }

    // MARK: - Token Balance Operations

    /// Save or update a token balance
    public func saveTokenBalance(tokenId: String, identityId: Data, balance: UInt64, frozen: Bool = false, tokenInfo: (name: String, symbol: String, decimals: Int32)? = nil) throws {
        let predicate = PersistentTokenBalance.predicate(tokenId: tokenId, identityId: identityId)
        let descriptor = FetchDescriptor<PersistentTokenBalance>(predicate: predicate)

        if let existingBalance = try modelContext.fetch(descriptor).first {
            // Update existing balance
            existingBalance.updateUnsignedBalance(balance)
            if frozen != existingBalance.frozen {
                if frozen {
                    existingBalance.freeze()
                } else {
                    existingBalance.unfreeze()
                }
            }
            if let info = tokenInfo {
                existingBalance.updateTokenInfo(name: info.name, symbol: info.symbol, decimals: info.decimals)
            }
        } else {
            // Create new balance
            let persistentBalance = PersistentTokenBalance(
                tokenId: tokenId,
                identityId: identityId,
                unsignedBalance: balance,
                frozen: frozen,
                tokenName: tokenInfo?.name,
                tokenSymbol: tokenInfo?.symbol,
                tokenDecimals: tokenInfo?.decimals,
                network: currentNetwork
            )
            modelContext.insert(persistentBalance)
        }

        try modelContext.save()
    }

    /// Fetch token balances for an identity
    public func fetchTokenBalances(identityId: Data) throws -> [(tokenId: String, balance: UInt64, frozen: Bool)] {
        let predicate = PersistentTokenBalance.predicate(identityId: identityId)
        let descriptor = FetchDescriptor<PersistentTokenBalance>(predicate: predicate)
        let persistentBalances = try modelContext.fetch(descriptor)
        return persistentBalances
            .sorted { $0.unsignedBalance > $1.unsignedBalance }
            .map { $0.toTokenBalance() }
    }

    // MARK: - Utility Operations

    /// Clear all data (for testing or reset)
    func clearAllData() throws {
        // Delete all identities
        try modelContext.delete(model: PersistentIdentity.self)

        // Delete all documents
        try modelContext.delete(model: PersistentDocument.self)

        // Delete all contracts
        try modelContext.delete(model: PersistentDataContract.self)

        // Delete all public keys
        try modelContext.delete(model: PersistentPublicKey.self)

        // Delete all token balances
        try modelContext.delete(model: PersistentTokenBalance.self)

        try modelContext.save()
    }

    /// Get statistics about stored data
    public func getDataStatistics() throws -> (identities: Int, documents: Int, contracts: Int, tokenBalances: Int) {
        let identityCount = try modelContext.fetchCount(FetchDescriptor<PersistentIdentity>())
        let documentCount = try modelContext.fetchCount(FetchDescriptor<PersistentDocument>())
        let contractCount = try modelContext.fetchCount(FetchDescriptor<PersistentDataContract>())
        let tokenBalanceCount = try modelContext.fetchCount(FetchDescriptor<PersistentTokenBalance>())

        return (identities: identityCount, documents: documentCount, contracts: contractCount, tokenBalances: tokenBalanceCount)
    }
}
