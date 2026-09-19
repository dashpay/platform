import Foundation

/// Local confirmation history. This is a change warning, never an authorization
/// to pay: `sendShieldedTip` still verifies the confirmed destination on Platform.
public final class ShieldedTipRecipientHistory {
    private let defaults: UserDefaults

    public init(defaults: UserDefaults = .standard) {
        self.defaults = defaults
    }

    public func hasChanged(network: Network, walletId: Data, username: String,
                           recipient: ShieldedTipRecipient) -> Bool {
        guard let previous = defaults.data(forKey: key(network: network, walletId: walletId, username: username)) else {
            return false
        }
        return previous != recipient.identityId + recipient.address
    }

    /// Call only after the user explicitly confirms, including any change warning.
    public func confirm(network: Network, walletId: Data, username: String,
                        recipient: ShieldedTipRecipient) {
        defaults.set(recipient.identityId + recipient.address,
                     forKey: key(network: network, walletId: walletId, username: username))
    }

    private func key(network: Network, walletId: Data, username: String) -> String {
        let name = PersistentDPNSName.normalize(username.trimmingCharacters(in: .whitespacesAndNewlines))
        let canonical = name.hasSuffix(".dash") ? name : name + ".dash"
        return "dashpay.tipRecipient.\(network.rawValue).\(walletId.toBase58String()).\(canonical)"
    }
}
