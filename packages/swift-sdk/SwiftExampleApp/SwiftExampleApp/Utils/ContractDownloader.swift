import Foundation
import SwiftData
import SwiftDashSDK

/// Errors surfaced by `ContractDownloader.downloadAndPersistContract`.
/// Distinct cases let callers (e.g. the contracts search bar) decide
/// whether to fall back to a different lookup strategy — e.g. a
/// "not found" on a direct contract fetch can pivot into a
/// `getTokenContractInfo` resolution attempt.
public enum ContractDownloadError: Error, LocalizedError {
    /// The fetch returned an empty result. The error message text is
    /// the original FFI message so callers can match on substrings
    /// like "Data contract not found" if they need to distinguish
    /// this case from other failures.
    case notFound(String)
    /// Any other failure path: FFI error, JSON parse failure, missing
    /// JSON payload, persistence error, etc. Message is human readable.
    case fetchFailed(String)
    /// The user-supplied id couldn't be converted into 32 bytes via
    /// any of the supported encodings (base58 / hex). Distinct from
    /// `notFound` because it never reached the network.
    case invalidContractId

    public var errorDescription: String? {
        switch self {
        case .notFound(let msg): return msg
        case .fetchFailed(let msg): return msg
        case .invalidContractId: return "Invalid contract ID"
        }
    }
}

/// Result of a successful download + persist. Surfaces both the
/// SwiftData row and a flag indicating whether the contract was
/// already saved before this call (so callers can avoid showing
/// "Saved!" toasts for an idempotent re-download).
public struct ContractDownloadResult {
    public let contract: PersistentDataContract
    /// `true` when the contract id was already in the local store
    /// before this call ran. The persisted row is returned either way
    /// — if `alreadyExisted` is true, `contract` is the existing row,
    /// not a new one.
    public let alreadyExisted: Bool
}

