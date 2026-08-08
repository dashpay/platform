import Foundation
import DashSDKFFI

/// Result of ``SDK/documentCount(dataContractId:documentType:whereJSON:orderByJSON:groupByJSON:limit:)``.
///
/// Mirrors the FFI's `{"counts": {"<hexKey>": <u64>}}` payload. Keys are the
/// hex-encoded group keys; for an ungrouped (aggregate) count there is a
/// single entry under the empty-string key, exposed via ``total``.
public struct DocumentCountResult: Sendable {
    /// Hex-encoded group key → count. The empty-string key holds the
    /// aggregate total for an ungrouped count.
    public let counts: [String: UInt64]

    public init(counts: [String: UInt64]) {
        self.counts = counts
    }

    /// The aggregate total for an ungrouped count: `counts[""]`. `nil` when
    /// the request was grouped (no empty-string entry).
    public var total: UInt64? {
        counts[""]
    }

    /// `true` when the result carries per-group counts (any non-empty key).
    public var isGrouped: Bool {
        counts.keys.contains { !$0.isEmpty }
    }
}

/// Result of ``SDK/documentSum(dataContractId:documentType:sumProperty:whereJSON:orderByJSON:groupByJSON:limit:)``.
///
/// Mirrors the FFI's `{"sums": {"<hexKey>": <i64>}}` payload. Keys are the
/// hex-encoded group keys; for an ungrouped (aggregate) sum there is a single
/// entry under the empty-string key, exposed via ``total``. Values are signed
/// (`Int64`) to match grovedb's `SumValue = i64`.
public struct DocumentSumResult: Sendable {
    /// Hex-encoded group key → sum. The empty-string key holds the aggregate
    /// total for an ungrouped sum.
    public let sums: [String: Int64]

    public init(sums: [String: Int64]) {
        self.sums = sums
    }

    /// The aggregate total for an ungrouped sum: `sums[""]`. `nil` when the
    /// request was grouped (no empty-string entry).
    public var total: Int64? {
        sums[""]
    }

    /// `true` when the result carries per-group sums (any non-empty key).
    public var isGrouped: Bool {
        sums.keys.contains { !$0.isEmpty }
    }
}

/// One `(count, sum)` pair returned by the average aggregation FFI for a single
/// key. Mirrors the FFI's `{"count": <u64>, "sum": <i64>}` object. The FFI
/// returns the raw pair un-divided so callers pick their own precision; compute
/// ``average`` (`sum / count`) at the point of display.
public struct DocumentAverageEntry: Sendable {
    /// Number of documents folded into this entry. Unsigned (`UInt64`).
    public let count: UInt64
    /// Sum of the numeric property over those documents. Signed (`Int64`) to
    /// match grovedb's `SumValue = i64`.
    public let sum: Int64

    public init(count: UInt64, sum: Int64) {
        self.count = count
        self.sum = sum
    }

    /// `sum / count` as a `Double`, or `nil` when `count == 0` (no documents
    /// matched, so the average is undefined).
    public var average: Double? {
        count > 0 ? Double(sum) / Double(count) : nil
    }
}

/// Result of ``SDK/documentAverage(dataContractId:documentType:sumProperty:whereJSON:orderByJSON:groupByJSON:limit:)``.
///
/// Mirrors the FFI's `{"averages": {"<hexKey>": {"count": <u64>, "sum": <i64>}}}`
/// payload. Keys are the hex-encoded group keys; for an ungrouped (aggregate)
/// average there is a single entry under the empty-string key, exposed via
/// ``total``.
public struct DocumentAverageResult: Sendable {
    /// Hex-encoded group key → `(count, sum)` pair. The empty-string key holds
    /// the aggregate total for an ungrouped average.
    public let averages: [String: DocumentAverageEntry]

    public init(averages: [String: DocumentAverageEntry]) {
        self.averages = averages
    }

    /// The aggregate `(count, sum)` for an ungrouped average: `averages[""]`.
    /// `nil` when the request was grouped (no empty-string entry).
    public var total: DocumentAverageEntry? {
        averages[""]
    }

    /// `true` when the result carries per-group averages (any non-empty key).
    public var isGrouped: Bool {
        averages.keys.contains { !$0.isEmpty }
    }
}

/// One element returned by ``SDK/systemPathElements(path:keys:)``.
///
/// Mirrors a single object in the FFI's JSON array payload
/// (`[{"key":"<hex>","element":"<formatted>","type":"<grovedb-variant>"}]`).
public struct PathElement: Sendable {
    /// Hex-encoded GroveDB key.
    public let key: String
    /// The formatted element value (e.g. a hex item, `sum_item:<n>`, or
    /// `tree`), exactly as rendered by the Rust side.
    public let element: String
    /// The GroveDB element variant (e.g. `tree`, `item`, `sum_item`,
    /// `sum_tree`).
    public let type: String

    public init(key: String, element: String, type: String) {
        self.key = key
        self.element = element
        self.type = type
    }
}

// MARK: - Platform Query Extensions for SDK
@MainActor
extension SDK {
    // Helper to pass non-Sendable pointers across @Sendable closures when safe
    private final class SendablePtr<T>: @unchecked Sendable {
        let ptr: UnsafeMutablePointer<T>
        init(_ p: UnsafeMutablePointer<T>) { self.ptr = p }
    }

    // MARK: - Helper Functions

    /// Process DashSDKResult and extract JSON
    internal func processJSONResult(_ result: DashSDKResult) throws -> [String: Any] {
        print("🔵 processJSONResult: Processing result...")

        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            print("❌ processJSONResult: FFI returned error: \(errorMessage)")
            // Preserve the FFI error code so transport failures surface as
            // .networkError / .timeout rather than a misleading .internalError.
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        guard let dataPtr = result.data else {
            print("❌ processJSONResult: No data returned from FFI")
            throw SDKError.notFound("No data returned")
        }

        // Check if the pointer is null (identity not found)
        if dataPtr == UnsafeMutableRawPointer(bitPattern: 0) {
            print("🔵 processJSONResult: Null pointer returned (identity not found)")
            throw SDKError.notFound("Identity not found")
        }

        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        print("🔵 processJSONResult: JSON string: \(jsonString)")
        dash_sdk_string_free(dataPtr)

        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            print("❌ processJSONResult: Failed to parse JSON")
            throw SDKError.serializationError("Failed to parse JSON data")
        }

