import Foundation
import DashSDKFFI

extension SDK {
    /// Mark an informational check as pending before queuing its refresh.
    /// This local, synchronous call never restricts wallet operations.
    public func beginProtocolCompatibilityCheck() throws -> UInt64 {
        guard let handle else { throw SDKError.invalidState("SDK not initialized") }
        var generation: UInt64 = 0
        try Self.checkProtocolCompatibilityResult(
            dash_sdk_begin_protocol_compatibility_check(handle, &generation)
        )
        return generation
    }

    /// Perform Rust's bounded check. This blocks; call off the main actor.
    /// A superseded check returns a snapshot with the newer generation.
    public func refreshProtocolCompatibility(generation: UInt64) throws -> ProtocolCompatibility {
        guard let handle else { throw SDKError.invalidState("SDK not initialized") }
        return try Self.decodeProtocolCompatibilityResult(
            dash_sdk_refresh_protocol_compatibility(handle, generation)
        )
    }

    /// Read clone-shared state without contacting the network.
    public func protocolCompatibility() throws -> ProtocolCompatibility {
        guard let handle else { throw SDKError.invalidState("SDK not initialized") }
        return try Self.decodeProtocolCompatibilityResult(
            dash_sdk_get_protocol_compatibility(handle)
        )
    }

    private static func checkProtocolCompatibilityResult(_ result: DashSDKResult) throws {
        if let error = result.error {
            defer { dash_sdk_error_free(error) }
            throw SDKError.fromDashSDKError(error.pointee)
        }
    }

    private static func decodeProtocolCompatibilityResult<T: Decodable>(
        _ result: DashSDKResult
    ) throws -> T {
        try checkProtocolCompatibilityResult(result)
        guard let data = result.data else {
            throw SDKError.serializationError("Missing protocol compatibility result")
        }
        let string = data.assumingMemoryBound(to: CChar.self)
        defer { dash_sdk_string_free(string) }
        do {
            return try JSONDecoder().decode(T.self, from: Data(String(cString: string).utf8))
        } catch {
            throw SDKError.serializationError("Invalid protocol compatibility result: \(error)")
        }
    }
}
