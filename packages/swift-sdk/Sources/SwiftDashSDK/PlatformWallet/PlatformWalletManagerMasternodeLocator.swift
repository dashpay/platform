import DashSDKFFI
import Foundation

/// A private key's job on a masternode. Raw values are the FFI wire values;
/// the first four line up with the Android wallet's `MasternodeKeyType`.
public enum MasternodeKeyRole: UInt8, CaseIterable, Sendable, Hashable {
    /// secp256k1; signs ProUpRegTx, OWNER key of the Platform owner identity
    /// (can sign withdrawals).
    case owner = 0
    /// secp256k1; governance / contested-resource voting.
    case voting = 1
    /// BLS12-381; signs ProUpServTx.
    case `operator` = 2
    /// ed25519; the Tenderdash node key — identifies an evonode, signs
    /// nothing a wallet does.
    case platformNode = 3
    /// secp256k1 key of the owner payout address — the TRANSFER key of the
    /// owner identity, i.e. what withdraws owner rewards.
    case ownerPayout = 4
    /// secp256k1 key of the operator payout address.
    case operatorPayout = 5

    /// Roles encoded in an FFI bit mask (bit = raw value).
    static func roles(fromMask mask: UInt8) -> [MasternodeKeyRole] {
        allCases.filter { mask & (1 << $0.rawValue) != 0 }
    }
}

/// How the locator found a masternode.
public enum MasternodeLocatorMatchKind: UInt8, Sendable {
    case proTxHash = 0
    case serviceAddress = 1
    /// A pasted private key — see `MasternodeLocateMatch.matchedKeys`.
    case key = 2
}

/// Outcome of the optional Platform step of a locate.
public enum MasternodePlatformLookup: UInt8, Sendable {
    /// The input had no secp256k1 key, so Platform had nothing to add.
    case notNeeded = 0
    /// A secp256k1 key was given but `searchPlatform` was off.
    case notRequested = 1
    /// Ran to completion.
    case done = 2
    /// Attempted and failed; the local matches stand, owner / payout roles
    /// were not checked. `MasternodeLocateResult.platformError` says why.
    case unavailable = 3
}

/// One masternode the locator text names: what the deterministic masternode
/// list knows about it, plus how it was matched.
public struct MasternodeLocateMatch: Sendable, Hashable {
    /// proTxHash, 32 WIRE-order bytes — same orientation as
    /// `PlatformMasternode.proTxHash`.
    public let proTxHash: Data
    /// `"ip:port"` of the Core P2P endpoint; `nil` for Tor / I2P-only entries.
    public let serviceAddress: String?
    /// Platform HTTP (DAPI) port — evonodes only.
    public let platformHTTPPort: UInt16?
    /// Operator BLS public key as serialized in the list (48 bytes).
    public let operatorPublicKey: Data
    /// Voting key id (hash160, 20 bytes).
    public let votingKeyId: Data
    /// Tenderdash node id (20 bytes) — evonodes only.
    public let platformNodeId: Data?
    /// `false` when PoSe-banned.
    public let isValid: Bool
    public let isEvonode: Bool
    public let matchedBy: MasternodeLocatorMatchKind
    /// Roles the pasted key fills on this masternode (empty unless
    /// `matchedBy == .key`). Usually one; a key used as both owner and
    /// voting key yields two.
    public let matchedKeys: [MasternodeKeyRole]
    /// Set when this masternode is already one of a loaded wallet's own —
    /// show "already in wallet" instead of offering to track it.
    public let inWalletId: Data?

    /// proTxHash in display (explorer / Tenderdash / identity id) order.
    public var proTxHashHex: String {
        Data(proTxHash.reversed()).map { String(format: "%02x", $0) }.joined()
    }

    /// `https://host:port` of the node's DAPI, when it is an evonode with a
    /// routable service address.
    public var platformDAPIAddress: String? {
        guard let port = platformHTTPPort, let host = serviceHost else { return nil }
        return "https://\(host):\(port)"
    }

    /// Host part of `serviceAddress` (IPv6 keeps its brackets).
    public var serviceHost: String? {
        guard let address = serviceAddress else { return nil }
        if address.hasPrefix("[") {
            guard let close = address.firstIndex(of: "]") else { return nil }
            return String(address[...close])
        }
        guard let colon = address.lastIndex(of: ":") else { return address }
        return String(address[..<colon])
    }
}

public struct MasternodeLocateResult: Sendable {
    /// In the list's order; one entry per distinct proTxHash. Empty means
    /// "nothing on the list by that locator".
    public let matches: [MasternodeLocateMatch]
    public let platformLookup: MasternodePlatformLookup
    /// Reason when `platformLookup == .unavailable`.
    public let platformError: String?
}

/// Result of checking a key against a role.
public enum MasternodeKeyVerification: UInt8, Sendable {
    case matches = 0
    case doesNotMatch = 1
    /// The reference for this role isn't known (e.g. the owner key hash of a
    /// node that isn't one of this wallet's and whose registration details
    /// haven't been fetched). Not a pass.
    case unverifiable = 2
}

