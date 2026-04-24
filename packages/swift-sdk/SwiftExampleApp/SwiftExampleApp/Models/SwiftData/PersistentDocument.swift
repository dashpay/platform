import Foundation
import SwiftDashSDK

// Re-export the SDK type so app-side sources can keep referring to
// an unqualified `PersistentDocument`. The legacy
// `toDocumentModel()` / `from(_:)` bridges that converted between
// this row and the deleted `DocumentModel` value type are gone;
// callers work against `PersistentDocument` directly now.
public typealias PersistentDocument = SwiftDashSDK.PersistentDocument
