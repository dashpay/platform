import Foundation
import SwiftDashSDK

/// How a contact request send or accept that threw ended, for the DashPay
/// views. Follows `SendViewModel`: a shielded spend whose result could not be
/// confirmed may already be on chain (Rust keeps its notes reserved), so the
/// view keeps it pending and never offers a fresh submission; the next sync
/// settles it. Every other error is an ordinary failure the user may retry.
enum ContactRequestSubmissionOutcome: Equatable {
    /// The private send or reply may have gone through: do not submit again.
    case mayHaveGoneThrough
    /// It failed; the message is the error's.
    case failed(String)

    static let mayHaveGoneThroughMessage = "This may have gone through. It shows up "
        + "after the next sync confirms it; do not submit it again."

    static func of(_ error: Error) -> ContactRequestSubmissionOutcome {
        if let walletError = error as? PlatformWalletError,
           case .shieldedSpendUnconfirmed = walletError {
            return .mayHaveGoneThrough
        }
        return .failed(error.localizedDescription)
    }
}

/// What the request list's private-store refresh tells the user. A network
/// that does not serve the store yet (before protocol version 15) is expected
/// and stays quiet; an operational failure, or requests read but not saved,
/// is shown so the user knows to refresh again.
enum PrivateContactRefreshNotice {
    /// The notice a pass that read the store leaves, or nil when every
    /// identity's requests reached disk.
    static func notice(unpersistedIdentityCount: UInt32) -> String? {
        guard unpersistedIdentityCount > 0 else { return nil }
        return "Private requests for \(unpersistedIdentityCount) "
            + (unpersistedIdentityCount == 1 ? "identity were" : "identities were")
            + " read but not saved. Pull to refresh again."
    }

    /// The notice a pass that failed leaves, or nil when the network does not
    /// serve the private store yet.
    static func notice(for error: Error) -> String? {
        if let walletError = error as? PlatformWalletError,
           case .protocolVersionNotActive = walletError {
            return nil
        }
        return "Private requests could not be read: \(error.localizedDescription)"
    }
}
