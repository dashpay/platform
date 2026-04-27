import Foundation
import SwiftDashSDK

// Re-export SDK Document types so app-side sources can use the
// bare names. The legacy `DPPDocument(from: DocumentModel)`
// convenience is gone — callers construct `DPPDocument` directly,
// or derive it from a `PersistentDocument` row when they need the
// DPP projection.
public typealias DPPDocument = SwiftDashSDK.DPPDocument
public typealias ExtendedDocument = SwiftDashSDK.ExtendedDocument
public typealias DocumentMetadata = SwiftDashSDK.DocumentMetadata
public typealias TokenPaymentInfo = SwiftDashSDK.TokenPaymentInfo
public typealias DocumentPatch = SwiftDashSDK.DocumentPatch
public typealias DocumentPropertyNames = SwiftDashSDK.DocumentPropertyNames

// MARK: - Helper Extensions

extension Data {
    /// Pad or truncate data to specified length
    func paddedToLength(_ length: Int) -> Data {
        if self.count >= length {
            return self.prefix(length)
        } else {
            var padded = self
            padded.append(Data(repeating: 0, count: length - self.count))
            return padded
        }
    }
}
