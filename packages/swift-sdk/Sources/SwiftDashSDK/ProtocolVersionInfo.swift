/// Cached protocol information reported by Rust after proof and signature verification.
/// This is advisory and does not describe whether an operation is allowed.
public struct ProtocolVersionInfo: Decodable, Equatable, Sendable {
    public let verifiedVersion: UInt32?
    public let effectiveVersion: UInt32
    public let maximumSupportedVersion: UInt32
    public let newerProtocolAvailable: Bool
}
