import Foundation
import SwiftDashSDK

/// "Has this identity already taken its single once-per-identity claim on
/// this token?"
///
/// A once-per-identity distribution pays every identity a fixed amount
/// exactly once, so eligibility is universal right up to the moment the
/// identity claims, after which Drive rejects the next claim with
/// `TokenOncePerIdentityDistributionAlreadyClaimedError` (consensus state
/// error 40722) and the user pays the fee for the rejection. There is no
/// DAPI query for "has identity X claimed" yet, so the only thing this app
/// can do is remember the claims it saw itself.
///
/// Split from `OncePerIdentityClaimRecording` so the permission resolver can
/// take the read side alone and stay pure and unit-testable.
protocol OncePerIdentityClaimReading {
    func hasClaimed(token: PersistentToken, identity: PersistentIdentity) -> Bool
}

/// The write side, held by the claim form.
protocol OncePerIdentityClaimRecording: OncePerIdentityClaimReading {
    func recordClaim(token: PersistentToken, identity: PersistentIdentity)
}

/// `UserDefaults`-backed record of once-per-identity claims this app saw
/// succeed, plus the ones Drive told us had already happened.
///
/// Deliberately not SwiftData: `DashSchemaV5` is frozen, and a new stored
/// property or model would cost a schema version for what is a local hint,
/// not protocol state. The hint is one-way, set and never cleared, which is
/// safe because the fact it caches cannot become false again: a spent claim
/// stays spent, and identity ids are not reused.
///
/// It is a hint, not an authority. An identity that claimed on another
/// device is absent from it, and that claim is still caught the expensive
/// way, by Drive rejecting the attempt.
///
/// `@unchecked Sendable`: the only stored value is a `UserDefaults`, whose
/// accessors are thread-safe.
final class OncePerIdentityClaimStore: OncePerIdentityClaimRecording, @unchecked Sendable {
    static let shared = OncePerIdentityClaimStore()

    private let defaults: UserDefaults

    init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    func hasClaimed(token: PersistentToken, identity: PersistentIdentity) -> Bool {
        defaults.bool(forKey: Self.key(token: token, identity: identity))
    }

    func recordClaim(token: PersistentToken, identity: PersistentIdentity) {
        defaults.set(true, forKey: Self.key(token: token, identity: identity))
    }

    /// Keyed by network as well as token and identity: the same identity id
    /// can exist on testnet and on a devnet with different claim histories,
    /// and the app switches networks in place.
    static func key(token: PersistentToken, identity: PersistentIdentity) -> String {
        [
            "tokenOncePerIdentityClaimed",
            String(identity.networkRaw),
            token.contractIdBase58,
            String(token.position),
            identity.identityIdBase58
        ].joined(separator: ".")
    }
}

/// Recognises Drive's "this identity already claimed" rejection in the error
/// a claim submission throws.
///
/// A claim rejected by Platform arrives as a `PlatformWalletError` carrying
/// Platform's own verdict: `consensusError` holds the rs-dpp consensus code
/// and family that Rust read off the rejection and passed across the FFI
/// boundary. The already-claimed rejection is state error 40722, and rs-dpp
/// codes are unique across the families, so recognising it is an equality
/// check on that one number.
///
/// Nothing here reads the message. Claim errors quote token amounts and
/// millisecond timestamps, so a claim time such as 1758140722000 contains the
/// digits of the code, and the prose around them is rs-dpp's to reword.
///
/// A miss is not fatal in either direction: missing the rejection only means
/// the kind stays offered until the next attempt, and a wrong hit would take
/// a different rejection arriving under this code.
enum OncePerIdentityClaimRejection {
    /// Consensus code of `TokenOncePerIdentityDistributionAlreadyClaimedError`
    /// (rs-dpp `errors/consensus/codes.rs`), the state error Drive returns
    /// for a second claim by the same identity.
    static let alreadyClaimedConsensusCode: UInt32 = 40722

    static func isAlreadyClaimed(_ error: Error) -> Bool {
        guard let walletError = error as? PlatformWalletError else { return false }
        return walletError.consensusError?.code == alreadyClaimedConsensusCode
    }
}
