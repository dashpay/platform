import Foundation
import SwiftDashSDK

// Re-export SDK Identity types so app-side sources can use the
// bare names. The legacy `DPPIdentity(from: IdentityModel)`
// convenience is gone — callers build `DPPIdentity` directly, or
// derive it from a `PersistentIdentity` row when they need the DPP
// projection.
public typealias DPPIdentity = SwiftDashSDK.DPPIdentity
public typealias PartialIdentity = SwiftDashSDK.PartialIdentity
