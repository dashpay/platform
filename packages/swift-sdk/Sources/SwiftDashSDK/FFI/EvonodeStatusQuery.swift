import DashSDKFFI
import Foundation

// Not part of the `@MainActor` query extension on purpose: this is a
// blocking FFI call that callers run off the main actor (like
// `Identities.getBalance`), so it must stay nonisolated.
extension SDK {
    /// Ask ONE evonode for its DAPI `getStatus` self-report.
    ///
    /// Unlike the queries in `PlatformQueryExtensions` this does not go
    /// through the SDK's address list: the request is sent to `address` only
    /// (`https://<host>:<port>`, e.g. `PlatformMasternode.platformDAPIAddress`),
    /// with a single retry and no failover to another node — an unreachable
    /// node surfaces as `SDKError.networkError` / `.timeout`. The response is
    /// unproved by nature (the node describing itself); see `EvonodeStatus`.
    ///
    /// Blocking FFI call — run it off the main actor.
    public func getEvonodeStatus(address: String) throws -> EvonodeStatus {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = address.withCString { cAddress in
            dash_sdk_evonode_get_status(handle, cAddress)
        }

        if let error = result.error {
            defer { dash_sdk_error_free(error) }
            throw SDKError.fromDashSDKError(error.pointee)
        }
        guard let dataPtr = result.data else {
            throw SDKError.notFound("No status returned")
        }

        let cString = dataPtr.assumingMemoryBound(to: CChar.self)
        let json = String(cString: cString)
        dash_sdk_string_free(cString)

        do {
            return try JSONDecoder().decode(EvonodeStatus.self, from: Data(json.utf8))
        } catch {
            throw SDKError.serializationError("Failed to decode evonode status: \(error)")
        }
    }
}
