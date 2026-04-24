import Foundation
import SwiftData

public struct ModelContainerHelper {
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
}