/// Shared download + parse + persist pipeline for data contracts.
///
/// This is the single Swift-side bridge for the "fetch a contract by
/// id, write it into the local SwiftData store, and register it in
/// the SDK's trusted-context cache" flow. Both
/// `LoadDataContractView` (the manual-paste sheet) and the new
/// search bar in `ContractsTabView` route through here so the steps
/// stay in one place:
///
/// 1. Fetch the contract via `dash_sdk_data_contract_fetch_with_serialization`
///    (JSON + binary in one round trip).
/// 2. Free the FFI error / contract handle on every exit path.
/// 3. Parse the JSON dictionary.
/// 4. Capture the binary serialization and register it with the
///    SDK's trusted-context provider via `sdk.addContractToContext`,
///    so subsequent state-transition validation doesn't have to
///    re-fetch it.
/// 5. If the contract id is already in the local SwiftData store,
///    update `lastAccessedAt` and return the existing row with
///    `alreadyExisted == true` — don't insert a duplicate.
/// 6. Otherwise build a `PersistentDataContract`, attach the binary
///    blob, run `DataContractParser.parseDataContract` to extract
///    tokens + document types, and save.
///
/// The function is deliberately a free `func` (not a class method)
/// because it owns no mutable state — every dependency is injected.
/// All `ModelContext` writes happen on the calling actor; callers
/// invoking this from a background `Task` should hop to the main
/// actor before touching the same `modelContext` afterwards (this
/// matches the existing pattern in `LocalDataContractsView`).
@MainActor
public enum ContractDownloader {
    /// Download a data contract by base58 id, persist it locally, and
    /// register its binary serialization with the SDK trusted context.
    ///
    /// - Parameters:
    ///   - contractId: Base58-encoded 32-byte contract id. The caller
    ///     is responsible for trimming whitespace / validating shape
    ///     before this call — the function will surface a fetch error
    ///     for an obviously-malformed id rather than try to repair it.
    ///   - suggestedName: Optional human-readable name to use when
    ///     creating a new `PersistentDataContract` row. Ignored if
    ///     the contract is already saved. If nil/empty, the function
    ///     derives a sensible default from the contract's tokens or
    ///     document types.
    ///   - sdk: The platform SDK handle for the active network.
    ///   - modelContext: SwiftData context used for both the
    ///     duplicate-check `FetchDescriptor` and the `insert`/`save`.
    ///   - network: Active app network. Stored on the new row so the
    ///     network-scoped `@Query` predicates resolve correctly.
    ///   - existingContracts: Optional pre-fetched array of saved
    ///     contracts. When the caller already holds a `@Query` over
    ///     `PersistentDataContract` (the modal sheet does), passing
    ///     it in avoids a redundant `FetchDescriptor` round trip.
    ///     When nil, the function runs its own fetch.
    /// - Returns: A `ContractDownloadResult` carrying the persisted
    ///   row plus whether it was newly created or already existed.
    /// - Throws: `ContractDownloadError.notFound` when the network
    ///   reports the contract doesn't exist, or
    ///   `ContractDownloadError.fetchFailed` for any other failure.
    public static func downloadAndPersistContract(
        contractId: String,
        suggestedName: String?,
        sdk: SDK,
        modelContext: ModelContext,
        network: AppNetwork,
        existingContracts: [PersistentDataContract]? = nil
    ) async throws -> ContractDownloadResult {
        let trimmedId = contractId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else {
            throw ContractDownloadError.invalidContractId
        }

        guard let handle = sdk.handle else {
            throw ContractDownloadError.fetchFailed("SDK not initialized")
        }

        // Fetch JSON + binary in one round trip. The
        // `dash_sdk_data_contract_fetch_with_serialization` FFI owns
        // the returned pointers, so we must free the error and
        // contract handle on every exit path.
        let result = trimmedId.withCString { idCStr in
            dash_sdk_data_contract_fetch_with_serialization(handle, idCStr, true, true)
        }

        if let error = result.error {
            let message = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            // The FFI doesn't expose a structured "not found" code, so
            // we have to substring-match on the message. This is the
            // same pattern the modal sheet was using before this
            // refactor, just centralized.
            if message.contains("Data contract not found")
                || message.contains("not found") {
                throw ContractDownloadError.notFound(message)
            }
            throw ContractDownloadError.fetchFailed(
                "Failed to fetch data contract: \(message)"
            )
        }

        // Pull JSON and binary out of the result before scheduling
        // cleanup of the contract handle.
        guard let jsonPtr = result.json_string else {
            if result.contract_handle != nil {
                dash_sdk_data_contract_destroy(result.contract_handle)
            }
            throw ContractDownloadError.fetchFailed(
                "No JSON data returned from contract fetch"
            )
        }
        let jsonString = String(cString: jsonPtr)

        var binaryData: Data? = nil
        if result.serialized_data != nil && result.serialized_data_len > 0 {
            binaryData = Data(
                bytes: result.serialized_data,
                count: Int(result.serialized_data_len)
            )
        }

        defer {
            if result.contract_handle != nil {
                dash_sdk_data_contract_destroy(result.contract_handle)
            }
        }

        // Parse the contract JSON into a dictionary the parser
        // understands.
        guard let jsonData = jsonString.data(using: .utf8),
              let contractData = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
            throw ContractDownloadError.fetchFailed(
                "Failed to parse contract JSON"
            )
        }

        // Register the binary serialization with the trusted-context
        // provider so future state transitions don't refetch.
        // Failures here are non-fatal — log and keep going, matching
        // the pre-refactor behavior in `LoadDataContractView`.
        if let binaryData = binaryData,
           let idString = contractData["id"] as? String {
            do {
                try sdk.addContractToContext(
                    contractId: idString,
                    binaryData: binaryData
                )
            } catch {
                print("⚠️ Failed to add contract to trusted context: \(error)")
            }
        }

