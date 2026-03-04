import Foundation
import SwiftDashSDK

// Re-export SDK Document types for backward compatibility
public typealias DPPDocument = SwiftDashSDK.DPPDocument
public typealias ExtendedDocument = SwiftDashSDK.ExtendedDocument
public typealias DocumentMetadata = SwiftDashSDK.DocumentMetadata
public typealias TokenPaymentInfo = SwiftDashSDK.TokenPaymentInfo
public typealias DocumentPatch = SwiftDashSDK.DocumentPatch
public typealias DocumentPropertyNames = SwiftDashSDK.DocumentPropertyNames

// MARK: - App-Specific Extensions

extension DPPDocument {
    /// Create from our simplified DocumentModel
    init(from model: DocumentModel) {
        // model.id is a string, convert it to Data
        let documentId = Data.identifier(fromHex: model.id) ?? Data(repeating: 0, count: 32)
        // model.ownerId is already Data
        let ownerIdData = model.ownerId

        // Convert properties - in a real implementation, this would properly convert types
        var platformProperties: [String: PlatformValue] = [:]
        for (key, value) in model.data {
            if let stringValue = value as? String {
                platformProperties[key] = .string(stringValue)
            } else if let intValue = value as? Int {
                platformProperties[key] = .integer(Int64(intValue))
            } else if let boolValue = value as? Bool {
                platformProperties[key] = .bool(boolValue)
            }
            // Add more type conversions as needed
        }

        self.init(
            id: documentId,
            ownerId: ownerIdData,
            properties: platformProperties,
            revision: 0,
            createdAt: model.createdAt.map { TimestampMillis($0.timeIntervalSince1970 * 1000) },
            updatedAt: model.updatedAt.map { TimestampMillis($0.timeIntervalSince1970 * 1000) },
            transferredAt: nil,
            createdAtBlockHeight: nil,
            updatedAtBlockHeight: nil,
            transferredAtBlockHeight: nil,
            createdAtCoreBlockHeight: nil,
            updatedAtCoreBlockHeight: nil,
            transferredAtCoreBlockHeight: nil
        )
    }
}

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
