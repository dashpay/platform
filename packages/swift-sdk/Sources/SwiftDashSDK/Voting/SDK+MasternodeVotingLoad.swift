import Foundation
import DashSDKFFI

// MARK: - Loading a masternode to vote for it
//
// A thin bridge over `dash_sdk_masternode_load_for_voting` in `rs-sdk-ffi`,
// which calls rs-sdk's `load_masternode_for_voting`. Rust decides which
// identity the host stores for the masternode: its owner identity when
// Platform holds one, or else, for a masternode without an owner address (a
// shared masternode, from protocol version 14), its voter identity when
// Platform proves that one holds the enabled voting key. This marshals the
// arguments in and the decision out.

/// An identity ``SDK/loadMasternodeForVoting(proTxHash:votingAddress:)``
/// loaded for a masternode.
public struct MasternodeVotingLoadedIdentity: Sendable {
    /// Which of the masternode's identities was loaded.
    public enum Role: String, Equatable, Sendable {
        /// The owner identity, whose id is the pro_tx_hash.
        case owner
        /// The voter identity of a masternode with no owner identity. Its
        /// id is not the pro_tx_hash.
        case voter
    }

    /// Which identity was loaded.
    public let role: Role
    /// The loaded identity's 32-byte id: store the row under it. The
    /// masternode's pro_tx_hash is kept beside it, never derived from it.
    public let identityId: Data
    /// The loaded identity as JSON, the shape ``SDK/identityGet(identityId:)``
    /// parses.
    public let identityJSON: String
    /// The voting key's check, when a voting address was given: always
    /// ``MasternodeVotingKeyCheck/valid`` for ``Role/voter``. For
    /// ``Role/owner`` apply ``MasternodeVotingKeyCheck/outcome(votingAddress:slot:)``
    /// to it as for any checked key.
    public let votingKeyCheck: MasternodeVotingKeyCheck?

    /// ``identityJSON`` as the dictionary ``SDK/identityGet(identityId:)``
    /// returns.
    public func identityObject() throws -> sending [String: Any] {
        guard let data = identityJSON.data(using: .utf8),
              let object = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse JSON data")
        }
        return object
    }
}

/// What ``SDK/loadMasternodeForVoting(proTxHash:votingAddress:)`` decided.
public enum MasternodeVotingLoad: Sendable {
    /// Store this identity for the masternode.
    case loaded(MasternodeVotingLoadedIdentity)
    /// Store nothing. `message`, worded in Rust, says why. When the voting key
    /// was refused, `votingKeyRefusedFor` is the voter identity it was refused
    /// for: a row this device already stores for that identity holding the
    /// same key records the refusal
    /// (``MasternodeVotingKeyCheck/outcome(votingAddress:slot:)`` with
    /// ``MasternodeVotingKeyCheck/invalid(message:)``). Nothing is created or
    /// replaced.
    case refused(message: String, votingKeyRefusedFor: Data?)
}

extension SDK {
    /// Which identity to store to vote for the masternode `proTxHash`, with
    /// the voting key of `votingAddress` when the user gave one
    /// (`dash_sdk_masternode_load_for_voting`).
    ///
    /// - Parameters:
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     as the masternode vote functions take it.
    ///   - votingAddress: The 20-byte voting address of the voting key, or
    ///     `nil` when no voting key was given.
    /// - Throws: ``SDKError/invalidParameter(_:)`` for a pro_tx_hash of the
    ///   wrong length, and the FFI's error when nothing could be proved (the
    ///   owner identity could not be fetched, or it is absent and the voter
    ///   identity could not be fetched).
    public nonisolated func loadMasternodeForVoting(
        proTxHash: Data,
        votingAddress: MasternodeVotingKeyAddress?
    ) async throws -> MasternodeVotingLoad {
        // The FFI reads exactly 32 bytes through this pointer.
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        return try await performBlockingQuery { handle in
            var load = DashSDKMasternodeVotingLoad()
            let result = proTxHash.withUnsafeBytes { proTxPtr in
                dash_sdk_masternode_load_for_voting(
                    handle,
                    proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                    votingAddress != nil,
                    MasternodeVotingKeyCheckFFI.address(votingAddress),
                    &load
                )
            }
            try DecisionProposalFFI.throwIfFailed(result)
            return try MasternodeVotingLoadFFI.consume(&load)
        }
    }
}

