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
/// The consensus error's numeric code (40722) does not survive the trip: the
/// FFI hands Swift a `PlatformWalletError` carrying a rendered message, and
/// rs-drive-abci puts the consensus error's `Display` text in it. So the
/// match is on that text, which rs-dpp spells "identity '<id>' already
/// claimed the once-per-identity distribution of token '<id>' at <ms>". The
/// type name is checked too, in case a path renders the variant rather than
/// its message.
///
/// A miss is not fatal in either direction: missing the error only means the
/// kind stays offered until the next attempt, and the phrase is specific
/// enough that a false positive would take a deliberately crafted message.
enum OncePerIdentityClaimRejection {
    private static let messageSignals = [
        "already claimed the once-per-identity distribution",
        "tokenonceperidentitydistributionalreadyclaimederror"
    ]

    static func isAlreadyClaimed(_ error: Error) -> Bool {
        let message = error.localizedDescription.lowercased()
        return messageSignals.contains { message.contains($0) }
    }
}
