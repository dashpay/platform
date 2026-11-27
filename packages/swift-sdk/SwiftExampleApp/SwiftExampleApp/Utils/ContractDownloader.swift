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

/// Throwaway in-memory representation of a fetched contract used by
/// the search-result preview path. Carries everything the preview UI
/// renders, plus the raw JSON / binary buffers that the eventual
/// `downloadAndPersistContract` call needs if the user opts to save.
///
/// Intentionally not a SwiftData model — closing the preview drops it,
/// which is the whole point of the "view without saving" flow.
public struct ContractPreviewState: Identifiable {
    /// `Identifiable` conformance for `.sheet(item:)`. The base58 id
    /// is unique per contract.
    public var id: String { contractIdBase58 }
    /// Base58 form of the contract id, retained so the parent view
    /// can dedupe / look up the corresponding saved row.
    public let contractIdBase58: String
    /// The parsed contract row, owned by `container`. Hand this to
    /// `DataContractDetailsView` as its `contract` argument.
    public let contract: PersistentDataContract
    /// In-memory container backing `contract`. The caller MUST keep
    /// this alive for the lifetime of the preview sheet — dropping it
    /// invalidates the row.
    public let container: ModelContainer
    /// `AppState.sdkGeneration` of the SDK the contract was fetched
    /// through. The moderation hub opened from the preview is scoped
    /// to it, so after an SDK rebuild the hub asks the user to reopen
    /// rather than act on the new connection for a contract read from
    /// the old one.
    public let sdkGeneration: Int
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
        network: Network,
        existingContracts: [PersistentDataContract]? = nil
    ) async throws -> ContractDownloadResult {
        let trimmedId = contractId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else {
            throw ContractDownloadError.invalidContractId
        }

        let fetched = try fetchContract(contractId: trimmedId, sdk: sdk)
        let jsonData = fetched.jsonData
        let contractData = fetched.contractData
        let binaryData = fetched.binaryData

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
        let contractIdData = try resolveContractId(
            contractData: contractData,
            requestedId: trimmedId
        )

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

    /// Fetch a data contract for preview, parse it into a brand-new
    /// in-memory `ModelContainer`, and return the resulting row.
    /// The caller is expected to present `DataContractDetailsView`
    /// against this row so the entire detail UI (tokens, document
    /// types + indexes, groups) is reused as-is for previews.
    ///
    /// The in-memory container is returned alongside the row so the
    /// caller can keep it alive for the lifetime of the preview
    /// sheet — when the caller drops `ContractPreviewState`, the
    /// container deallocates and every preview-only row goes with
    /// it. No on-disk state is touched. The trusted-context
    /// registration that `downloadAndPersistContract` performs is
    /// intentionally skipped here so a previewed-but-not-saved
    /// contract never leaks into transition validation.
    /// `sdkGeneration` is `AppState.sdkGeneration` of `sdk`, kept on the
    /// returned preview.
    public static func previewContractInMemory(
        contractId: String,
        sdk: SDK,
        sdkGeneration: Int,
        network: Network
    ) async throws -> ContractPreviewState {
        let trimmedId = contractId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmedId.isEmpty else {
            throw ContractDownloadError.invalidContractId
        }

        let fetched = try fetchContract(contractId: trimmedId, sdk: sdk)
        let jsonData = fetched.jsonData
        let contractData = fetched.contractData
        let binaryData = fetched.binaryData

        let contractIdData = try resolveContractId(
            contractData: contractData,
            requestedId: trimmedId
        )
        let contractIdBase58 = contractIdData.toBase58String()

        // Stand up an ephemeral SwiftData store. Anything we insert
        // here disappears the moment the caller drops the returned
        // `ContractPreviewState`.
        let container = try DashModelContainer.createInMemory()
        let context = ModelContext(container)

        // Default-name heuristic: keep the same labels the persisted
        // row will carry once saved (token-only contracts get
        // "<TokenName> Token Contract", contracts with documents get
        // "Contract with <firstDocType>", etc.).
        let documents = contractData["documents"] as? [String: Any]
            ?? contractData["documentSchemas"] as? [String: Any]
            ?? [:]
        let tokensDict = contractData["tokens"] as? [String: Any] ?? [:]
        let resolvedName: String = {
            if documents.isEmpty && tokensDict.count == 1,
               let tokenData = tokensDict.values.first as? [String: Any] {
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
        context.insert(persistent)
        try context.save()

        // Run the same parser the persist path uses. It populates
        // tokens, document types (with indexes + properties), groups,
        // and the contract's owner / version / flags — everything
        // `DataContractDetailsView` and its drill-downs need.
        // Identity-linking is intentionally skipped: the in-memory
        // store has no identities, and even if one happened to share
        // a row id it would be wrong to mark a previewed contract
        // as locally-owned.
        try DataContractParser.parseDataContract(
            contractData: contractData,
            contractId: contractIdData,
            modelContext: context
        )
        try context.save()

        return ContractPreviewState(
            contractIdBase58: contractIdBase58,
            contract: persistent,
            container: container,
            sdkGeneration: sdkGeneration
        )
    }

    /// Re-fetch an already stored contract and bring its row up to date,
    /// for after the contract was updated on Platform.
    /// `downloadAndPersistContract` cannot do this: it returns a stored
    /// row untouched.
    ///
    /// Replaces the row's `serializedContract` and `binarySerialization`
    /// with what Platform returns, registers the new binary with the SDK's
    /// trusted context (so later transitions validate against the updated
    /// contract), and runs `DataContractParser.parseDataContract` again,
    /// which refreshes the version, groups, tokens and document types in
    /// place. The row keeps its identity, so the contract's documents,
    /// token balances and token history stay attached. The description,
    /// keywords and `hasTokens` are refreshed from the fetched contract too.
    ///
    /// The fetch can return a valid proof of an older contract than the one
    /// stored or just updated (a node one block behind), so the fetched
    /// `version` must be at least the stored row's version and at least
    /// `minimumVersion`; otherwise nothing is registered or written.
    ///
    /// The rows are written in a context of their own on
    /// `modelContext.container`, saved once at the end. A failure drops that
    /// context, so nothing pending in `modelContext` (the caller's, usually
    /// the app's main context) is saved or discarded by this call. After the
    /// save the refreshed rows are fetched again in `modelContext`, so the
    /// copies it already holds show the refresh.
    ///
    /// - Parameter minimumVersion: The version the caller knows Platform
    ///   holds, such as the new version of an update just confirmed. `nil`
    ///   applies only the stored version as the floor.
    /// - Throws: `ContractDownloadError.notFound` / `.fetchFailed` when the
    ///   fetch fails, the response is for another contract or for an older
    ///   version, or the trusted context refuses the binary; any error the
    ///   parser or the save throws, after which the stored rows are left as
    ///   they were. `.fetchFailed` too when the refresh is saved but its rows
    ///   cannot be fetched again in `modelContext`: then only the copies
    ///   `modelContext` holds are stale, and a second refresh, which accepts
    ///   the version just saved, fetches them again.
    public static func refreshPersistedContract(
        _ contract: PersistentDataContract,
        sdk: SDK,
        modelContext: ModelContext,
        minimumVersion: Int? = nil
    ) async throws {
        let requestedId = contract.idBase58
        let fetched = try fetchContract(contractId: requestedId, sdk: sdk)
        try apply(
            fetched: fetched,
            toContractWithId: contract.id,
            minimumVersion: minimumVersion,
            modelContext: modelContext
        ) { binaryData in
            // Unlike the first download, a failure here is reported: the
            // trusted context would otherwise keep validating against the
            // contract as it was before the update.
            do {
                try sdk.addContractToContext(
                    contractId: requestedId,
                    binaryData: binaryData
                )
            } catch {
                throw ContractDownloadError.fetchFailed(
                    "Failed to add the updated contract to the trusted context: \(error.localizedDescription)"
                )
            }
        }
    }

    /// Store what a token configuration update did to contract `contractId`
    /// (`ManagedPlatformWallet.tokenUpdateConfig`).
    ///
    /// An applied update comes with the contract it produced, which Rust
    /// fetched and proved from the update's block or later: it is written
    /// through `apply`, so the stored version, tokens and groups advance
    /// together. A pending group action changes nothing.
    ///
    /// - Returns: nil when the stored contract is current, otherwise why it
    ///   still has the old configuration (the update itself is confirmed
    ///   either way).
    static func storeConfigUpdateOutcome(
        _ outcome: TokenConfigUpdateOutcome,
        contractId: Data,
        sdk: SDK?,
        modelContext: ModelContext
    ) -> String? {
        switch outcome {
        case .pending:
            return nil
        case .appliedNotRefreshed(_, let reason):
            return reason
        case .applied(let contractJSON, let serialization):
            guard let contractData = (try? JSONSerialization.jsonObject(with: contractJSON)) as? [String: Any] else {
                return "the updated contract could not be read"
            }
            do {
                try apply(
                    fetched: FetchedContract(
                        jsonData: contractJSON,
                        contractData: contractData,
                        binaryData: serialization
                    ),
                    toContractWithId: contractId,
                    minimumVersion: nil,
                    modelContext: modelContext
                ) { binaryData in
                    guard let sdk else {
                        throw ContractDownloadError.fetchFailed("The SDK is not initialized")
                    }
                    try sdk.addContractToContext(
                        contractId: contractId.toBase58String(),
                        binaryData: binaryData
                    )
                }
                return nil
            } catch {
                return error.localizedDescription
            }
        }
    }

    /// The part of `refreshPersistedContract` after the fetch: check the
    /// fetched contract against the stored row, then write it, in a context
    /// of its own on `modelContext.container`, and fetch the refreshed rows
    /// again in `modelContext`.
    ///
    /// `registerBinary` is called with the fetched binary serialization
    /// after the checks pass and before any row is written; a throw from it
    /// stops the refresh. A throw before the save leaves the stored rows as
    /// they were; a throw from the fetch after it means the refresh is saved
    /// and only `modelContext`'s copies are stale. Internal so tests can run
    /// it without the network.
    static func apply(
        fetched: FetchedContract,
        toContractWithId contractId: Data,
        minimumVersion: Int?,
        modelContext: ModelContext,
        registerBinary: (Data) throws -> Void
    ) throws {
        let requestedId = contractId.toBase58String()
        let fetchedId = try resolveContractId(
            contractData: fetched.contractData,
            requestedId: requestedId
        )
        guard fetchedId == contractId else {
            throw ContractDownloadError.fetchFailed(
                "The fetch returned contract \(fetchedId.toBase58String()), not \(requestedId)"
            )
        }

        // Dropped unsaved on any throw below.
        let context = ModelContext(modelContext.container)
        context.autosaveEnabled = false
        let row = try context.fetch(
            FetchDescriptor<PersistentDataContract>(predicate: #Predicate { $0.id == contractId })
        ).first
        guard let row else {
            throw ContractDownloadError.fetchFailed(
                "Contract \(requestedId) is not stored locally, so there is nothing to refresh"
            )
        }

        let floor = max(row.version ?? 0, minimumVersion ?? 0)
        guard let fetchedVersion = contractVersion(of: fetched.contractData) else {
            throw ContractDownloadError.fetchFailed(
                "The fetched contract \(requestedId) carries no version, so the local copy was not changed"
            )
        }
        guard fetchedVersion >= floor else {
            throw ContractDownloadError.fetchFailed(
                "Platform returned version \(fetchedVersion) of contract \(requestedId), older than version \(floor); the local copy was not changed. Refresh again later."
            )
        }

        if let binaryData = fetched.binaryData {
            try registerBinary(binaryData)
        }

        row.serializedContract = fetched.jsonData
        row.binarySerialization = fetched.binaryData
        row.lastUpdated = Date()
        row.lastSyncedAt = Date()
        try DataContractParser.parseDataContract(
            contractData: fetched.contractData,
            contractId: contractId,
            modelContext: context
        )
        refreshMetadataProjections(of: row, from: fetched.contractData, in: context)
        ContractIdentityLinker.linkContractToOwner(
            contract: row,
            modelContext: context
        )
        try context.save()
        do {
            try reloadRefreshedRows(ofContractWithId: contractId, in: modelContext)
        } catch {
            throw ContractDownloadError.fetchFailed(
                "The refreshed contract \(requestedId) was saved, but the copy on this screen could not be read again: \(error.localizedDescription)"
            )
        }
    }

    /// Fetch the contract's row, keywords, tokens and document types (with
    /// their indexes and properties) again in `modelContext`. A context does
    /// not take in another context's save by itself: the rows it already
    /// holds keep their old values, and can still point at rows the save
    /// deleted, until a fetch reads them from the store. A fetch leaves the
    /// context's pending edits alone.
    ///
    /// The refresh is saved by then: a throw means only those copies are
    /// stale, and running the refresh again (which accepts the saved version)
    /// reads them again.
    private static func reloadRefreshedRows(ofContractWithId contractId: Data, in modelContext: ModelContext) throws {
        var contract = FetchDescriptor<PersistentDataContract>(predicate: #Predicate { $0.id == contractId })
        contract.relationshipKeyPathsForPrefetching = [\.keywordRelations, \.tokens, \.documentTypes]
        _ = try modelContext.fetch(contract)
        var documentTypes = FetchDescriptor<PersistentDocumentType>(predicate: #Predicate { $0.contractId == contractId })
        documentTypes.relationshipKeyPathsForPrefetching = [\.indices, \.propertiesList]
        _ = try modelContext.fetch(documentTypes)
    }

    /// The contract JSON's `version`, or nil when it has no integer one.
    private static func contractVersion(of contractData: [String: Any]) -> Int? {
        guard let value = contractData["version"] as? NSNumber,
              CFGetTypeID(value as CFTypeRef) != CFBooleanGetTypeID()
        else {
            return nil
        }
        return Int(exactly: value)
    }

    /// Bring the row's stored copies of the contract's description, keywords
    /// and token presence in line with `contractData`. The parser does not
    /// write these; `downloadAndPersistContract` leaves them as the row was
    /// created.
    ///
    /// A keyword the contract still has keeps its row (the id is the
    /// contract id and the keyword, unique); the others are deleted and the
    /// new ones inserted.
    private static func refreshMetadataProjections(
        of row: PersistentDataContract,
        from contractData: [String: Any],
        in context: ModelContext
    ) {
        row.contractDescription = contractData["description"] as? String
        row.hasTokens = !((contractData["tokens"] as? [String: Any]) ?? [:]).isEmpty

        // Keywords are matched by their UTF-8 bytes, as Platform compares
        // them. Swift `String` equality treats canonically equivalent
        // spellings (`caf\u{E9}` and `cafe\u{301}`) as equal, which would keep
        // a replaced keyword's row or drop one of two distinct keywords.
        var fetchedBytes = Set<Data>()
        var fetchedKeywords: [String] = []
        for keyword in contractData["keywords"] as? [String] ?? []
        where fetchedBytes.insert(Data(keyword.utf8)).inserted {
            fetchedKeywords.append(keyword)
        }
        let storedRows = row.keywordRelations
        let kept = storedRows.filter { fetchedBytes.contains(Data($0.keyword.utf8)) }
        let keptBytes = Set(kept.map { Data($0.keyword.utf8) })
        let added = fetchedKeywords
            .filter { !keptBytes.contains(Data($0.utf8)) }
            .map { PersistentKeyword(keyword: $0, contractId: row.idBase58) }
        for stale in storedRows where !fetchedBytes.contains(Data(stale.keyword.utf8)) {
            context.delete(stale)
        }
        for keyword in added {
            context.insert(keyword)
        }
        row.keywordRelations = kept + added
    }

    // MARK: - Shared fetch

    /// What `fetchContract` copies out of the FFI result. Internal so tests
    /// can hand `apply` a fetched contract.
    struct FetchedContract {
        /// The contract JSON as returned.
        let jsonData: Data
        /// `jsonData` parsed into a dictionary.
        let contractData: [String: Any]
        /// The binary serialization, when the response carries one.
        let binaryData: Data?
    }

    /// Fetch the contract `contractId` (base58), JSON and binary in one
    /// round trip.
    ///
    /// The `dash_sdk_data_contract_fetch_with_serialization` FFI returns
    /// the result struct by value with heap-allocated inner pointers
    /// (`json_string` via `CString::into_raw`, `serialized_data` via
    /// `Box::into_raw(boxed_slice)`, `contract_handle` via `Box::into_raw`,
    /// `error` via `Box::into_raw`). Everything needed is copied into
    /// Swift-owned `Data` / `String` values, then
    /// `dash_sdk_data_contract_fetch_result_free` is called exactly once on
    /// the way out to reclaim every owned pointer.
    private static func fetchContract(contractId: String, sdk: SDK) throws -> FetchedContract {
        guard let handle = sdk.handle else {
            throw ContractDownloadError.fetchFailed("SDK not initialized")
        }

        var result = contractId.withCString { idCStr in
            dash_sdk_data_contract_fetch_with_serialization(handle, idCStr, true, true)
        }
        defer {
            withUnsafeMutablePointer(to: &result) { ptr in
                dash_sdk_data_contract_fetch_result_free(ptr)
            }
        }

        if let error = result.error {
            let message = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            // The FFI doesn't expose a structured "not found" code, so
            // we have to substring-match on the message.
            if message.contains("Data contract not found")
                || message.contains("not found") {
                throw ContractDownloadError.notFound(message)
            }
            throw ContractDownloadError.fetchFailed(
                "Failed to fetch data contract: \(message)"
            )
        }

        // The deferred free above takes ownership of the underlying
        // buffers, so copy everything out before returning.
        guard let jsonPtr = result.json_string else {
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

        guard let jsonData = jsonString.data(using: .utf8),
              let contractData = try? JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
            throw ContractDownloadError.fetchFailed(
                "Failed to parse contract JSON"
            )
        }

        return FetchedContract(
            jsonData: jsonData,
            contractData: contractData,
            binaryData: binaryData
        )
    }

    /// The contract id bytes: the id the JSON reports (the canonical form),
    /// or `requestedId` when the JSON omits it.
    private static func resolveContractId(
        contractData: [String: Any],
        requestedId: String
    ) throws -> Data {
        if let idString = contractData["id"] as? String,
           let idData = Data.identifier(fromBase58: idString) ?? Data(hexString: idString) {
            return idData
        }
        if let idData = Data.identifier(fromBase58: requestedId) {
            return idData
        }
        throw ContractDownloadError.fetchFailed(
            "Could not extract contract ID from response"
        )
    }
}
