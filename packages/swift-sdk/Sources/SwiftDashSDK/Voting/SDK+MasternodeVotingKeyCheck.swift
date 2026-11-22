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
        let votingAddress = try MasternodeVotingAddress.of(privateKey: votingPrivateKey)
            .mapError { SDKError.invalidParameter($0.message) }
            .get()
        return try await checkMasternodeVotingKey(proTxHash: proTxHash, votingAddress: votingAddress)
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
