import Foundation
import SwiftUI
import SwiftDashSDK

/// Device-local DPNS-label hint captured when a contact is added by
/// username search.
///
/// Alias, note and hidden now live on the `PersistentDashpayContactRequest`
/// rows written through `setDashPayContactInfo`; this store keeps only the
/// add-time hint, which has no `contactInfo` counterpart.
///
/// Keys are scoped by `(network, owner identity, contact identity)`
/// so two owner identities (or two networks) never share a contact's
/// hint. The published `version` counter makes SwiftUI views that
/// read through this store re-render after a write — UserDefaults
/// alone doesn't participate in SwiftUI invalidation for computed
/// reads.
@MainActor
final class DashPayContactMetaStore: ObservableObject {
    /// Bumped on every write so observing views recompute reads.
    @Published private(set) var version = 0

    private let defaults = UserDefaults.standard

    // MARK: - DPNS hint

    /// DPNS label observed when the contact was added via username
    /// search. Display-precedence fallback only — contacts' DPNS
    /// labels aren't persisted in SwiftData (only managed identities'
    /// are), so this hint is "the data available" for the M2 rows.
    func dpnsHint(network: Network, owner: Data, contact: Data) -> String? {
        nonEmpty(defaults.string(forKey: key("dpnsHint", network, owner, contact)))
    }

    func setDpnsHint(_ name: String?, network: Network, owner: Data, contact: Data) {
        write(nonEmpty(name), forKey: key("dpnsHint", network, owner, contact))
    }

    // MARK: - Helpers

    private func key(_ field: String, _ network: Network, _ owner: Data, _ contact: Data) -> String {
        "dashpay.meta.\(field).\(network.rawValue).\(owner.toHexString()).\(contact.toHexString())"
    }

    private func write(_ value: String?, forKey key: String) {
        if let value {
            defaults.set(value, forKey: key)
        } else {
            defaults.removeObject(forKey: key)
        }
        version += 1
    }

    private func nonEmpty(_ value: String?) -> String? {
        guard let trimmed = value?.trimmingCharacters(in: .whitespacesAndNewlines),
              !trimmed.isEmpty else {
            return nil
        }
        return trimmed
    }
}

// MARK: - Display-name precedence

/// Resolve the display precedence for a DashPay contact:
/// local alias → DashPay profile `displayName` → DPNS label →
/// truncated hex id. Every input but the id is optional; empty
/// strings count as absent.
func dashPayContactDisplayName(
    contactId: Data,
    alias: String?,
    profileDisplayName: String?,
    dpnsLabel: String?
) -> String {
    for candidate in [alias, profileDisplayName, dpnsLabel] {
        if let trimmed = candidate?.trimmingCharacters(in: .whitespacesAndNewlines),
           !trimmed.isEmpty {
            return trimmed
        }
    }
    return String(contactId.toHexString().prefix(12)) + "…"
}

// MARK: - Cache-only profile read

/// Cache-only DashPay profile read off a loaded wallet handle (no
/// network). Prefers the per-contact `getContactProfile` entry, then
/// falls back to the global `getDashPayProfile` for the same identity.
/// Misses are common — a contact's profile only populates after a
/// profile sync has seen them — so both throwing calls degrade to nil.
func dashPayCachedProfile(
    wallet: ManagedPlatformWallet,
    ownerIdentityId: Data,
    contactId: Data
) -> DashPayProfile? {
    (try? wallet.getContactProfile(
        ownerIdentityId: ownerIdentityId,
        contactIdentityId: contactId
    )) ?? (try? wallet.getDashPayProfile(identityId: contactId)) ?? nil
}

// MARK: - Txid display order

/// Hex-encode a raw 32-byte txid in canonical (reversed) display
/// order, matching `PersistentTransaction.txidHex` and the tx list /
/// payment history. The FFI hands back wire/internal byte order, so a
/// bare `toHexString()` reads reversed from block explorers — this
/// flip lines the toasted id up with everything else the user sees.
func txidDisplayHex(_ txid: Data) -> String {
    txid.reversed().map { String(format: "%02x", $0) }.joined()
}

// MARK: - Avatar

/// Shared avatar bubble: AsyncImage when the profile has an
/// `avatarUrl`, initial-circle fallback otherwise (§6.2). The
/// initial comes from the resolved display name.
struct DashPayAvatarView: View {
    let avatarUrl: String?
    let displayName: String
    var size: CGFloat = 40

    var body: some View {
        if let url = avatarUrl.flatMap({ URL(string: $0) }) {
            AsyncImage(url: url) { phase in
                if let image = phase.image {
                    image
                        .resizable()
                        .aspectRatio(contentMode: .fill)
                } else {
                    initialCircle
                }
            }
            .frame(width: size, height: size)
            .clipShape(Circle())
        } else {
            initialCircle
                .frame(width: size, height: size)
        }
    }

    private var initialCircle: some View {
        Circle()
            .fill(Color.blue.opacity(0.2))
            .overlay(
                Text(displayName.prefix(1).uppercased())
                    .font(size > 50 ? .title : .headline)
                    .foregroundColor(.blue)
            )
    }
}
