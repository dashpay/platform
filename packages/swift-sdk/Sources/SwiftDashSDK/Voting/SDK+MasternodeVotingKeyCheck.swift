import Foundation
import DashSDKFFI

// MARK: - Checking a masternode voting key
//
// A thin bridge over `dash_sdk_masternode_check_voting_key` in `rs-sdk-ffi`,
// which calls rs-sdk's `check_masternode_voting_key`: one proof-verified fetch
// of the masternode's voter identity decides whether a voting key can vote
// for the masternode. Rust decides the verdict and words its message; this
// marshals the arguments in and the verdict out.

/// Whether a voting key can vote for a masternode, as Platform's state
/// proves it (``SDK/checkMasternodeVotingKey(proTxHash:votingAddress:)``).
public enum MasternodeVotingKeyCheck: Equatable, Sendable {
    /// The masternode's voter identity holds an enabled voting key for this
    /// address, so the key can vote, apart from one history Platform's state
    /// cannot rule out (rs-sdk's `VotingKeyCheck`: a masternode that moved its
    /// voting address away and back before protocol version 15). Store it as
    /// checked.
    case valid
    /// Platform proves the key cannot vote for the masternode: the
    /// pro_tx_hash is not a masternode, the key is not its voting key, or
    /// the key was rotated away. Refuse the key. `message` says which.
    case invalid(message: String)
    /// The voter identity could not be fetched, so nothing is known about the
    /// key. The key may be kept, marked unchecked, and checked again later.
    /// `message` says why.
    case undetermined(message: String)
}

extension SDK {
    /// Whether the voting key with `votingAddress` can vote for the
    /// masternode `proTxHash` (`dash_sdk_masternode_check_voting_key`).
    ///
    /// One proof-verified fetch of the masternode's voter identity; no Core
    /// or SPV sync is needed. A failed fetch is
    /// ``MasternodeVotingKeyCheck/undetermined(message:)``, not an error. A
    /// masternode registered, or a voting key changed, in the last few blocks
    /// reads as ``MasternodeVotingKeyCheck/invalid(message:)`` until Platform
    /// processes the masternode list update.
    ///
    /// - Parameters:
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**,
    ///     as the masternode vote functions take it, not the byte order of
    ///     the hex Core displays.
    ///   - votingAddress: The masternode's 20-byte voting address, the
    ///     hash160 of the voting public key.
    /// - Throws: ``SDKError/invalidParameter(_:)`` for a value of the wrong
    ///   length, and the FFI's error if the call itself fails.
    public nonisolated func checkMasternodeVotingKey(
        proTxHash: Data,
        votingAddress: Data
    ) async throws -> MasternodeVotingKeyCheck {
        // The FFI reads exactly 32 and 20 bytes through these pointers.
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        guard votingAddress.count == 20 else {
            throw SDKError.invalidParameter("votingAddress must be exactly 20 bytes")
        }
        return try await performBlockingQuery { handle in
            var verdict = DashSDKVotingKeyVerdict_Undetermined
            let result = proTxHash.withUnsafeBytes { proTxPtr in
                votingAddress.withUnsafeBytes { addressPtr in
                    dash_sdk_masternode_check_voting_key(
                        handle,
                        proTxPtr.bindMemory(to: UInt8.self).baseAddress,
                        addressPtr.bindMemory(to: UInt8.self).baseAddress,
                        &verdict
                    )
                }
            }
            // Frees the error or the message on every path.
            let message = try DecisionProposalFFI.consumeString(result)
            switch verdict {
            case DashSDKVotingKeyVerdict_Valid:
                return .valid
            case DashSDKVotingKeyVerdict_Invalid:
                return .invalid(message: message ?? "")
            case DashSDKVotingKeyVerdict_Undetermined:
                return .undetermined(message: message ?? "")
            default:
                throw SDKError.internalError(
                    "dash_sdk_masternode_check_voting_key returned an unknown verdict \(verdict.rawValue)"
                )
            }
        }
    }

    /// Whether the voting private key `votingPrivateKey` can vote for the
    /// masternode `proTxHash`: its voting address is formed in Rust
    /// (`dash_sdk_masternode_voting_address_from_private_key`), then checked
    /// as ``checkMasternodeVotingKey(proTxHash:votingAddress:)`` does.
    ///
    /// - Parameters:
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash in **wire order**.
    ///   - votingPrivateKey: The 32-byte voting private key. It is not
    ///     retained and never leaves this device.
    /// - Throws: ``SDKError/invalidParameter(_:)`` when the key is not a
    ///   secp256k1 secret key or a value has the wrong length.
    public nonisolated func checkMasternodeVotingKey(
        proTxHash: Data,
        votingPrivateKey: Data
    ) async throws -> MasternodeVotingKeyCheck {
        let votingAddress = try SDK.masternodeVotingAddress(privateKey: votingPrivateKey)
        return try await checkMasternodeVotingKey(proTxHash: proTxHash, votingAddress: votingAddress)
    }
}

