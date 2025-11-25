import Foundation
import SwiftData
import SwiftDashSDK

// Re-export SDK type for backward compatibility
public typealias PersistentDocument = SwiftDashSDK.PersistentDocument

// App-specific extensions that depend on app types
extension SwiftDashSDK.PersistentDocument {
    func toDocumentModel() -> DocumentModel {
        let dataDict = (try? JSONSerialization.jsonObject(with: data, options: [])) as? [String: Any] ?? [:]

        return DocumentModel(
            id: documentId,
            contractId: contractId,
            documentType: documentType,
            ownerId: Data.identifier(fromBase58: ownerId) ?? Data(),
            data: dataDict,
            createdAt: createdAt,
            updatedAt: updatedAt,
            dppDocument: nil,
            revision: Revision(revision)
        )
    }

    static func from(_ document: DocumentModel) -> SwiftDashSDK.PersistentDocument {
        let dataToStore = (try? JSONSerialization.data(withJSONObject: document.data, options: [])) ?? Data()

        return SwiftDashSDK.PersistentDocument(
            documentId: document.id,
            documentType: document.documentType,
            revision: Int32(document.revision),
            data: dataToStore,
            contractId: document.contractId,
            ownerId: document.ownerId.toBase58String(),
            network: "testnet"
        )
    }
}
