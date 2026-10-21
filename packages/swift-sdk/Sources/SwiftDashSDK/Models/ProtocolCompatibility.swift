import Foundation

/// Opt-in write policy enforced by the shared Rust SDK.
public enum ProtocolCompatibilityPolicy: UInt32, Sendable {
    case legacy = 0
    case requireVerified = 1
}

/// A snapshot of Rust's verified protocol compatibility state.
///
/// `effectiveProtocolVersion` is the supported table used by this SDK.
/// `verifiedProtocolVersion` is authenticated network evidence, which can
/// exceed that table. Only `status` describes write readiness.
public struct ProtocolCompatibility: Codable, Equatable, Sendable {
    public enum Status: String, Codable, Sendable {
        case unavailable
        case checking
        case supported
        case unsupported
    }

    public enum UnavailableReason: String, Codable, Sendable {
        case notChecked
        case timedOut
        case checkFailed
        case proofsDisabled
    }

    public let generation: UInt64
    public let status: Status
    public let verifiedProtocolVersion: UInt32?
    public let effectiveProtocolVersion: UInt32
    public let maximumSupportedProtocolVersion: UInt32
    public let unavailableReason: UnavailableReason?

    public init(
        generation: UInt64,
        status: Status,
        verifiedProtocolVersion: UInt32?,
        effectiveProtocolVersion: UInt32,
        maximumSupportedProtocolVersion: UInt32,
        unavailableReason: UnavailableReason? = nil
    ) {
        self.generation = generation
        self.status = status
        self.verifiedProtocolVersion = verifiedProtocolVersion
        self.effectiveProtocolVersion = effectiveProtocolVersion
        self.maximumSupportedProtocolVersion = maximumSupportedProtocolVersion
        self.unavailableReason = unavailableReason
    }
}

/// Rust-issued context for work prepared against a particular SDK and check.
/// Retain it with a preparation and ask Rust to validate it before use.
public struct PlatformWritePermit: Codable, Equatable, Sendable {
    public let instanceID: UInt64
    public let generation: UInt64
    public let protocolVersion: UInt32

    public init(instanceID: UInt64, generation: UInt64, protocolVersion: UInt32) {
        self.instanceID = instanceID
        self.generation = generation
        self.protocolVersion = protocolVersion
    }
}
