import Foundation

// MARK: - Contract fee claims (protocol version 14)
//
// The shapes `platform_wallet_contract_fee_claim` and
// `platform_wallet_contract_fee_claim_eligibility` write
// (`packages/rs-platform-wallet-ffi/src/contract_fee_claim.rs`): identifiers
// as base58 strings, credits and times as JSON integers. Rust decides every
// value here; these types only decode them.

/// One of a contract's two fee pots.
public enum ContractFeePot: String, Codable, Sendable, CaseIterable, Identifiable {
    /// The pot the contract owner claims.
    case owner
    /// The pot the contract's moderation team claims, for the whole team.
    case moderators

    public var id: String { rawValue }

    /// The `pot` byte `platform_wallet_contract_fee_claim` takes: 0 for the
    /// owner pot, 1 for the moderators pot (`CONTRACT_FEE_POT_OWNER` and
    /// `CONTRACT_FEE_POT_MODERATORS` in the FFI). Any other byte is refused
    /// there.
    public var ffiTag: UInt8 {
        switch self {
        case .owner: return 0
        case .moderators: return 1
        }
    }
}

/// Who a contract's moderators pot belonged to when it was read, which
/// decided who may claim it.
public enum ContractModeratorsPotTeam: String, Codable, Sendable {
    /// The contract declares no moderation: nobody claims its moderators pot.
    case noModeration
    /// The team the contract's moderation declaration names.
    case declared
    /// An elected contract with no seated charter: the interim team its
    /// declaration names, which may be nobody.
    case electedInterim
    /// An elected contract with a seated charter: the charter's leader and
    /// its active members.
    case electedSeated
}

/// Which identities of one wallet may claim which fee pot of a contract
/// (`ManagedPlatformWallet.contractFeeClaimEligibility(contractId:)`), as
/// Rust read it from the contract and, for an elected contract, from the
/// moderation charters contract.
///
/// It names who may sign a claim, nothing more: whether the pot holds enough
/// to pay everyone and whether it was already paid out this epoch are
/// Platform's checks at claim time.
public struct ContractFeeClaimEligibility: Codable, Sendable, Equatable {
    /// The contract asked about, base58.
    public let contractId: String
    /// The wallet's identities (base58) that may claim the owner pot: the
    /// contract owner, when this wallet holds it.
    public let ownerPotClaimants: [String]
    /// The wallet's identities (base58) that may claim the moderators pot.
    public let moderatorsPotClaimants: [String]
    /// Who the moderators pot belongs to.
    public let moderatorsPotTeam: ContractModeratorsPotTeam

    /// The identities (base58) that may claim `pot`, as Rust listed them.
    public func claimants(of pot: ContractFeePot) -> [String] {
        switch pot {
        case .owner: return ownerPotClaimants
        case .moderators: return moderatorsPotClaimants
        }
    }

    /// Decode the JSON `platform_wallet_contract_fee_claim_eligibility`
    /// writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> ContractFeeClaimEligibility {
        try ContractFeeClaimJSON.decode(ContractFeeClaimEligibility.self, json, what: "contract fee claim eligibility")
    }
}

/// The balance of one identity after a claim, as the claim's proof shows it.
public struct ContractFeeClaimBalance: Codable, Sendable, Equatable {
    /// The identity, base58.
    public let identityId: String
    /// Its balance after the claim, in credits.
    public let balance: UInt64
}

/// A fee pot claim as its proof shows it
/// (`ManagedPlatformWallet.claimContractFees(claimantIdentityId:contractId:pot:signer:)`).
public struct ClaimedContractFees: Codable, Sendable, Equatable {
    /// The contract whose pot was paid out, base58.
    public let contractId: String
    /// The pot that was paid out.
    public let pot: ContractFeePot
    /// The pot as the claim left it, in the shape of the fee pot read: its
    /// credits are what the split left over plus any fee collected since, and
    /// its last claim is this claim unless the pot was claimed again since.
    public let potAfterClaim: ContractFeePotState
    /// The balance after the claim of every identity the proof shows: every
    /// recipient the contract names, or, for a claim by a member of an
    /// elected contract's seated team, the claimant alone. In identity id
    /// order.
    public let balances: [ContractFeeClaimBalance]

    /// Decode the JSON `platform_wallet_contract_fee_claim` writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> ClaimedContractFees {
        try ContractFeeClaimJSON.decode(ClaimedContractFees.self, json, what: "contract fee claim")
    }
}

/// Decodes the contract fee claim JSON, for the wrappers and the hermetic
/// decoder tests alike.
enum ContractFeeClaimJSON {
    static func decode<T: Decodable>(_ type: T.Type, _ json: String, what: String) throws -> T {
        do {
            return try JSONDecoder().decode(type, from: Data(json.utf8))
        } catch {
            throw PlatformWalletError.deserialization("Failed to decode the \(what): \(error)")
        }
    }
}
