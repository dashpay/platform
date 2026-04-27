import Foundation
import SwiftDashSDK

// Re-export the SDK type so app-side sources can keep referring to
// an unqualified `PersistentDataContract`. The legacy
// `toContractModel()` / `from(_:)` bridges that converted between
// this row and the deleted `ContractModel` value type are gone;
// callers work against `PersistentDataContract` directly now.
public typealias PersistentDataContract = SwiftDashSDK.PersistentDataContract