// MARK: - What a host does with a checked or stored voting key
//
// Thin bridges over `dash_sdk_masternode_voting_key_check_outcome` and
// `dash_sdk_masternode_recorded_voting_key_use` in `rs-sdk-ffi`, which call
// rs-sdk's `voting_key_check_outcome` and `recorded_voting_key_use`. Rust
// holds the rules (store or refuse a key, what to record for it, whether a
// stored key may vote or is checked first, and which record belongs to which
// key); the host only reports what it stores and persists what comes back.
//
// Voting addresses here are the 20-byte hash160 of the voting public key, as
// ``SDK/masternodeVotingAddress(privateKey:)`` and the Keychain signer return
// them. Passing any other length is a programming error and traps.

/// A check the host recorded, with the voting address of the key it was made
/// of. The address ties the record to its key: Rust uses a record only for a
/// key with that address.
public struct MasternodeRecordedVotingKey: Equatable, Sendable {
    /// What the check found.
    public let check: MasternodeVotingKeyCheck
    /// The 20-byte voting address of the key that was checked.
    public let votingAddress: Data

    /// A recorded check with its key's address; `nil` when `votingAddress`
    /// is not 20 bytes, so it is no address and the record is no record.
    public init?(check: MasternodeVotingKeyCheck, votingAddress: Data) {
        guard votingAddress.count == 20 else { return nil }
        self.check = check
        self.votingAddress = votingAddress
    }
}

/// What the host's voting key slot for a masternode holds, compared with the
/// key that was checked. The host reads its Keychain and its identity row to
/// say this; Rust decides what follows from it
/// (``MasternodeVotingKeyCheck/outcome(votingAddress:slot:)``).
public enum MasternodeVotingKeySlot: Equatable, Sendable {
    /// No voting key is stored for the masternode.
    case empty
    /// Exactly the checked key is stored, with the record the host holds for
    /// the masternode, `nil` when none is.
    case sameKey(recorded: MasternodeRecordedVotingKey?)
    /// A different key is stored.
    case differentKey
}

/// What the host does with a voting key once checked, as Rust decides it
/// (``MasternodeVotingKeyCheck/outcome(votingAddress:slot:)``).
public struct MasternodeVotingKeyCheckOutcome: Equatable, Sendable {
    /// Whether the host stores the checked key.
    public let storeKey: Bool
    /// What the host records, with the checked key's voting address, for the
    /// checked key once it is in the slot, or for the stored key when it is
    /// the same one. `nil` leaves the record unchanged.
    public let record: MasternodeVotingKeyCheck?
}

/// How the host may use a stored voting key, as Rust decides it from the
/// record the host holds for the masternode.
public struct MasternodeRecordedVotingKeyUse: Equatable, Sendable {
    /// Whether the key is offered for voting.
    public let canVote: Bool
    /// Whether the host checks the key
    /// (``SDK/checkMasternodeVotingKey(proTxHash:votingAddress:)``) before it
    /// uses it.
    public let checkBeforeUse: Bool

    /// How the stored voting key may be used
    /// (`dash_sdk_masternode_recorded_voting_key_use`).
    ///
    /// - Parameters:
    ///   - recorded: The record the host holds for the masternode, `nil`
    ///     when it holds none.
    ///   - storedVotingAddress: The 20-byte voting address of the stored key,
    ///     when the host has read it; `nil` otherwise.
    public init(recorded: MasternodeRecordedVotingKey?, storedVotingAddress: Data?) {
        let keyUse = dash_sdk_masternode_recorded_voting_key_use(
            MasternodeVotingKeyCheckFFI.recorded(recorded),
            storedVotingAddress != nil,
            MasternodeVotingKeyCheckFFI.address(storedVotingAddress)
        )
        self.canVote = keyUse.can_vote
        self.checkBeforeUse = keyUse.check_before_use
    }
}

extension MasternodeVotingKeyCheck {
    /// What the host does with the key this check was made of, given what
    /// its voting key slot holds
    /// (`dash_sdk_masternode_voting_key_check_outcome`).
    ///
    /// - Parameters:
    ///   - votingAddress: The 20-byte voting address of the checked key.
    ///   - slot: What the host's voting key slot for the masternode holds,
    ///     compared with the checked key.
    /// - Returns: Whether to store the key, and what to record for it with
    ///   `votingAddress`. A record of this check's own kind is this check,
    ///   with its message.
    public func outcome(
        votingAddress: Data,
        slot: MasternodeVotingKeySlot
    ) -> MasternodeVotingKeyCheckOutcome {
        let ffiSlot: DashSDKVotingKeySlot
        var recordedForSameKey: MasternodeRecordedVotingKey?
        switch slot {
        case .empty:
            ffiSlot = DashSDKVotingKeySlot_Empty
        case .sameKey(let recorded):
            ffiSlot = DashSDKVotingKeySlot_SameKey
            recordedForSameKey = recorded
        case .differentKey:
            ffiSlot = DashSDKVotingKeySlot_DifferentKey
        }
        let outcome = dash_sdk_masternode_voting_key_check_outcome(
            MasternodeVotingKeyCheckFFI.verdict(of: self),
            MasternodeVotingKeyCheckFFI.address(votingAddress),
            ffiSlot,
            MasternodeVotingKeyCheckFFI.recorded(recordedForSameKey)
        )
        return MasternodeVotingKeyCheckOutcome(
            storeKey: outcome.store_key,
            record: MasternodeVotingKeyCheckFFI.check(recorded: outcome.record, preferring: self)
        )
    }
}