extension MasternodeVotingKeyCheck {
    /// The message a voting key carries once another key was proved valid
    /// for its masternode.
    public static let checkAgainAfterAnotherKeyMessage =
        "Another voting key was loaded for this masternode, so this one is checked again before it votes."

    /// What the host records for another voting key it stores for the same
    /// masternode, once this check of the key with `votingAddress` was made
    /// on a load (`dash_sdk_masternode_sibling_voting_key_record`, rs-sdk's
    /// `sibling_voting_key_record`, which holds the rule).
    ///
    /// - Parameters:
    ///   - votingAddress: The voting address of the key just loaded.
    ///   - sibling: The record the host holds for the other key.
    /// - Returns: The record to persist for the other key, `nil` to leave it
    ///   unchanged.
    public func siblingRecord(
        votingAddress: MasternodeVotingKeyAddress,
        sibling: MasternodeRecordedVotingKey?
    ) -> MasternodeRecordedVotingKey? {
        let record = dash_sdk_masternode_sibling_voting_key_record(
            MasternodeVotingKeyCheckFFI.verdict(of: self),
            MasternodeVotingKeyCheckFFI.address(votingAddress),
            MasternodeVotingKeyCheckFFI.recorded(sibling)
        )
        return MasternodeVotingKeyCheckFFI.recordedKey(
            record,
            preferring: .undetermined(message: Self.checkAgainAfterAnotherKeyMessage)
        )
    }
}

/// Marshalling of `DashSDKMasternodeVotingLoad`.
enum MasternodeVotingLoadFFI {
    /// The decision in `load`, with its two C strings freed on every path.
    static func consume(_ load: inout DashSDKMasternodeVotingLoad) throws -> MasternodeVotingLoad {
        let identityJSON = take(&load.identity_json)
        let message = take(&load.message)
        let role: MasternodeVotingLoadedIdentity.Role
        switch load.kind {
        case DashSDKMasternodeVotingLoadKind_OwnerIdentity:
            role = .owner
        case DashSDKMasternodeVotingLoadKind_VoterIdentity:
            role = .voter
        case DashSDKMasternodeVotingLoadKind_Refused:
            // Rust sets a verdict on a refusal only for a refused voting
            // key, and then names the voter identity it was refused for.
            let refusedFor = load.has_voting_key_verdict
                ? withUnsafeBytes(of: load.identity_id) { Data($0) }
                : nil
            return .refused(message: message ?? "", votingKeyRefusedFor: refusedFor)
        default:
            throw SDKError.internalError(
                "dash_sdk_masternode_load_for_voting returned an unknown kind \(load.kind.rawValue)"
            )
        }
        guard let identityJSON else {
            throw SDKError.internalError("dash_sdk_masternode_load_for_voting loaded no identity")
        }
        var votingKeyCheck: MasternodeVotingKeyCheck?
        if load.has_voting_key_verdict {
            switch load.voting_key_verdict {
            case DashSDKVotingKeyVerdict_Valid:
                votingKeyCheck = .valid
            case DashSDKVotingKeyVerdict_Invalid:
                votingKeyCheck = .invalid(message: message ?? "")
            case DashSDKVotingKeyVerdict_Undetermined:
                votingKeyCheck = .undetermined(message: message ?? "")
            default:
                throw SDKError.internalError(
                    "dash_sdk_masternode_load_for_voting returned an unknown verdict \(load.voting_key_verdict.rawValue)"
                )
            }
        }
        let identityId = withUnsafeBytes(of: load.identity_id) { Data($0) }
        return .loaded(MasternodeVotingLoadedIdentity(
            role: role,
            identityId: identityId,
            identityJSON: identityJSON,
            votingKeyCheck: votingKeyCheck
        ))
    }

    /// The C string at `pointer`, freed and set to null.
    private static func take(_ pointer: inout UnsafeMutablePointer<CChar>?) -> String? {
        guard let owned = pointer else { return nil }
        pointer = nil
        let string = String(cString: owned)
        dash_sdk_string_free(owned)
        return string
    }
}
