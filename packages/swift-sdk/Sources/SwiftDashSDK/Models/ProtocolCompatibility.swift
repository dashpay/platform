import Foundation

/// An informational snapshot of Rust's verified protocol observations.
///
/// `effectiveProtocolVersion` is the supported table used by this SDK.
/// `verifiedProtocolVersion` is the highest accepted verified network version
/// observed by this SDK, retained even when a later check fails. `status` describes only
/// the latest check; it never grants or denies permission to write.
public struct ProtocolCompatibility: Codable, Equatable, Sendable {
    public enum Status: String, Codable, Sendable {
        case unavailable
        case checking
        case checked
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

    /// True only when authenticated evidence exceeds this SDK's supported tables.
    public let newerProtocolAvailable: Bool

    public init(
        generation: UInt64,
        status: Status,
        verifiedProtocolVersion: UInt32?,
        effectiveProtocolVersion: UInt32,
        maximumSupportedProtocolVersion: UInt32,
        newerProtocolAvailable: Bool,
        unavailableReason: UnavailableReason? = nil
    ) {
        self.generation = generation
        self.status = status
        self.verifiedProtocolVersion = verifiedProtocolVersion
        self.effectiveProtocolVersion = effectiveProtocolVersion
        self.maximumSupportedProtocolVersion = maximumSupportedProtocolVersion
        self.newerProtocolAvailable = newerProtocolAvailable
        self.unavailableReason = unavailableReason
    }
}
