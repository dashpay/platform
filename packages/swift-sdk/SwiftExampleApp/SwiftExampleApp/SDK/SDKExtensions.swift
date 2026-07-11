import Foundation
import SwiftDashSDK

// Re-export SDK types for backward compatibility.
//
// The `Signer` protocol now lives in SwiftDashSDK. Production
// signing is performed by `KeychainSigner` from SwiftDashSDK; the
// legacy `TestSigner` mock has been removed.
public typealias Signer = SwiftDashSDK.Signer
