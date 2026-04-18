import Foundation
import SwiftData

public struct ModelContainerHelper {
    public static func createContainer() throws -> ModelContainer {
        let schema = Schema([
            // Core models
            HDWallet.self,

            // Platform models
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
            PersistentAddressBalance.self,
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
