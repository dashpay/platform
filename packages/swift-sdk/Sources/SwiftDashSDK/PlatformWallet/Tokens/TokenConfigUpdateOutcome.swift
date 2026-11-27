import Foundation

/// What `ManagedPlatformWallet.tokenUpdateConfig` did, as Rust decided it
/// (`platform_wallet::TokenConfigUpdateOutcome`).
///
/// Platform applies a configuration change to the contract and raises its
/// version, but the update's proof does not carry the contract. So after an
/// applied update Rust fetches the contract again, refusing an answer from
/// before the block that confirmed the update, and hands it back here for the
/// host to store.
public enum TokenConfigUpdateOutcome: Sendable, Equatable {
    /// A group action still waiting for signatures: the contract is unchanged.
    case pending

    /// Applied. `contractJSON` is the proof-verified contract the update
    /// produced, as JSON in the shape the contract fetch returns, and
    /// `serialization` its platform serialization.
    case applied(contractJSON: Data, serialization: Data?)

    /// Applied, but no fetch satisfied the confirmation-height and version
    /// checks (`reason` says why): a stored copy still has the old
    /// configuration. `minimumVersion` is a lower bound based on the version
    /// read before submission, not proof that a contract at that version
    /// includes this update: another edit may have landed first.
    case appliedNotRefreshed(minimumVersion: UInt32, reason: String)

    /// Decode the JSON `platform_wallet_token_update_config_and_refresh`
    /// writes.
    ///
    /// - Throws: `PlatformWalletError.deserialization` when `json` is not
    ///   that shape.
    public static func decode(fromJSON json: String) throws -> TokenConfigUpdateOutcome {
        guard let root = (try? JSONSerialization.jsonObject(with: Data(json.utf8))) as? [String: Any] else {
            throw PlatformWalletError.deserialization("The token config update outcome is not a JSON object")
        }
        guard let kind = root["outcome"] as? String else {
            throw PlatformWalletError.deserialization("The token config update outcome names no outcome")
        }
        switch kind {
        case "pending":
            return .pending
        case "applied":
            guard let contract = root["contract"] as? [String: Any],
                  let contractJSON = try? JSONSerialization.data(withJSONObject: contract) else {
                throw PlatformWalletError.deserialization("An applied token config update carries no contract")
            }
            var serialization: Data? = nil
            if let hex = root["serializationHex"] as? String {
                guard let bytes = Data(hexString: hex) else {
                    throw PlatformWalletError.deserialization(
                        "The contract serialization of an applied token config update is not hex"
                    )
                }
                serialization = bytes
            }
            return .applied(contractJSON: contractJSON, serialization: serialization)
        case "appliedNotRefreshed":
            guard let number = root["minimumVersion"] as? NSNumber,
                  let minimumVersion = UInt32(exactly: number) else {
                throw PlatformWalletError.deserialization(
                    "A token config update that was not refreshed carries no minimum version"
                )
            }
            return .appliedNotRefreshed(
                minimumVersion: minimumVersion,
                reason: root["reason"] as? String ?? ""
            )
        default:
            throw PlatformWalletError.deserialization("Unknown token config update outcome '\(kind)'")
        }
    }
}
