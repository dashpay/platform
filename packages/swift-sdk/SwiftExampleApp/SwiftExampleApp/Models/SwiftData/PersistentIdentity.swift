import Foundation
import SwiftDashSDK

// Re-export the SDK type so app-side sources can keep referring to
// an unqualified `PersistentIdentity`. The legacy
// `toIdentityModel()` / `from(_:)` bridges that converted between
// this row and the deleted `IdentityModel` value type are gone;
// callers work against `PersistentIdentity` directly now.
public typealias PersistentIdentity = SwiftDashSDK.PersistentIdentity