/// Marshalling between the Swift voting key types and the FFI's.
enum MasternodeVotingKeyCheckFFI {
    static func verdict(of check: MasternodeVotingKeyCheck) -> DashSDKVotingKeyVerdict {
        switch check {
        case .valid: return DashSDKVotingKeyVerdict_Valid
        case .invalid: return DashSDKVotingKeyVerdict_Invalid
        case .undetermined: return DashSDKVotingKeyVerdict_Undetermined
        }
    }

    /// The FFI address for 20 address bytes; all zero for `nil`, which the
    /// FFI does not read.
    static func address(_ bytes: Data?) -> DashSDKVotingAddress {
        var address = DashSDKVotingAddress()
        guard let bytes else { return address }
        precondition(bytes.count == 20, "a voting address has 20 bytes, not \(bytes.count)")
        withUnsafeMutableBytes(of: &address.bytes) { $0.copyBytes(from: bytes) }
        return address
    }

    static func recorded(_ recorded: MasternodeRecordedVotingKey?) -> DashSDKRecordedVotingKey {
        let check: DashSDKRecordedVotingKeyCheck
        switch recorded?.check {
        case nil: check = DashSDKRecordedVotingKeyCheck_None
        case .valid: check = DashSDKRecordedVotingKeyCheck_Valid
        case .invalid: check = DashSDKRecordedVotingKeyCheck_Invalid
        case .undetermined: check = DashSDKRecordedVotingKeyCheck_Undetermined
        }
        return DashSDKRecordedVotingKey(check: check, voting_address: address(recorded?.votingAddress))
    }

    /// The check for a record Rust returned: `nil` for `None` (leave the
    /// record unchanged), `check` itself when it is of the returned kind, so
    /// its message is kept, otherwise the kind with no message.
    static func check(
        recorded: DashSDKRecordedVotingKeyCheck,
        preferring check: MasternodeVotingKeyCheck
    ) -> MasternodeVotingKeyCheck? {
        let kind: MasternodeVotingKeyCheck
        switch recorded {
        case DashSDKRecordedVotingKeyCheck_Valid:
            kind = .valid
        case DashSDKRecordedVotingKeyCheck_Invalid:
            kind = .invalid(message: "")
        case DashSDKRecordedVotingKeyCheck_Undetermined:
            kind = .undetermined(message: "")
        default:
            // `None`. The C enum holds no other value today.
            return nil
        }
        return verdict(of: kind) == verdict(of: check) ? check : kind
    }
}

extension SDK {
    /// The 20-byte voting address of a 32-byte masternode voting private key,
    /// formed in Rust (`dash_sdk_masternode_voting_address_from_private_key`).
    ///
    /// - Throws: ``SDKError/invalidParameter(_:)`` when the key is not a
    ///   secp256k1 secret key or has the wrong length.
    public nonisolated static func masternodeVotingAddress(privateKey: Data) throws -> Data {
        try MasternodeVotingAddress.of(privateKey: privateKey)
            .mapError { SDKError.invalidParameter($0.message) }
            .get()
    }
}

/// The voting address Rust forms from a 32-byte masternode voting private key
/// (`dash_sdk_masternode_voting_address_from_private_key`): bytes in, bytes
/// out. Shared by ``SDK/checkMasternodeVotingKey(proTxHash:votingPrivateKey:)``
/// and the `KeychainSigner` voting key reads, each mapping the refusal onto its
/// own error.
enum MasternodeVotingAddress {
    struct Refusal: Error {
        let message: String
    }

    static func of(privateKey: Data) -> Result<Data, Refusal> {
        // The FFI reads exactly 32 bytes through the key pointer.
        guard privateKey.count == 32 else {
            return .failure(Refusal(
                message: "A voting private key has 32 bytes, not \(privateKey.count)"
            ))
        }
        var address = [UInt8](repeating: 0, count: 20)
        let result = privateKey.withUnsafeBytes { keyRaw in
            address.withUnsafeMutableBufferPointer { out in
                dash_sdk_masternode_voting_address_from_private_key(
                    keyRaw.bindMemory(to: UInt8.self).baseAddress,
                    out.baseAddress
                )
            }
        }
        if let error = result.error {
            let message = error.pointee.message.map { String(cString: $0) } ?? "unknown error"
            dash_sdk_error_free(error)
            return .failure(Refusal(message: message))
        }
        return .success(Data(address))
    }
}
