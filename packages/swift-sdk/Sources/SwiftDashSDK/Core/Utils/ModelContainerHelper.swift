import Foundation
import SwiftData

public struct ModelContainerHelper {
    /// Schema mirrored by both `createContainer` and
    /// `createInMemoryContainer`. Kept private + computed so the two
    /// container variants stay in lockstep — adding a model type
    /// in one place automatically applies to both.
    private static func schema() -> Schema {
        Schema([
            PersistentIdentity.self,
            PersistentPublicKey.self,
            PersistentDocument.self,
            PersistentTokenBalance.self,
            PersistentDataContract.self,
            PersistentToken.self,
            PersistentDocumentType.self,
            PersistentTokenHistoryEvent.self,
            PersistentKeyword.self,
            PersistentIndex.self,
            PersistentProperty.self,
            PersistentPlatformAddress.self,
            PersistentSyncState.self,
            PersistentWallet.self,
            PersistentAccount.self,
            PersistentCoreAddress.self,
            PersistentTransaction.self,
            PersistentUtxo.self,
            PersistentWalletManagerMetadata.self,
        ])
    }

    public static func createContainer() throws -> ModelContainer {
        let schema = Schema([
            // Platform + core-wallet rows. `PersistentWallet`
            // replaces the former `HDWallet` @Model as the
            // canonical SwiftData wallet row; the wallet-level
            // fields that lived on `HDWallet` (label, network,
            // isWatchOnly, isImported) are all on
            // `PersistentWallet` now.
            PersistentIdentity.self,
            PersistentPublicKey.self,
            PersistentDocument.self,
            PersistentTokenBalance.self,
            PersistentDataContract.self,
            PersistentToken.self,
            PersistentDocumentType.self,
            PersistentTokenHistoryEvent.self,
            PersistentKeyword.self,
            PersistentIndex.self,
            PersistentProperty.self,
            PersistentPlatformAddress.self,
            PersistentSyncState.self,
            PersistentWallet.self,
            PersistentAccount.self,
            PersistentCoreAddress.self,
            PersistentTransaction.self,
            PersistentUtxo.self,
            PersistentWalletManagerMetadata.self,
        ])

        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: false,
            allowsSave: true
        )

        return try ModelContainer(
            for: schema,
            configurations: [modelConfiguration]
        )
    }

    /// Build a fresh, ephemeral ModelContainer that lives entirely in
    /// memory. Used by the contracts-search preview flow: the caller
    /// fetches a contract, parses it into the in-memory container,
    /// and renders `DataContractDetailsView` against it. When the
    /// preview sheet is dismissed the container is dropped, taking
    /// the contract / token / document-type / index / property rows
    /// with it. No on-disk state is touched.
    public static func createInMemoryContainer() throws -> ModelContainer {
        let schema = schema()
        let modelConfiguration = ModelConfiguration(
            schema: schema,
            isStoredInMemoryOnly: true,
            allowsSave: true
        )
        return try ModelContainer(
            for: schema,
            configurations: [modelConfiguration]
        )
    }
}