        // Resolve the contract id bytes. Prefer the id reported in the
        // JSON (it's the canonical form), fall back to the input
        // string if for some reason the JSON omits it.
        let contractIdData: Data
        if let idString = contractData["id"] as? String,
           let idData = Data.identifier(fromBase58: idString) ?? Data(hexString: idString) {
            contractIdData = idData
        } else if let idData = Data.identifier(fromBase58: trimmedId) {
            contractIdData = idData
        } else {
            throw ContractDownloadError.fetchFailed(
                "Could not extract contract ID from response"
            )
        }

        // Duplicate check. Prefer the caller-supplied list when
        // available (the modal sheet has a `@Query` already); fall
        // back to a SwiftData fetch when not.
        let preExisting: PersistentDataContract?
        if let existingContracts = existingContracts {
            preExisting = existingContracts.first(where: { $0.id == contractIdData })
        } else {
            let descriptor = FetchDescriptor<PersistentDataContract>(
                predicate: #Predicate { $0.id == contractIdData }
            )
            preExisting = try? modelContext.fetch(descriptor).first
        }

        if let preExisting = preExisting {
            // Touch lastAccessedAt so the contract floats to the top of
            // the recents list, but don't reinsert.
            preExisting.updateLastAccessed()
            // Run the linker on the existing row too — the owner
            // identity might have appeared in the local store since
            // the last download. Idempotent when nothing has changed.
            ContractIdentityLinker.linkContractToOwner(
                contract: preExisting,
                modelContext: modelContext
            )
            try? modelContext.save()
            return ContractDownloadResult(
                contract: preExisting,
                alreadyExisted: true
            )
        }

        // Derive a display name when the caller didn't supply one.
        // Same heuristics as the pre-refactor modal: token-only
        // contracts get "<TokenName> Token Contract", contracts with
        // documents get "Contract with <firstDocType>", and anything
        // else gets a truncated-id fallback.
        let resolvedName: String = {
            if let supplied = suggestedName?.trimmingCharacters(in: .whitespacesAndNewlines),
               !supplied.isEmpty {
                return supplied
            }
            let documents = contractData["documents"] as? [String: Any]
                ?? contractData["documentSchemas"] as? [String: Any]
                ?? [:]
            let tokens = contractData["tokens"] as? [String: Any] ?? [:]

            if documents.isEmpty && tokens.count == 1,
               let tokenData = tokens.values.first as? [String: Any] {
                if let conventions = tokenData["conventions"] as? [String: Any],
                   let localizations = conventions["localizations"] as? [String: Any],
                   let enLocalization = localizations["en"] as? [String: Any],
                   let singularForm = enLocalization["singularForm"] as? String {
                    return "\(singularForm) Token Contract"
                }
                if let desc = tokenData["description"] as? String {
                    return "\(desc) Token Contract"
                }
                if let nm = tokenData["name"] as? String {
                    return "\(nm) Token Contract"
                }
                return "Token Contract"
            }
            if let firstDocType = documents.keys.first {
                return "Contract with \(firstDocType)"
            }
            return "Contract \(trimmedId.prefix(8))..."
        }()

        let persistent = PersistentDataContract(
            id: contractIdData,
            name: resolvedName,
            serializedContract: jsonData,
            network: network
        )
        persistent.binarySerialization = binaryData

        modelContext.insert(persistent)
        try modelContext.save()

        // Tokens + document-type rows hang off the contract via
        // cascade-delete relationships; the parser populates them
        // straight on the SwiftData row.
        try DataContractParser.parseDataContract(
            contractData: contractData,
            contractId: contractIdData,
            modelContext: modelContext
        )

        // Now that the parser has stamped `ownerId` on the row,
        // attempt to link it to a local `PersistentIdentity`. No-op
        // if the owner isn't in the local store — most contracts in
        // the cache are owned by identities the user doesn't hold.
        ContractIdentityLinker.linkContractToOwner(
            contract: persistent,
            modelContext: modelContext
        )

        try modelContext.save()

        return ContractDownloadResult(
            contract: persistent,
            alreadyExisted: false
        )
    }
}