        print("✅ processJSONResult: Successfully parsed JSON")
        return json
    }

    /// Process DashSDKResult and extract JSON array
    private func processJSONArrayResult(_ result: DashSDKResult) throws -> [[String: Any]] {
        if let error = result.error {
            // Preserve the FFI error code so transport failures surface as
            // .networkError / .timeout rather than a misleading .internalError.
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        guard let dataPtr = result.data else {
            return [] // Empty array
        }

        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)

        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
            throw SDKError.serializationError("Failed to parse JSON array")
        }

        return json
    }

    /// Process DashSDKResult and extract string
    private func processStringResult(_ result: DashSDKResult) throws -> String {
        if let error = result.error {
            // Preserve the FFI error code so transport failures surface as
            // .networkError / .timeout rather than a misleading .internalError.
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }

        let string: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)

        return string
    }

    /// Process DashSDKResult and extract UInt64
    private func processUInt64Result(_ result: DashSDKResult) throws -> UInt64 {
        let string = try processStringResult(result)
        guard let value = UInt64(string) else {
            throw SDKError.serializationError("Failed to parse UInt64 value")
        }
        return value
    }

    // MARK: - Identity Queries

    /// Get an identity by ID
    @MainActor
    public func identityGet(identityId: String) async throws -> [String: Any] {
        print("🔵 SDK.identityGet: Called with ID: \(identityId)")

        guard let sdkHandle = handle else {
            print("❌ SDK.identityGet: SDK handle is nil")
            throw SDKError.invalidState("SDK not initialized")
        }

        print("🔵 SDK.identityGet: SDK handle exists: \(sdkHandle)")
        print("🔵 SDK.identityGet: About to call dash_sdk_identity_fetch with handle and ID: \(identityId)")

        // Make the FFI call directly and return the parsed JSON
        let result = dash_sdk_identity_fetch(sdkHandle, identityId)
        return try processJSONResult(result)
    }

    /// Get identity keys
    public func identityGetKeys(
        identityId: String,
        keyRequestType: String? = nil,
        specificKeyIds: [String]? = nil,
        searchPurposeMap: String? = nil,
        limit: UInt32? = nil,
        offset: UInt32? = nil
    ) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // For now, use the simple fetch - would need to implement complex key fetching in FFI
        let result = dash_sdk_identity_fetch_public_keys(handle, identityId)
        return try processJSONResult(result)
    }

    /// Get identities contract keys
    public func identityGetContractKeys(
        identityIds: [String],
        contractId: String,
        documentType: String?,
        purposes: [String]? = nil
    ) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Join identity IDs with commas
        let identityIdsStr = identityIds.joined(separator: ",")

        // Convert purposes to comma-separated string (default to all purposes if not specified)
        let purposesStr = purposes?.joined(separator: ",") ?? "0,1,2,3"

        let result = dash_sdk_identities_fetch_contract_keys(
            handle,
            identityIdsStr,
            contractId,
            documentType,
            purposesStr
        )

        return try processJSONResult(result)
    }

    /// Get identity nonce
    public func identityGetNonce(identityId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_fetch_nonce(handle, identityId)
        return try processUInt64Result(result)
    }

    /// Get identity contract nonce
    public func identityGetContractNonce(identityId: String, contractId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_fetch_contract_nonce(handle, identityId, contractId)
        return try processUInt64Result(result)
    }

    /// Get identity balance
    public func identityGetBalance(identityId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_fetch_balance(handle, identityId)
        return try processUInt64Result(result)
    }

    /// Get identities balances
    public func identityGetBalances(identityIds: [String]) async throws -> [String: UInt64] {
        // This would need to call dash_sdk_identity_fetch_balance for each ID
        // or we need a batch FFI function
        var balances: [String: UInt64] = [:]

        for identityId in identityIds {
            do {
                let balance = try await identityGetBalance(identityId: identityId)
                balances[identityId] = balance
            } catch {
                // Skip failed fetches
                continue
            }
        }

        return balances
    }

    /// Get identity balance and revision
    public func identityGetBalanceAndRevision(identityId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_fetch_balance_and_revision(handle, identityId)
        return try processJSONResult(result)
    }

    /// Get identity by public key hash
    public func identityGetByPublicKeyHash(publicKeyHash: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_fetch_by_public_key_hash(handle, publicKeyHash)
        return try processJSONResult(result)
    }

    /// Get identities by non-unique public key hash
    public func identityGetByNonUniquePublicKeyHash(publicKeyHash: String, startAfter: String? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_fetch_by_non_unique_public_key_hash(handle, publicKeyHash, startAfter)
        return try processJSONArrayResult(result)
    }

    // MARK: - Trusted Context Management

    /// Add a data contract to the trusted context provider cache
    /// This allows the SDK to use the contract without fetching it from the network
    public func addContractToContext(contractId: String, binaryData: Data) throws {
        guard self.handle != nil else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // The Rust FFI expects comma-separated contract IDs and binary serialized contract data
        let contracts = [(id: contractId, data: binaryData)]

        // Use the existing loadKnownContracts function which properly handles binary data
        try loadKnownContracts(contracts)

        print("✅ Added contract \(contractId) to trusted context")
    }

    // MARK: - Data Contract Queries

    /// Get a data contract by ID
    public func dataContractGet(id: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Use the new unified function with return_json = true, return_serialized = false
        let result = id.withCString { idCStr in
            dash_sdk_data_contract_fetch_with_serialization(handle, idCStr, true, false)
        }

        // Check for error
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
        }

        // Get the JSON string
        guard result.json_string != nil else {
            throw SDKError.internalError("No JSON data returned from contract fetch")
        }

        let jsonString = String(cString: result.json_string!)

        // Free the result
        var mutableResult = result
        dash_sdk_data_contract_fetch_result_free(&mutableResult)

        // Parse the JSON
        guard let jsonData = jsonString.data(using: .utf8),
              let jsonObject = try? JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse contract JSON")
        }

        return jsonObject
    }

    /// Get data contract history
    public func dataContractGetHistory(id: String, limit: UInt32?, offset: UInt32?, startAtMs: UInt64? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_data_contract_fetch_history(handle, id, limit ?? 100, offset ?? 0, startAtMs ?? 0)

        // The result is a JSON object with an "entries" field containing the array
        let jsonObject = try processJSONResult(result)

        // Extract the entries array
        guard let entries = jsonObject["entries"] as? [[String: Any]] else {
            throw SDKError.serializationError("Expected 'entries' array in data contract history response")
        }

        return entries
    }

    /// Get multiple data contracts
    public func dataContractGetMultiple(ids: [String]) async throws -> [[String: Any]] {
        // Call fetch for each contract ID
        var contracts: [[String: Any]] = []

        for id in ids {
            do {
                let contract = try await dataContractGet(id: id)
                contracts.append(contract)
            } catch {
                // Skip failed fetches
                continue
            }
        }

        return contracts
    }

    // MARK: - Document Queries

    /// List documents
    public func documentList(
        dataContractId: String,
        documentType: String,
        whereClause: String? = nil,
        orderByClause: String? = nil,
        limit: UInt32? = nil,
        startAfter: String? = nil,
        startAt: String? = nil
    ) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // First fetch the data contract
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
        }

        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }

        defer {
            // Clean up contract handle when done
            let contractPtr = OpaquePointer(contractHandle)
            dash_sdk_data_contract_destroy(contractPtr)
        }

        // Create search parameters struct with proper string handling
        let documentTypeCString = documentType.cString(using: .utf8)!
        let whereClauseCString = whereClause?.cString(using: .utf8)
        let orderByClauseCString = orderByClause?.cString(using: .utf8)

        let result = documentTypeCString.withUnsafeBufferPointer { documentTypePtr in
            if let whereClause = whereClauseCString {
                return whereClause.withUnsafeBufferPointer { wherePtr in
                    if let orderByClause = orderByClauseCString {
                        return orderByClause.withUnsafeBufferPointer { orderByPtr in
                            var searchParams = DashSDKDocumentSearchParams()
                            searchParams.data_contract_handle = OpaquePointer(contractHandle)
                            searchParams.document_type = documentTypePtr.baseAddress
                            searchParams.where_json = wherePtr.baseAddress
                            searchParams.order_by_json = orderByPtr.baseAddress
                            searchParams.limit = limit ?? 100
                            // Handle pagination - startAt takes precedence over startAfter
                            if let startAt = startAt {
                                // startAt is inclusive - start from this exact position
                                searchParams.start_at = UInt32(startAt) ?? 0
                            } else if let startAfter = startAfter {
                                // startAfter is exclusive - start from the next position
                                searchParams.start_at = (UInt32(startAfter) ?? 0) + 1
                            } else {
                                searchParams.start_at = 0
                            }

                            return dash_sdk_document_search(handle, &searchParams)
                        }
                    } else {
                        var searchParams = DashSDKDocumentSearchParams()
                        searchParams.data_contract_handle = OpaquePointer(contractHandle)
                        searchParams.document_type = documentTypePtr.baseAddress
                        searchParams.where_json = wherePtr.baseAddress
                        searchParams.order_by_json = nil
                        searchParams.limit = limit ?? 100
                        // Handle pagination - startAt takes precedence over startAfter
                        if let startAt = startAt {
                            // startAt is inclusive - start from this exact position
                            searchParams.start_at = UInt32(startAt) ?? 0
                        } else if let startAfter = startAfter {
                            // startAfter is exclusive - start from the next position
                            searchParams.start_at = (UInt32(startAfter) ?? 0) + 1
                        } else {
                            searchParams.start_at = 0
                        }

                        return dash_sdk_document_search(handle, &searchParams)
                    }
                }
            } else {
                var searchParams = DashSDKDocumentSearchParams()
                searchParams.data_contract_handle = OpaquePointer(contractHandle)
                searchParams.document_type = documentTypePtr.baseAddress
                searchParams.where_json = nil
                searchParams.order_by_json = nil
                searchParams.limit = limit ?? 100
                searchParams.start_at = 0

                return dash_sdk_document_search(handle, &searchParams)
            }
        }

        return try processJSONResult(result)
    }

    /// Get a specific document
    @MainActor
    public func documentGet(dataContractId: String, documentType: String, documentId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // First fetch the data contract
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch data contract: \(errorMessage)")
        }

        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }

        defer {
            // Clean up contract handle when done
            let contractPtr = OpaquePointer(contractHandle)
            dash_sdk_data_contract_destroy(contractPtr)
        }

        // Now fetch the document
        let documentResult = dash_sdk_document_fetch(handle, OpaquePointer(contractHandle), documentType, documentId)

        if let error = documentResult.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError("Failed to fetch document: \(errorMessage)")
        }

        guard let documentHandle = documentResult.data else {
            throw SDKError.notFound("Document not found")
        }

        defer {
            // Clean up document handle
            dash_sdk_document_destroy(handle, OpaquePointer(documentHandle))
        }

        // Get document info to convert to JSON
        let info = dash_sdk_document_get_info(OpaquePointer(documentHandle))
        defer {
            if let info = info {
                dash_sdk_document_info_free(info)
            }
        }

        guard let infoPtr = info else {
            throw SDKError.internalError("Failed to get document info")
        }

        // Convert document info to dictionary
        let documentInfo = infoPtr.pointee

        // Build JSON representation from document info fields
        var json: [String: Any] = [
            "$id": documentInfo.id != nil ? String(cString: documentInfo.id!) : "",
            "$ownerId": documentInfo.owner_id != nil ? String(cString: documentInfo.owner_id!) : "",
            "$dataContractId": documentInfo.data_contract_id != nil ? String(cString: documentInfo.data_contract_id!) : "",
            "$type": documentInfo.document_type != nil ? String(cString: documentInfo.document_type!) : "",
            "$revision": documentInfo.revision,
            "$createdAt": documentInfo.created_at,
            "$updatedAt": documentInfo.updated_at
        ]

        // Add data fields
        if documentInfo.data_fields_count > 0 && documentInfo.data_fields != nil {
            for i in 0..<Int(documentInfo.data_fields_count) {
                let field = documentInfo.data_fields!.advanced(by: i).pointee

                let fieldName = field.name != nil ? String(cString: field.name!) : ""
                let fieldValueStr = field.value != nil ? String(cString: field.value!) : ""

                // Convert field value based on type
                let fieldValue: Any
                switch field.field_type {
                case FieldInteger:
                    fieldValue = field.int_value
                case FieldFloat:
                    fieldValue = field.float_value
                case FieldBoolean:
                    fieldValue = field.bool_value
                case FieldArray, FieldObject:
                    // Parse JSON string
                    if let data = fieldValueStr.data(using: String.Encoding.utf8),
                       let parsed = try? JSONSerialization.jsonObject(with: data) {
                        fieldValue = parsed
                    } else {
                        fieldValue = fieldValueStr
                    }
                case FieldNull:
                    fieldValue = NSNull()
                case FieldBytes:
                    // For bytes, we get hex string - could convert back to Data if needed
                    fieldValue = fieldValueStr
                default: // FieldString or unknown
                    fieldValue = fieldValueStr
                }

                json[fieldName] = fieldValue
            }
        }

        return json
    }

    /// Count documents of a given type, optionally filtered by `where` and
    /// grouped by `group_by`.
    ///
    /// Thin bridge over `dash_sdk_document_count`: it fetches the contract
    /// handle (same precedent as `documentList`/`documentGet`), marshals the
    /// document type + optional `where`/`order_by`/`group_by` JSON + the
    /// `limit` sentinel in, calls the FFI, and marshals the
    /// `{"counts": {"<hexKey>": <u64>}}` payload out. All aggregation is done
    /// on the Rust side; nothing is decided here.
    ///
    /// - Parameters:
    ///   - dataContractId: Base58 id of the data contract holding the type.
    ///   - documentType: The document type name to count.
    ///   - whereJSON: Optional `[{field, operator, value}]` filter JSON.
    ///     Pass `nil` for an unfiltered count.
    ///   - orderByJSON: Optional `[{field, direction}]` JSON. Pass `nil` for
    ///     none (server defaults to ascending).
    ///   - groupByJSON: Optional `["<field>", ...]` JSON. Pass `nil` for an
    ///     aggregate (ungrouped) count.
    ///   - limit: Sentinel-encoded `int64`. `-1` = use server default
    ///     (the default). `> 0` = explicit cap. `0` is rejected at the FFI
    ///     boundary, so the wrapper rejects it before calling.
    /// - Returns: A ``DocumentCountResult`` mapping hex-encoded group key →
    ///   count. For an ungrouped count the total lives under the empty-string
    ///   key and is surfaced via ``DocumentCountResult/total``.
    @MainActor
    public func documentCount(
        dataContractId: String,
        documentType: String,
        whereJSON: String? = nil,
        orderByJSON: String? = nil,
        groupByJSON: String? = nil,
        limit: Int64 = -1
    ) async throws -> DocumentCountResult {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        guard limit != 0 else {
            // The FFI rejects limit == 0 (the v1 wire rejects Some(0)); fail
            // fast with a clear message rather than relaying it.
            throw SDKError.invalidParameter("limit must be -1 (server default) or a positive value, not 0")
        }

        // Fetch the contract handle (precedent: documentList / documentGet).
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }
        defer {
            dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
        }

        // Marshal the optional JSON strings in. nil → null pointer = "none".
        let result = documentType.withCString { typePtr in
            withOptionalCString(whereJSON) { wherePtr in
                withOptionalCString(orderByJSON) { orderPtr in
                    withOptionalCString(groupByJSON) { groupPtr in
                        dash_sdk_document_count(
                            handle,
                            OpaquePointer(contractHandle),
                            typePtr,
                            wherePtr,
                            orderPtr,
                            groupPtr,
                            limit
                        )
                    }
                }
            }
        }

        let json = try processJSONResult(result)

        // Payload shape: {"counts": {"<hexKey>": <u64>}}.
        guard let countsObject = json["counts"] as? [String: Any] else {
            throw SDKError.serializationError("Expected 'counts' object in document count response")
        }

        var counts: [String: UInt64] = [:]
        for (key, value) in countsObject {
            if let num = value as? NSNumber {
                counts[key] = num.uint64Value
            }
        }

        return DocumentCountResult(counts: counts)
    }

    /// Sum a numeric property of a document type, optionally filtered by
    /// `where` and grouped by `group_by`.
    ///
    /// Thin bridge over `dash_sdk_document_sum`: it fetches the contract handle
    /// (same precedent as `documentCount`), marshals the document type + the
    /// required `sumProperty` + optional `where`/`order_by`/`group_by` JSON +
    /// the `limit` sentinel in, calls the FFI, and marshals the
    /// `{"sums": {"<hexKey>": <i64>}}` payload out. All aggregation is done on
    /// the Rust side; nothing is decided here.
    ///
    /// - Parameters:
    ///   - dataContractId: Base58 id of the data contract holding the type.
    ///   - documentType: The document type name to aggregate.
    ///   - sumProperty: Name of the integer property to sum. Required and
    ///     non-empty (the FFI rejects an empty string), so the wrapper rejects
    ///     it before calling.
    ///   - whereJSON: Optional `[{field, operator, value}]` filter JSON. Pass
    ///     `nil` for an unfiltered sum.
    ///   - orderByJSON: Optional `[{field, direction}]` JSON. Pass `nil` for
    ///     none.
    ///   - groupByJSON: Optional `["<field>", ...]` JSON. Pass `nil` for an
    ///     aggregate (ungrouped) sum.
    ///   - limit: Sentinel-encoded `int64`. `-1` = server default (the
    ///     default). `> 0` = explicit cap. `0` is rejected at the FFI boundary,
    ///     so the wrapper rejects it before calling.
    /// - Returns: A ``DocumentSumResult`` mapping hex-encoded group key → sum.
    ///   For an ungrouped sum the total lives under the empty-string key and is
    ///   surfaced via ``DocumentSumResult/total``.
    @MainActor
    public func documentSum(
        dataContractId: String,
        documentType: String,
        sumProperty: String,
        whereJSON: String? = nil,
        orderByJSON: String? = nil,
        groupByJSON: String? = nil,
        limit: Int64 = -1
    ) async throws -> DocumentSumResult {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        guard !sumProperty.isEmpty else {
            // The FFI rejects an empty sum property; fail fast with a clear
            // message rather than relaying it.
            throw SDKError.invalidParameter("sumProperty must name the numeric property to sum; it cannot be empty")
        }
        guard limit != 0 else {
            // The FFI rejects limit == 0 (the v1 wire rejects Some(0)); fail
            // fast with a clear message rather than relaying it.
            throw SDKError.invalidParameter("limit must be -1 (server default) or a positive value, not 0")
        }

        // Fetch the contract handle (precedent: documentCount).
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }
        defer {
            dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
        }

        // Marshal the strings in. sumProperty is required (non-null);
        // nil where/order/group → null pointer = "none".
        let result = documentType.withCString { typePtr in
            sumProperty.withCString { sumPropPtr in
                withOptionalCString(whereJSON) { wherePtr in
                    withOptionalCString(orderByJSON) { orderPtr in
                        withOptionalCString(groupByJSON) { groupPtr in
                            dash_sdk_document_sum(
                                handle,
                                OpaquePointer(contractHandle),
                                typePtr,
                                sumPropPtr,
                                wherePtr,
                                orderPtr,
                                groupPtr,
                                limit
                            )
                        }
                    }
                }
            }
        }

        let json = try processJSONResult(result)

        // Payload shape: {"sums": {"<hexKey>": <i64>}}.
        guard let sumsObject = json["sums"] as? [String: Any] else {
            throw SDKError.serializationError("Expected 'sums' object in document sum response")
        }

        var sums: [String: Int64] = [:]
        for (key, value) in sumsObject {
            if let num = value as? NSNumber {
                sums[key] = num.int64Value
            }
        }

        return DocumentSumResult(sums: sums)
    }

    /// Average a numeric property of a document type, optionally filtered by
    /// `where` and grouped by `group_by`.
    ///
    /// Thin bridge over `dash_sdk_document_average`: it fetches the contract
    /// handle, marshals the document type + the required `sumProperty` +
    /// optional `where`/`order_by`/`group_by` JSON + the `limit` sentinel in,
    /// calls the FFI, and marshals the
    /// `{"averages": {"<hexKey>": {"count": <u64>, "sum": <i64>}}}` payload
    /// out. The FFI returns the raw `(count, sum)` pair un-divided so callers
    /// pick their own precision; the wrapper preserves that pair verbatim and
    /// leaves the division to the caller. All aggregation is done on the Rust
    /// side; nothing is decided here.
    ///
    /// - Parameters:
    ///   - dataContractId: Base58 id of the data contract holding the type.
    ///   - documentType: The document type name to aggregate.
    ///   - sumProperty: Name of the integer property to average. Required and
    ///     non-empty (the FFI rejects an empty string), so the wrapper rejects
    ///     it before calling.
    ///   - whereJSON: Optional `[{field, operator, value}]` filter JSON. Pass
    ///     `nil` for an unfiltered average.
    ///   - orderByJSON: Optional `[{field, direction}]` JSON. Pass `nil` for
    ///     none.
    ///   - groupByJSON: Optional `["<field>", ...]` JSON. Pass `nil` for an
    ///     aggregate (ungrouped) average.
    ///   - limit: Sentinel-encoded `int64`. `-1` = server default (the
    ///     default). `> 0` = explicit cap. `0` is rejected at the FFI boundary,
    ///     so the wrapper rejects it before calling.
    /// - Returns: A ``DocumentAverageResult`` mapping hex-encoded group key →
    ///   `(count, sum)`. For an ungrouped average the total lives under the
    ///   empty-string key and is surfaced via ``DocumentAverageResult/total``.
    @MainActor
    public func documentAverage(
        dataContractId: String,
        documentType: String,
        sumProperty: String,
        whereJSON: String? = nil,
        orderByJSON: String? = nil,
        groupByJSON: String? = nil,
        limit: Int64 = -1
    ) async throws -> DocumentAverageResult {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        guard !sumProperty.isEmpty else {
            // The FFI rejects an empty sum property; fail fast with a clear
            // message rather than relaying it.
            throw SDKError.invalidParameter("sumProperty must name the numeric property to average; it cannot be empty")
        }
        guard limit != 0 else {
            // The FFI rejects limit == 0 (the v1 wire rejects Some(0)); fail
            // fast with a clear message rather than relaying it.
            throw SDKError.invalidParameter("limit must be -1 (server default) or a positive value, not 0")
        }

        // Fetch the contract handle (precedent: documentCount).
        let contractResult = dash_sdk_data_contract_fetch(handle, dataContractId)
        if let error = contractResult.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        guard let contractHandle = contractResult.data else {
            throw SDKError.notFound("Data contract not found")
        }
        defer {
            dash_sdk_data_contract_destroy(OpaquePointer(contractHandle))
        }

        // Marshal the strings in. sumProperty is required (non-null);
        // nil where/order/group → null pointer = "none".
        let result = documentType.withCString { typePtr in
            sumProperty.withCString { sumPropPtr in
                withOptionalCString(whereJSON) { wherePtr in
                    withOptionalCString(orderByJSON) { orderPtr in
                        withOptionalCString(groupByJSON) { groupPtr in
                            dash_sdk_document_average(
                                handle,
                                OpaquePointer(contractHandle),
                                typePtr,
                                sumPropPtr,
                                wherePtr,
                                orderPtr,
                                groupPtr,
                                limit
                            )
                        }
                    }
                }
            }
        }

        let json = try processJSONResult(result)

        // Payload shape: {"averages": {"<hexKey>": {"count": <u64>, "sum": <i64>}}}.
        guard let averagesObject = json["averages"] as? [String: Any] else {
            throw SDKError.serializationError("Expected 'averages' object in document average response")
        }

        var averages: [String: DocumentAverageEntry] = [:]
        for (key, value) in averagesObject {
            guard let entry = value as? [String: Any],
                  let countNum = entry["count"] as? NSNumber,
                  let sumNum = entry["sum"] as? NSNumber else {
                continue
            }
            averages[key] = DocumentAverageEntry(
                count: countNum.uint64Value,
                sum: sumNum.int64Value
            )
        }

        return DocumentAverageResult(averages: averages)
    }

    /// Call `body` with a `const char *` for an optional Swift string,
    /// passing a null pointer when the string is `nil`. Mirrors the FFI's
    /// "null/empty means none" contract for the where/order/group JSON args.
    private func withOptionalCString<R>(
        _ string: String?,
        _ body: (UnsafePointer<CChar>?) -> R
    ) -> R {
        if let string = string {
            return string.withCString { body($0) }
        } else {
            return body(nil)
        }
    }

    // MARK: - DPNS Queries

    /// Get DPNS usernames for identity
    @MainActor
    public func dpnsGetUsername(identityId: String, limit: UInt32?) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Call native FFI function with identity ID as string
        let result = dash_sdk_dpns_get_usernames(handle, identityId, limit ?? 10)

        return try processJSONArrayResult(result)
    }

    /// Check DPNS name availability
    public func dpnsCheckAvailability(name: String) async throws -> Bool {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Call native FFI function
        let result = dash_sdk_dpns_check_availability(handle, name)

        // Process the result to get the availability info
        let json = try processJSONResult(result)

        // Extract the "available" boolean from the result
        guard let isAvailable = json["available"] as? Bool else {
            throw SDKError.serializationError("Failed to parse availability result")
        }

        return isAvailable
    }

    /// Get non-resolved DPNS contests for a specific identity
    @MainActor
    public func dpnsGetNonResolvedContestsForIdentity(identityId: String, limit: UInt32?) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Call native FFI function which now returns a pointer to DashSDKContestedNamesList
        guard let contestedNamesListPtr = dash_sdk_dpns_get_non_resolved_contests_for_identity(handle, identityId, limit ?? 20) else {
            throw SDKError.internalError("Failed to get contested names")
        }

        defer {
            // Free the C structure when done
            dash_sdk_contested_names_list_free(contestedNamesListPtr)
        }

        // Convert C structure to Swift dictionary
        let contestedNamesList = contestedNamesListPtr.pointee
        var result: [String: Any] = [:]

        if contestedNamesList.count > 0 && contestedNamesList.names != nil {
            for i in 0..<contestedNamesList.count {
                let contestedName = contestedNamesList.names![Int(i)]

                // Get the name
                guard let namePtr = contestedName.name else { continue }
                let name = String(cString: namePtr)

                // Parse contest info
                var contestInfo: [String: Any] = [:]
                let contest = contestedName.contest_info

                // Add end time
                contestInfo["endTime"] = contest.end_time
                contestInfo["hasWinner"] = contest.has_winner

                // Add vote tallies
                contestInfo["abstainVotes"] = contest.abstain_votes
                contestInfo["lockVotes"] = contest.lock_votes

                // Parse contenders
                var contenders: [[String: Any]] = []
                if contest.contender_count > 0 && contest.contenders != nil {
                    for j in 0..<contest.contender_count {
                        let contender = contest.contenders![Int(j)]

                        var contenderDict: [String: Any] = [:]
                        if let idPtr = contender.identity_id {
                            contenderDict["identifier"] = String(cString: idPtr)
                        }
                        contenderDict["votes"] = "ResourceVote { vote_choice: TowardsIdentity, strength: \(contender.vote_count) }"
                        // The spelling this contender actually requested
                        // ("pizza"), where the contest key is the normalized
                        // form ("p1zza"). Absent when the FFI could not decode
                        // their document. Prefer the typed
                        // `dpnsActiveContests` / `dpnsContestsForIdentity`,
                        // which also give the tally as an integer.
                        if let labelPtr = contender.label {
                            let label = String(cString: labelPtr)
                            if !label.isEmpty { contenderDict["label"] = label }
                        }

                        contenders.append(contenderDict)
                    }
                }
                contestInfo["contenders"] = contenders

                result[name] = contestInfo
            }
        }

        return result
    }

    /// Get current DPNS contests (active vote polls)
    @MainActor
    public func dpnsGetCurrentContests(startTime: UInt64 = 0, endTime: UInt64 = 0, limit: UInt16 = 100) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Call native FFI function which returns a pointer to DashSDKNameTimestampList
        guard let nameTimestampListPtr = dash_sdk_dpns_get_current_contests(handle, startTime, endTime, limit) else {
            throw SDKError.internalError("Failed to get current contests")
        }

        defer {
            // Free the C structure when done
            dash_sdk_name_timestamp_list_free(nameTimestampListPtr)
        }

        // Convert C structure to Swift dictionary
        let nameTimestampList = nameTimestampListPtr.pointee
        var result: [String: UInt64] = [:]

        if nameTimestampList.count > 0 && nameTimestampList.entries != nil {
            for i in 0..<nameTimestampList.count {
                let entry = nameTimestampList.entries![Int(i)]

                guard let namePtr = entry.name else { continue }
                let name = String(cString: namePtr)
                result[name] = entry.end_time
            }
        }

        return result
    }

    /// Get the vote state for a contested DPNS username
    @MainActor
    public func dpnsGetContestedVoteState(name: String, limit: UInt32 = 100) async throws -> [String: Any] {
        guard let handle = self.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        let result = name.withCString { namePtr in
            dash_sdk_dpns_get_contested_vote_state(handle, namePtr, limit)
        }

        // Check for error
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        // Parse the JSON result
        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }

        let jsonString = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))

        guard let jsonData = jsonString.data(using: .utf8),
              let voteState = try? JSONSerialization.jsonObject(with: jsonData, options: []) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse vote state JSON")
        }

        return voteState
    }

    /// Get contested DPNS usernames that are not yet resolved
    public func dpnsGetContestedNonResolvedUsernames(limit: UInt32 = 100) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Call native FFI function which returns a pointer to DashSDKContestedNamesList
        guard let contestedNamesListPtr = dash_sdk_dpns_get_contested_non_resolved_usernames(handle, limit) else {
            throw SDKError.internalError("Failed to get contested names")
        }

        defer {
            // Free the C structure when done
            dash_sdk_contested_names_list_free(contestedNamesListPtr)
        }

        // Convert C structure to Swift dictionary
        let contestedNamesList = contestedNamesListPtr.pointee
        var result: [String: Any] = [:]

        if contestedNamesList.count > 0 && contestedNamesList.names != nil {
            for i in 0..<contestedNamesList.count {
                let contestedName = contestedNamesList.names![Int(i)]

                // Get the name
                guard let namePtr = contestedName.name else { continue }
                let name = String(cString: namePtr)

                // Parse contest info
                var contestInfo: [String: Any] = [:]
                let contest = contestedName.contest_info

                // Add end time
                contestInfo["endTime"] = contest.end_time
                contestInfo["hasWinner"] = contest.has_winner

                // Add vote tallies
                contestInfo["abstainVotes"] = contest.abstain_votes
                contestInfo["lockVotes"] = contest.lock_votes

                // Parse contenders
                var contenders: [[String: Any]] = []
                if contest.contender_count > 0 && contest.contenders != nil {
                    for j in 0..<contest.contender_count {
                        let contender = contest.contenders![Int(j)]

                        var contenderDict: [String: Any] = [:]
                        if let idPtr = contender.identity_id {
                            contenderDict["identifier"] = String(cString: idPtr)
                        }
                        contenderDict["votes"] = "ResourceVote { vote_choice: TowardsIdentity, strength: \(contender.vote_count) }"
                        // The spelling this contender actually requested
                        // ("pizza"), where the contest key is the normalized
                        // form ("p1zza"). Absent when the FFI could not decode
                        // their document. Prefer the typed
                        // `dpnsActiveContests` / `dpnsContestsForIdentity`,
                        // which also give the tally as an integer.
                        if let labelPtr = contender.label {
                            let label = String(cString: labelPtr)
                            if !label.isEmpty { contenderDict["label"] = label }
                        }

                        contenders.append(contenderDict)
                    }
                }
                contestInfo["contenders"] = contenders

                result[name] = contestInfo
            }
        }

        return result
    }

    /// Resolve DPNS name to identity ID
    public func dpnsResolve(name: String) async throws -> String {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_identity_resolve_name(handle, name)

        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            throw SDKError.notFound("Name not found")
        }

        // Cast to DashSDKBinaryData to get the binary identity ID
        let binaryData = dataPtr.assumingMemoryBound(to: DashSDKBinaryData.self).pointee

        // Convert the 32-byte identity ID to hex string
        let identityIdData = Data(bytes: binaryData.data, count: Int(binaryData.len))
        let identityIdHex = identityIdData.toHexString()

        // Free the binary data
        dash_sdk_binary_data_free(dataPtr.assumingMemoryBound(to: DashSDKBinaryData.self))

        return identityIdHex
    }

    /// Search DPNS names by prefix
    @MainActor
    public func dpnsSearch(prefix: String, limit: UInt32? = nil) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Call native FFI function
        let result = dash_sdk_dpns_search(handle, prefix, limit ?? 10)

        return try processJSONArrayResult(result)
    }

    // MARK: - Voting & Contested Resources Queries

    /// Get contested resources
    public func getContestedResources(
        documentTypeName: String,
        dataContractId: String,
        indexName: String,
        resultType: String,
        allowIncludeLockedAndAbstainingVoteTally: Bool,
        startAtValue: String?,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_contested_resource_get_resources(
            handle,
            dataContractId,
            documentTypeName,
            indexName,
            startAtValue,
            nil, // end_index_values_json
            limit ?? 100,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }

    /// Get contested resource vote state
    public func getContestedResourceVoteState(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: [String]? = nil,
        resultType: String,
        allowIncludeLockedAndAbstainingVoteTally: Bool,
        startAtIdentifierInfo: String?,
        count: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Convert result type to integer
        let resultTypeInt: UInt8 = switch resultType {
        case "contenders": 0
        case "abstainers": 1
        case "locked": 2
        default: 0
        }

        // Create index values JSON array
        let indexValuesData = try JSONSerialization.data(withJSONObject: indexValues ?? [])
        let indexValuesJson = String(data: indexValuesData, encoding: .utf8) ?? "[]"

        let result = dash_sdk_contested_resource_get_vote_state(
            handle,
            dataContractId,
            documentTypeName,
            indexName,
            indexValuesJson,
            resultTypeInt,
            allowIncludeLockedAndAbstainingVoteTally,
            count ?? 100
        )
        return try processJSONArrayResult(result)
    }

    /// Get contested resource voters for identity
    public func getContestedResourceVotersForIdentity(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: [String]? = nil,
        contestantId: String,
        startAtIdentifierInfo: String?,
        count: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Create index values JSON array
        let indexValuesData = try JSONSerialization.data(withJSONObject: indexValues ?? [])
        let indexValuesJson = String(data: indexValuesData, encoding: .utf8) ?? "[]"

        let result = dash_sdk_contested_resource_get_voters_for_identity(
            handle,
            dataContractId,
            documentTypeName,
            indexName,
            indexValuesJson,
            contestantId,
            count ?? 100,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }

    /// Get contested resource identity votes
    public func getContestedResourceIdentityVotes(
        identityId: String,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_contested_resource_get_identity_votes(
            handle,
            identityId,
            limit ?? 100,
            offset ?? 0,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }

    // MARK: - Contested Resource Vote (write path)

    /// Cast a masternode contested-resource (DPNS) vote and wait for the
    /// platform response.
    ///
    /// Thin bridge over `dash_sdk_contested_resource_cast_vote`: it marshals
    /// the vote poll, the choice, the masternode `proTxHash`, and the 32-byte
    /// voting private key in, then surfaces success / failure out. All of the
    /// transition assembly + signing + broadcast happens on the Rust side
    /// (rs-sdk's `PutVote`); nothing is decided here.
    ///
    /// - Important: Contested-resource votes are cast by **masternodes** using
    ///   their masternode voting key (tied to a `proTxHash`). A regular wallet
    ///   is not a masternode and has no voting key, so a vote from such a
    ///   wallet reaches a deterministic authorization rejection on the platform
    ///   side. Supplying a genuine masternode `proTxHash` + voting private key
    ///   is required for an accepted vote.
    ///
    /// - Parameters:
    ///   - dataContractId: Base58 contested resource's data-contract id (DPNS
    ///     for username contests).
    ///   - documentTypeName: Contested document type (e.g. `"domain"`).
    ///   - indexName: Contested index name (e.g. `"parentNameAndLabel"`).
    ///   - indexValues: Index values identifying the contested resource
    ///     (e.g. `["dash", "alice"]`).
    ///   - choice: TowardsIdentity / Abstain / Lock.
    ///   - proTxHash: The masternode's 32-byte pro_tx_hash.
    ///   - votingPrivateKey: The masternode's 32-byte voting private key. The
    ///     matching `ECDSA_HASH160` voting public key and the signer are
    ///     derived from this on the Rust side; the key bytes are not retained.
    public func castContestedResourceVote(
        dataContractId: String,
        documentTypeName: String,
        indexName: String,
        indexValues: [String],
        choice: ContestedResourceVoteChoice,
        proTxHash: Data,
        votingPrivateKey: Data
    ) async throws {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        guard proTxHash.count == 32 else {
            throw SDKError.invalidParameter("proTxHash must be exactly 32 bytes")
        }
        guard votingPrivateKey.count == 32 else {
            throw SDKError.invalidParameter("votingPrivateKey must be exactly 32 bytes")
        }

        let indexValuesData = try JSONSerialization.data(withJSONObject: indexValues)
        let indexValuesJson = String(data: indexValuesData, encoding: .utf8) ?? "[]"

        let voteChoiceTag = choice.ffiTag
        let contenderId: String? = {
            if case let .towardsIdentity(id) = choice { return id }
            return nil
        }()
        let networkValue = network.ffiValue

        let result = proTxHash.withUnsafeBytes { proTxPtr in
            votingPrivateKey.withUnsafeBytes { keyPtr in
                dash_sdk_contested_resource_cast_vote(
                    handle,
                    dataContractId,
                    documentTypeName,
                    indexName,
                    indexValuesJson,
                    voteChoiceTag,
                    contenderId,
                    proTxPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    keyPtr.baseAddress?.assumingMemoryBound(to: UInt8.self),
                    networkValue
                )
            }
        }

        try processVoidResult(result)
    }

    /// Process a `DashSDKResult` that carries no payload on success (the
    /// broadcast / put-style FFIs return `NoData`). Throws on the error arm,
    /// otherwise returns. Frees the FFI-owned error string on the failure path.
    private func processVoidResult(_ result: DashSDKResult) throws {
        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }
    }

    /// Get vote polls by end date
    public func getVotePollsByEndDate(
        startTimeMs: UInt64?,
        endTimeMs: UInt64?,
        limit: UInt32?,
        offset: UInt32?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_voting_get_vote_polls_by_end_date(
            handle,
            startTimeMs ?? 0,
            true, // start_time_included
            endTimeMs ?? UInt64.max,
            true, // end_time_included
            limit ?? 100,
            offset ?? 0,
            orderAscending
        )
        return try processJSONArrayResult(result)
    }


    // MARK: - Protocol & Version Queries

    /// Get protocol version upgrade state
    public func getProtocolVersionUpgradeState() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_protocol_version_get_upgrade_state(handle)

        // Special handling for protocol version upgrade state which returns an array
        if let error = result.error {
            // Preserve the FFI error code so transport failures (e.g. an evonode
            // serving an expired TLS certificate) surface as .networkError
            // rather than a misleading .internalError.
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        // If no data, return empty result
        if result.data == nil {
            return ["upgrades": []]
        }

        let jsonArray = try processJSONArrayResult(result)
        return ["upgrades": jsonArray]
    }

    /// Get protocol version upgrade vote status
    public func getProtocolVersionUpgradeVoteStatus(startProTxHash: String?, count: UInt32?) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_protocol_version_get_upgrade_vote_status(handle, startProTxHash, count ?? 100)
        return try processJSONArrayResult(result)
    }

    // MARK: - Epoch & Block Queries

    /// Get epochs info
    public func getEpochsInfo(startEpoch: UInt32?, count: UInt32?, ascending: Bool) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let startEpochString = startEpoch.map { String($0) }
        let result = dash_sdk_system_get_epochs_info(handle, startEpochString, count ?? 100, ascending)
        return try processJSONArrayResult(result)
    }

    /// Get current epoch
    public func getCurrentEpoch() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Get current epoch info by passing nil as start_epoch to get the latest
        let result = dash_sdk_system_get_epochs_info(handle, nil, 1, true)
        let epochs = try processJSONArrayResult(result)

        guard let currentEpoch = epochs.first else {
            throw SDKError.notFound("Current epoch not found")
        }

        return currentEpoch
    }

    /// Get finalized epoch infos
    public func getFinalizedEpochInfos(startEpoch: UInt32?, count: UInt32?, ascending: Bool) async throws -> [[String: Any]] {
        // For now, use getEpochsInfo as they might be the same
        // The FFI might need a separate function for finalized epochs only
        return try await getEpochsInfo(startEpoch: startEpoch, count: count, ascending: ascending)
    }

    /// Get evonodes proposed epoch blocks by IDs
    public func getEvonodesProposedEpochBlocksByIds(epoch: UInt32, ids: [String]) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Convert IDs array to JSON
        let idsData = try JSONSerialization.data(withJSONObject: ids)
        let idsStr = String(data: idsData, encoding: .utf8) ?? "[]"

        let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_ids(handle, epoch, idsStr)
        return try processJSONArrayResult(result)
    }

    /// Get evonodes proposed epoch blocks by range
    public func getEvonodesProposedEpochBlocksByRange(
        epoch: UInt32,
        limit: UInt32?,
        startAfter: String?,
        orderAscending: Bool
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_evonode_get_proposed_epoch_blocks_by_range(
            handle,
            epoch,
            UInt32(limit ?? 100),
            startAfter,
            nil  // start_at parameter - not used in this implementation
        )
        return try processJSONArrayResult(result)
    }

    // MARK: - Token Queries

    /// Get identity token balances - get balances for multiple tokens for a single identity
    public func getIdentityTokenBalances(identityId: String, tokenIds: [String]) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Join token IDs with commas
        let tokenIdsStr = tokenIds.joined(separator: ",")

        let result = dash_sdk_token_get_identity_balances(handle, identityId, tokenIdsStr)
        let json = try processJSONResult(result)

        // Convert JSON object to [String: UInt64]
        var balances: [String: UInt64] = [:]
        for (tokenId, balance) in json {
            if let balanceNum = balance as? NSNumber {
                balances[tokenId] = balanceNum.uint64Value
            }
        }

        return balances
    }

    /// Get identities token balances
    public func getIdentitiesTokenBalances(identityIds: [String], tokenId: String) async throws -> [String: UInt64] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Join identity IDs with commas
        let identityIdsStr = identityIds.joined(separator: ",")

        let result = dash_sdk_identities_fetch_token_balances(handle, identityIdsStr, tokenId)
        let json = try processJSONResult(result)

        // Convert the result to [String: UInt64].
        // JSONSerialization decodes numbers as NSNumber, so read .uint64Value
        // (matching getIdentityTokenBalances above). The Rust FFI emits `null`
        // for identities that have never held the token; those decode to NSNull
        // and are skipped, mirroring how the single-identity query omits
        // never-held tokens.
        var balances: [String: UInt64] = [:]
        for (key, value) in json {
            if let balance = value as? NSNumber {
                balances[key] = balance.uint64Value
            }
        }

        return balances
    }

    /// Get identity token infos
    public func getIdentityTokenInfos(
        identityId: String,
        tokenIds: [String]?,
        limit: UInt32?,
        offset: UInt32?
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Convert token IDs to comma-separated string or nil
        let tokenIdsStr = tokenIds?.joined(separator: ",")

        let result = dash_sdk_identity_fetch_token_infos(handle, identityId, tokenIdsStr)
        return try processJSONArrayResult(result)
    }

    /// Get identities token infos
    public func getIdentitiesTokenInfos(identityIds: [String], tokenId: String) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Join identity IDs with commas
        let identityIdsStr = identityIds.joined(separator: ",")

        let result = dash_sdk_identities_fetch_token_infos(handle, identityIdsStr, tokenId)
        return try processJSONArrayResult(result)
    }

    /// Get token statuses
    public func getTokenStatuses(tokenIds: [String]) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Join token IDs with commas
        let tokenIdsStr = tokenIds.joined(separator: ",")

        let result = dash_sdk_token_get_statuses(handle, tokenIdsStr)
        return try processJSONResult(result)
    }

    /// Get token direct purchase prices
    public func getTokenDirectPurchasePrices(tokenIds: [String]) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Join token IDs with commas
        let tokenIdsStr = tokenIds.joined(separator: ",")

        let result = dash_sdk_token_get_direct_purchase_prices(handle, tokenIdsStr)
        return try processJSONResult(result)
    }

    /// Get token contract info
    public func getTokenContractInfo(tokenId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_token_get_contract_info(handle, tokenId)
        return try processJSONResult(result)
    }

    /// Derive the canonical platform token id for `(contractId, position)`.
    ///
    /// This bridges `dash_dpp::tokens::calculate_token_id` over FFI — the
    /// formula is a protocol constant (`double_sha256("dash_token" ||
    /// contract_id || u16_be(position))`), not something Swift should
    /// mirror. Returns base58.
    ///
    /// Pure CPU work (a hash) — `nonisolated` and `throws` only, no
    /// `async` and no SDK handle. Safe to call from any actor.
    public nonisolated func calculateTokenId(
        contractId: String,
        position: UInt16
    ) throws -> String {
        let result = contractId.withCString { cId in
            dash_sdk_calculate_token_id(cId, position)
        }

        if let error = result.error {
            let errorMessage = error.pointee.message != nil
                ? String(cString: error.pointee.message!)
                : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            throw SDKError.notFound("No data returned")
        }

        let string: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)
        return string
    }

    /// Get token perpetual distribution last claim
    public func getTokenPerpetualDistributionLastClaim(identityId: String, tokenId: String) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_token_get_perpetual_distribution_last_claim(handle, tokenId, identityId)

        // Special handling for this query - null means no claim found
        if let error = result.error {
            let errorMessage = error.pointee.message != nil ? String(cString: error.pointee.message!) : "Unknown error"
            dash_sdk_error_free(error)
            throw SDKError.internalError(errorMessage)
        }

        guard let dataPtr = result.data else {
            // No claim found - return empty dictionary
            return [:]
        }

        // Check if the pointer is null (no claim found)
        if dataPtr == UnsafeMutableRawPointer(bitPattern: 0) {
            return [:]
        }

        let jsonString: String = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)

        guard let data = jsonString.data(using: String.Encoding.utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw SDKError.serializationError("Failed to parse JSON data")
        }

        return json
    }

    /// Get token total supply
    public func getTokenTotalSupply(tokenId: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_token_get_total_supply(handle, tokenId)
        return try processUInt64Result(result)
    }

    // MARK: - Group Queries

    /// Get group info
    public func getGroupInfo(contractId: String, groupContractPosition: UInt32) async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_group_get_info(handle, contractId, UInt16(groupContractPosition))
        return try processJSONResult(result)
    }

    /// Get group infos
    public func getGroupInfos(
        contractId: String,
        startAtGroupContractPosition: UInt32?,
        startGroupContractPositionIncluded: Bool,
        count: UInt32?
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_group_get_infos(
            handle,
            startAtGroupContractPosition.map { String($0) },  // Convert UInt32 to String
            UInt32(count ?? 100)
        )
        return try processJSONArrayResult(result)
    }

    /// Get group actions
    public func getGroupActions(
        contractId: String,
        groupContractPosition: UInt32,
        status: String,
        startActionId: String?,
        startActionIdIncluded: Bool,
        count: UInt32?
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Convert status string to enum value
        let statusValue: UInt8 = status == "ACTIVE" ? 0 : 1

        let result = dash_sdk_group_get_actions(
            handle,
            contractId,
            UInt16(groupContractPosition),
            statusValue,
            startActionId,  // Pass the string directly
            UInt16(count ?? 100)
        )
        return try processJSONArrayResult(result)
    }

    /// Get group action signers
    public func getGroupActionSigners(
        contractId: String,
        groupContractPosition: UInt32,
        status: String,
        actionId: String
    ) async throws -> [[String: Any]] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Convert status string to enum value
        let statusValue: UInt8 = status == "ACTIVE" ? 0 : 1

        let result = dash_sdk_group_get_action_signers(
            handle,
            contractId,
            UInt16(groupContractPosition),
            statusValue,
            actionId
        )
        return try processJSONArrayResult(result)
    }

    // MARK: - System Queries

    /// Get platform status
    public func getStatus() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_get_platform_status(handle)
        return try processJSONResult(result)
    }

    /// Get total credits in platform
    public func getTotalCreditsInPlatform() async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_system_get_total_credits_in_platform(handle)
        return try processUInt64Result(result)
    }

    /// Get current quorums info
    public func getCurrentQuorumsInfo() async throws -> [String: Any] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_system_get_current_quorums_info(handle)
        return try processJSONResult(result)
    }

    /// Get prefunded specialized balance
    public func getPrefundedSpecializedBalance(id: String) async throws -> UInt64 {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        let result = dash_sdk_system_get_prefunded_specialized_balance(handle, id)
        return try processUInt64Result(result)
    }

    /// Fetch the raw GroveDB elements stored under `keys` within `path`.
    ///
    /// Thin bridge over `dash_sdk_system_get_path_elements`: it serializes
    /// the `path` / `keys` string arrays to JSON, passes them to the FFI,
    /// and marshals the
    /// `[{"key":"<hex>","element":"<formatted>","type":"<variant>"}]`
    /// payload out into ``PathElement`` rows. The FFI accepts each string as
    /// either hex-encoded bytes (preferred) or plain UTF-8 (hex-decode is
    /// tried first, with a raw-bytes fallback). All of the GroveDB traversal
    /// + formatting happens on the Rust side; nothing is decided here.
    ///
    /// - Parameters:
    ///   - path: GroveDB path segments. `[]` is the root. Each segment is
    ///     hex bytes or a plain string (e.g. `["60"]` for the Balances
    ///     subtree).
    ///   - keys: Keys to fetch within `path`, same hex-or-plain rule
    ///     (e.g. `["20","40","10","60"]` for the top-level RootTree subtrees).
    /// - Returns: The matching ``PathElement`` rows. Empty when the FFI
    ///   returns no elements (`NoData`).
    @MainActor
    public func systemPathElements(path: [String], keys: [String]) async throws -> [PathElement] {
        guard let handle = handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        // Serialize the path/keys string arrays to JSON for the FFI.
        let pathData = try JSONSerialization.data(withJSONObject: path)
        let keysData = try JSONSerialization.data(withJSONObject: keys)
        guard let pathJSON = String(data: pathData, encoding: .utf8),
              let keysJSON = String(data: keysData, encoding: .utf8) else {
            throw SDKError.serializationError("Failed to encode path/keys JSON")
        }

        let result = pathJSON.withCString { pathPtr in
            keysJSON.withCString { keysPtr in
                dash_sdk_system_get_path_elements(handle, pathPtr, keysPtr)
            }
        }

        // Surface FFI errors with their original code (network/timeout/etc.).
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        // NoData (null payload, no error) means no elements were found —
        // treat it as an empty array rather than an error.
        guard let dataPtr = result.data else {
            return []
        }

        let jsonString = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
        dash_sdk_string_free(dataPtr)

        guard let data = jsonString.data(using: .utf8),
              let array = try? JSONSerialization.jsonObject(with: data) as? [[String: Any]] else {
            throw SDKError.serializationError("Failed to parse path elements JSON array")
        }

        return array.map { entry in
            PathElement(
                key: entry["key"] as? String ?? "",
                element: entry["element"] as? String ?? "",
                type: entry["type"] as? String ?? ""
            )
        }
    }
}
