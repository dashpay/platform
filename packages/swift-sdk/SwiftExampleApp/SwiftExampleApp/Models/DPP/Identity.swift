import Foundation
import SwiftDashSDK

// Re-export SDK Identity types for backward compatibility
public typealias DPPIdentity = SwiftDashSDK.DPPIdentity
public typealias PartialIdentity = SwiftDashSDK.PartialIdentity

// MARK: - App-Specific Extensions

extension DPPIdentity {
    /// Create an identity from our simplified IdentityModel
    init?(from model: IdentityModel) {
        // model.id is already Data, no conversion needed
        let idData = model.id

        self.init(
            id: idData,
            publicKeys: [:],
            balance: model.balance,
            revision: 0
        )

        // Note: In a real implementation, we would convert private keys to public keys
    }
}
