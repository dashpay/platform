import Foundation
import DashSDKFFI

/// Why Rust did not submit a paid erase.
public enum DocumentEraseNotSubmittedReason: Sendable {
    case active
    case absent
    case notNewestOwner(Identifier)
    case preflightUnavailable
}

/// Submission and lifecycle observation are separate facts.
public enum DocumentEraseOutcome: Sendable {
    case notSubmitted(reason: DocumentEraseNotSubmittedReason, before: DocumentHistoryLifecycle?)
    case submitted(before: DocumentHistoryLifecycle?, after: DocumentHistoryLifecycle?)

    public var before: DocumentHistoryLifecycle? {
        switch self {
        case .notSubmitted(_, let before), .submitted(let before, _): return before
        }
    }

    public var after: DocumentHistoryLifecycle? {
        if case .submitted(_, let after) = self { return after }
        return nil
    }

    init(ffi: DocumentEraseOutcomeFFI) throws {
        guard (0...4).contains(ffi.reason) else {
            throw DocumentEraseResultUnreadable()
        }
        func observation(_ value: DocumentLifecycleObservationFFI) throws -> DocumentHistoryLifecycle? {
            if value.state == 0 { return nil }
            return try DocumentHistoryLifecycle(ffi: value)
        }
        let before = try observation(ffi.before)
        let after = try observation(ffi.after)
        switch ffi.kind {
        case 1:
            let reason: DocumentEraseNotSubmittedReason
            switch ffi.reason {
            case 1: reason = .active
            case 2: reason = .absent
            case 3:
                var owner = ffi.owner
                reason = .notNewestOwner(withUnsafeBytes(of: &owner) { Data($0) })
            case 4: reason = .preflightUnavailable
            default: throw DocumentEraseResultUnreadable()
            }
            self = .notSubmitted(reason: reason, before: before)
        case 2: self = .submitted(before: before, after: after)
        default: throw DocumentEraseResultUnreadable()
        }
    }
}

/// An OK native result could not be decoded; submission failure cannot be inferred.
public struct DocumentEraseResultUnreadable: LocalizedError {
    public var errorDescription: String? {
        "Erase result unreadable; read history before retrying."
    }
}