extension PlatformWalletManager {
    /// Find the masternode(s) `text` names — an IP (`1.2.3.4`,
    /// `1.2.3.4:9999`, a DAPI URL), a proTxHash (display hex as explorers
    /// print it), or a private key (owner / voting / payout WIF or hex,
    /// operator BLS hex, Tenderdash node key in dashmate's base64 or hex).
    /// For a key each match says which role(s) it fills, so the host can
    /// pre-fill that key field.
    ///
    /// `searchPlatform` additionally asks Platform for the owner / payout
    /// roles of a pasted secp256k1 key (one `getIdentityByNonUniquePublicKeyHash`
    /// per key — it reveals which key hash the user holds to DAPI, hence
    /// opt-in). The result's `platformLookup` reports that step; local
    /// matches stand either way.
    ///
    /// Throws `PlatformWalletError.invalidParameter` with a user-facing
    /// message when the text can't be read (empty, unrecognized, a WIF for
    /// the other network, a corrupt node key) and
    /// `.masternodeListUnavailable` before masternode sync completes. Runs
    /// the FFI — which may round-trip to Platform — on a detached task.
    public func locateMasternode(
        _ text: String,
        searchPlatform: Bool = false
    ) async throws -> MasternodeLocateResult {
        guard isConfigured, handle != NULL_HANDLE else {
            throw PlatformWalletError.invalidParameter("Manager not configured")
        }
        let handle = self.handle
        return try await Task.detached(priority: .userInitiated) { () -> MasternodeLocateResult in
            var outMatches: UnsafePointer<MasternodeLocateMatchFFI>?
            var outCount: UInt = 0
            var outLookup: UInt8 = 0
            var outError: UnsafeMutablePointer<CChar>?
            let ffiResult = text.withCString { cText in
                platform_wallet_manager_locate_masternode(
                    handle, cText, searchPlatform,
                    &outMatches, &outCount, &outLookup, &outError
                )
            }
            let result = PlatformWalletResult(ffiResult)
            guard result.isSuccess else {
                throw PlatformWalletError(result: result)
            }
            defer {
                if let entries = outMatches, outCount > 0 {
                    platform_wallet_manager_free_masternode_matches(
                        UnsafeMutablePointer(mutating: entries), outCount)
                }
                if let error = outError {
                    platform_wallet_string_free(error)
                }
            }
            let platformError = outError.map { String(cString: $0) }
            let lookup = MasternodePlatformLookup(rawValue: outLookup) ?? .unavailable
            guard let entries = outMatches, outCount > 0 else {
                return MasternodeLocateResult(
                    matches: [], platformLookup: lookup, platformError: platformError)
            }
            let matches = (0..<Int(outCount)).map { i -> MasternodeLocateMatch in
                var entry = entries[i]
                return MasternodeLocateMatch(
                    proTxHash: withUnsafeBytes(of: &entry.pro_tx_hash) { Data($0) },
                    serviceAddress: entry.service_address.map { String(cString: $0) },
                    platformHTTPPort: entry.has_platform_http_port ? entry.platform_http_port : nil,
                    operatorPublicKey: withUnsafeBytes(of: &entry.operator_public_key) { Data($0) },
                    votingKeyId: withUnsafeBytes(of: &entry.voting_key_id) { Data($0) },
                    platformNodeId: entry.has_platform_node_id
                        ? withUnsafeBytes(of: &entry.platform_node_id) { Data($0) }
                        : nil,
                    isValid: entry.is_valid,
                    isEvonode: entry.is_evonode,
                    matchedBy: MasternodeLocatorMatchKind(rawValue: entry.matched_by) ?? .proTxHash,
                    matchedKeys: MasternodeKeyRole.roles(fromMask: entry.matched_key_roles),
                    inWalletId: entry.in_wallet
                        ? withUnsafeBytes(of: &entry.wallet_id) { Data($0) }
                        : nil
                )
            }
            return MasternodeLocateResult(
                matches: matches, platformLookup: lookup, platformError: platformError)
        }.value
    }

    /// Check `key` against the `role` key of masternode `proTxHash` (32
    /// WIRE-order bytes). The reference is the list entry (voting / operator
    /// / platform node) merged with the owning wallet's record (owner /
    /// payout) when the node is one of a loaded wallet's masternodes.
    /// `.unverifiable` means the reference for that role isn't known — it is
    /// NOT a pass. Throws `.invalidParameter` when `key` isn't a key of the
    /// role's curve (or is a WIF for the other network) and `.notFound` when
    /// neither the list nor any wallet knows the masternode. Local; no
    /// network.
    public func verifyMasternodeKey(
        proTxHash: Data,
        role: MasternodeKeyRole,
        key: String
    ) throws -> MasternodeKeyVerification {
        guard isConfigured, handle != NULL_HANDLE, proTxHash.count == 32 else {
            throw PlatformWalletError.invalidParameter(
                "Manager not configured, or proTxHash not 32 bytes")
        }
        var out: UInt8 = MasternodeKeyVerification.unverifiable.rawValue
        let ffiResult = proTxHash.withUnsafeBytes { (raw: UnsafeRawBufferPointer) -> PlatformWalletFFIResult in
            key.withCString { cKey in
                platform_wallet_manager_masternode_verify_key(
                    handle,
                    raw.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    role.rawValue,
                    cKey,
                    &out
                )
            }
        }
        let result = PlatformWalletResult(ffiResult)
        guard result.isSuccess else {
            throw PlatformWalletError(result: result)
        }
        return MasternodeKeyVerification(rawValue: out) ?? .unverifiable
    }
}
