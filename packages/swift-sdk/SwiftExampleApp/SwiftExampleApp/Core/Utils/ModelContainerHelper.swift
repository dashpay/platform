import Foundation
import SwiftData

public struct ModelContainerHelper {
    public static func createContainer() throws -> ModelContainer {
        let schema = Schema([
            // Core models
            HDWallet.self,
            HDAddress.self,
            HDTransaction.self,
            HDUTXO.self,
            HDWatchedAddress.self,
            
            // Platform models
            PersistentIdentity.self,
            PersistentPublicKey.self,
            PersistentContract.self,
            PersistentDocument.self,
            PersistentTokenBalance.self,
            PersistentDataContract.self,
            PersistentToken.self,
            PersistentDocumentType.self,
            PersistentTokenHistoryEvent.self
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