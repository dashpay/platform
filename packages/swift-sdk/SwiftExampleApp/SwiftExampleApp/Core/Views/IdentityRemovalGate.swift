import Foundation
import Observation
import SwiftDashSDK

/// What one identity removal did.
enum IdentityRemovalOutcome<Deleted: Sendable>: Sendable {
    /// The check couldn't run; nothing was deleted.
    case checkFailed(any Error)
    /// Rust didn't say `.notOwned`; nothing was deleted.
    case kept(ManagedPlatformWallet.IdentityWalletOwnership)
    /// The identity's row was already gone; nothing was left to delete.
    case alreadyRemoved
    /// The row couldn't be read; nothing was deleted.
    case lookupFailed(any Error)
    /// The delete ran and returned this.
    case deleted(Deleted)
    /// The delete threw.
    case deleteFailed(any Error)
}

/// Runs at most one removal per identity at a time: Rust's ownership
/// check (a Platform refresh), then, only when it says `.notOwned`, the
/// delete of the identity's row as the store holds it after the check.
///
/// The guard is taken before the removal's task starts and released when
/// it ends, on every path, so a second confirmation while the check is
/// pending does nothing. Swift only branches on the `.notOwned` Rust
/// returns. The steps are closures so a test can delay the check and
/// count the deletes.
@MainActor
@Observable
final class IdentityRemovalGate {
    /// Identities with a removal in progress.
    private(set) var inFlight: Set<Data> = []

    func isInFlight(_ identityId: Data) -> Bool {
        inFlight.contains(identityId)
    }

    /// Start a removal of `identityId`, or return nil while another one
    /// is in progress.
    ///
    /// - Parameters:
    ///   - check: Rust's ownership verdict for the identity.
    ///   - liveRow: the identity's row as the store holds it now, read
    ///     after the check; nil once it is gone.
    ///   - delete: removes that row.
    @discardableResult
    func start<Row, Deleted: Sendable>(
        _ identityId: Data,
        check: @escaping @MainActor () async throws -> ManagedPlatformWallet.IdentityWalletOwnership,
        liveRow: @escaping @MainActor () throws -> Row?,
        delete: @escaping @MainActor (Row) throws -> Deleted
    ) -> Task<IdentityRemovalOutcome<Deleted>, Never>? {
        guard inFlight.insert(identityId).inserted else { return nil }
        return Task { @MainActor in
            defer { self.inFlight.remove(identityId) }
            let ownership: ManagedPlatformWallet.IdentityWalletOwnership
            do {
                ownership = try await check()
            } catch {
                return .checkFailed(error)
            }
            guard ownership == .notOwned else { return .kept(ownership) }
            let row: Row?
            do {
                row = try liveRow()
            } catch {
                return .lookupFailed(error)
            }
            guard let row else { return .alreadyRemoved }
            do {
                return .deleted(try delete(row))
            } catch {
                return .deleteFailed(error)
            }
        }
    }
}
