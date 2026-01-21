import Foundation
import DashSDKFFI

/// Service for fetching Platform address information
public class Addresses: @unchecked Sendable {
    private weak var sdk: SDK?

    init(sdk: SDK) {
        self.sdk = sdk
    }

    // MARK: - Single Address Query

    /// Fetch information about a single Platform address
    ///
    /// - Parameter addressBytes: Address bytes (21 bytes: type byte + 20-byte hash)
    /// - Returns: PlatformAddressInfo containing nonce and balance, or nil if address not found
    /// - Throws: SDKError if the query fails
    public func getInfo(addressBytes: Data) throws -> PlatformAddressInfo? {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard addressBytes.count == 21 else {
            throw SDKError.invalidParameter("Address bytes must be exactly 21 bytes (1 type + 20 hash), got \(addressBytes.count)")
        }

        let result = addressBytes.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) -> DashSDKResult in
            let ptr = buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            return dash_sdk_address_fetch_info(handle, ptr, UInt(addressBytes.count))
        }

        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        guard let dataPtr = result.data else {
            return nil
        }

        // Parse DashSDKAddressInfo
        let infoPtr = dataPtr.assumingMemoryBound(to: DashSDKAddressInfo.self)
        let ffiInfo = infoPtr.pointee
        let addressInfo = PlatformAddressInfo(from: ffiInfo)

        // Free the FFI struct
        dash_sdk_address_info_free(infoPtr)

        // Return nil if address not found (indicated by max values)
        if !addressInfo.isFound {
            return nil
        }

        return addressInfo
    }

    /// Fetch information about a single Platform address using hex string
    ///
    /// - Parameter addressHex: Hex-encoded address (42 characters for 21 bytes)
    /// - Returns: PlatformAddressInfo containing nonce and balance, or nil if address not found
    /// - Throws: SDKError if the query fails or hex is invalid
    public func getInfo(addressHex: String) throws -> PlatformAddressInfo? {
        guard let addressBytes = Data(hexString: addressHex) else {
            throw SDKError.invalidParameter("Invalid hex string for address")
        }
        return try getInfo(addressBytes: addressBytes)
    }

    /// Fetch information about a single Platform address using bech32m string
    ///
    /// - Parameter bech32mAddress: Bech32m-encoded address (e.g., "tdashevo1qqyfsqyzcn5hzu7echru54njypdq0v4d7gv8pkdf")
    /// - Returns: PlatformAddressInfo containing nonce and balance, or nil if address not found
    /// - Throws: SDKError if the query fails or bech32m is invalid
    public func getInfo(bech32mAddress: String) throws -> PlatformAddressInfo? {
        guard let decoded = Bech32m.decode(bech32mAddress) else {
            throw SDKError.invalidParameter("Invalid bech32m address")
        }
        guard decoded.data.count == 21 else {
            throw SDKError.invalidParameter("Invalid Platform address: expected 21 bytes, got \(decoded.data.count)")
        }
        return try getInfo(addressBytes: decoded.data)
    }

    /// Fetch information about a single Platform address (auto-detects format)
    ///
    /// - Parameter address: Address string - can be hex (42 chars) or bech32m (tdashevo1.../dashevo1...)
    /// - Returns: PlatformAddressInfo containing nonce and balance, or nil if address not found
    /// - Throws: SDKError if the query fails or address format is invalid
    public func getInfo(address: String) throws -> PlatformAddressInfo? {
        let trimmed = address.trimmingCharacters(in: .whitespacesAndNewlines)

        // Check if it's a bech32m address (starts with dashevo1 or tdashevo1)
        if trimmed.lowercased().hasPrefix("dashevo1") || trimmed.lowercased().hasPrefix("tdashevo1") {
            return try getInfo(bech32mAddress: trimmed)
        }

        // Otherwise try as hex
        return try getInfo(addressHex: trimmed)
    }

    // MARK: - Multiple Addresses Query

    /// Fetch information about multiple Platform addresses
    ///
    /// - Parameter addressesBytesList: Array of address bytes (each 21 bytes)
    /// - Returns: PlatformAddressInfosResult containing info for all queried addresses
    /// - Throws: SDKError if the query fails
    public func getInfos(addressesBytesList: [Data]) throws -> PlatformAddressInfosResult {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }

        guard !addressesBytesList.isEmpty else {
            return PlatformAddressInfosResult(infos: [:])
        }

        // Validate all addresses
        for (index, bytes) in addressesBytesList.enumerated() {
            guard bytes.count == 21 else {
                throw SDKError.invalidParameter("Address at index \(index) must be exactly 21 bytes, got \(bytes.count)")
            }
        }

        // Prepare arrays for FFI call
        var addressPointers: [UnsafePointer<UInt8>?] = []
        var addressLengths: [UInt] = []
        var addressData: [Data] = [] // Keep data alive during the call

        for bytes in addressesBytesList {
            addressData.append(bytes)
        }

        // Create pointers
        for data in addressData {
            let pointer = data.withUnsafeBytes { (buffer: UnsafeRawBufferPointer) -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            addressPointers.append(pointer)
            addressLengths.append(UInt(data.count))
        }

        // Call FFI
        let result = addressPointers.withUnsafeBufferPointer { pointersBuffer -> DashSDKResult in
            addressLengths.withUnsafeBufferPointer { lengthsBuffer -> DashSDKResult in
                return dash_sdk_addresses_fetch_infos(
                    handle,
                    pointersBuffer.baseAddress,
                    lengthsBuffer.baseAddress,
                    UInt(addressesBytesList.count)
                )
            }
        }

        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }

        guard let dataPtr = result.data else {
            return PlatformAddressInfosResult(infos: [:])
        }

        // Parse DashSDKAddressInfoMap
        let mapPtr = dataPtr.assumingMemoryBound(to: DashSDKAddressInfoMap.self)
        let map = mapPtr.pointee

        var infos: [Data: PlatformAddressInfo] = [:]

        if map.count > 0 && map.entries != nil {
            for i in 0..<map.count {
                let entry = map.entries![Int(i)]

                let addressBytes: Data
                if entry.address != nil && entry.address_len > 0 {
                  addressBytes = Data(bytes: entry.address!, count: Int(entry.address_len))
                } else {
                    continue
                }

                let info = PlatformAddressInfo(
                    addressBytes: addressBytes,
                    nonce: entry.nonce,
                    balance: entry.balance
                )

                infos[addressBytes] = info
            }
        }

        // Free the FFI map
        dash_sdk_address_info_map_free(mapPtr)

        return PlatformAddressInfosResult(infos: infos)
    }

    /// Fetch information about multiple Platform addresses using hex strings
    ///
    /// - Parameter addressHexList: Array of hex-encoded addresses
    /// - Returns: PlatformAddressInfosResult containing info for all queried addresses
    /// - Throws: SDKError if the query fails or any hex is invalid
    public func getInfos(addressHexList: [String]) throws -> PlatformAddressInfosResult {
        let addressesBytesList = try addressHexList.enumerated().map { (index, hex) -> Data in
            guard let bytes = Data(hexString: hex) else {
                throw SDKError.invalidParameter("Invalid hex string at index \(index)")
            }
            return bytes
        }
        return try getInfos(addressesBytesList: addressesBytesList)
    }

    /// Fetch information about multiple Platform addresses using bech32m strings
    ///
    /// - Parameter bech32mAddresses: Array of bech32m-encoded addresses
    /// - Returns: PlatformAddressInfosResult containing info for all queried addresses
    /// - Throws: SDKError if the query fails or any bech32m is invalid
    public func getInfos(bech32mAddresses: [String]) throws -> PlatformAddressInfosResult {
        let addressesBytesList = try bech32mAddresses.enumerated().map { (index, bech32m) -> Data in
            guard let decoded = Bech32m.decode(bech32m) else {
                throw SDKError.invalidParameter("Invalid bech32m address at index \(index)")
            }
            guard decoded.data.count == 21 else {
                throw SDKError.invalidParameter("Invalid Platform address at index \(index): expected 21 bytes")
            }
            return decoded.data
        }
        return try getInfos(addressesBytesList: addressesBytesList)
    }

    /// Fetch information about multiple Platform addresses (auto-detects format)
    ///
    /// - Parameter addresses: Array of address strings - can be hex or bech32m (mixed formats allowed)
    /// - Returns: PlatformAddressInfosResult containing info for all queried addresses
    /// - Throws: SDKError if the query fails or any address format is invalid
    public func getInfos(addresses: [String]) throws -> PlatformAddressInfosResult {
        let addressesBytesList = try addresses.enumerated().map { (index, address) -> Data in
            let trimmed = address.trimmingCharacters(in: .whitespacesAndNewlines)

            // Check if it's a bech32m address
            if trimmed.lowercased().hasPrefix("dashevo1") || trimmed.lowercased().hasPrefix("tdashevo1") {
                guard let decoded = Bech32m.decode(trimmed) else {
                    throw SDKError.invalidParameter("Invalid bech32m address at index \(index)")
                }
                guard decoded.data.count == 21 else {
                    throw SDKError.invalidParameter("Invalid Platform address at index \(index): expected 21 bytes")
                }
                return decoded.data
            }

            // Otherwise try as hex
            guard let bytes = Data(hexString: trimmed) else {
                throw SDKError.invalidParameter("Invalid address format at index \(index)")
            }
            return bytes
        }
        return try getInfos(addressesBytesList: addressesBytesList)
    }

    // MARK: - Trunk State Query
    
    /// Fetch the trunk state of the address tree for privacy-preserving address synchronization.
    ///
    /// The trunk state contains:
    /// - Elements: Addresses with balances found at the top levels of the tree
    /// - Leaf boundaries: Subtrees that require further branch queries to explore
    ///
    /// This is a low-level API used for privacy-preserving address synchronization.
    /// Most applications should use the higher-level sync methods instead.
    ///
    /// - Returns: PlatformTrunkState containing elements and leaf boundaries
    /// - Throws: SDKError if the query fails
    public func getTrunkState() throws -> PlatformTrunkState {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_address_fetch_trunk_state(handle)
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.invalidState("No trunk state data returned")
        }
        
        // Parse DashSDKTrunkState
        let statePtr = dataPtr.assumingMemoryBound(to: DashSDKTrunkState.self)
        let ffiState = statePtr.pointee
        
        // Convert elements
        var elements: [TrunkStateElement] = []
        if ffiState.elements_count > 0 && ffiState.elements != nil {
            for i in 0..<ffiState.elements_count {
                let ffiElement = ffiState.elements![Int(i)]
                
                let keyData: Data
                if ffiElement.key != nil && ffiElement.key_len > 0 {
                    keyData = Data(bytes: ffiElement.key!, count: Int(ffiElement.key_len))
                } else {
                    continue
                }
                
                elements.append(TrunkStateElement(
                    key: keyData,
                    nonce: ffiElement.nonce,
                    balance: ffiElement.balance
                ))
            }
        }
        
        // Convert leaf boundaries
        var leafBoundaries: [LeafBoundary] = []
        if ffiState.leaf_boundaries_count > 0 && ffiState.leaf_boundaries != nil {
            for i in 0..<ffiState.leaf_boundaries_count {
                let ffiBoundary = ffiState.leaf_boundaries![Int(i)]
                
                let keyData: Data
                if ffiBoundary.key != nil && ffiBoundary.key_len > 0 {
                    keyData = Data(bytes: ffiBoundary.key!, count: Int(ffiBoundary.key_len))
                } else {
                    continue
                }
                
                // Convert fixed-size array to Data
                var hashArray = ffiBoundary.hash
                let hashData = Data(bytes: &hashArray, count: 32)
                
                leafBoundaries.append(LeafBoundary(
                    key: keyData,
                    hash: hashData,
                    estimatedCount: ffiBoundary.estimated_count
                ))
            }
        }
        
        let checkpointHeight = ffiState.checkpoint_height
        
        // Free the FFI struct
        dash_sdk_trunk_state_free(statePtr)
        
        return PlatformTrunkState(
            elements: elements,
            leafBoundaries: leafBoundaries,
            checkpointHeight: checkpointHeight
        )
    }
    
    // MARK: - Branch State Query
    
    /// Fetch the branch state of a subtree in the address tree.
    ///
    /// This is used after a trunk state query to explore subtrees indicated by leaf boundaries.
    /// The result contains elements (addresses with balances) and deeper leaf boundaries.
    ///
    /// - Parameters:
    ///   - key: Leaf boundary key bytes from trunk state
    ///   - depth: Query depth (how deep to explore)
    ///   - expectedHash: Expected hash of the subtree root (32 bytes, for proof verification)
    ///   - checkpointHeight: Block height from trunk state response for consistency
    /// - Returns: PlatformBranchState containing elements and leaf boundaries
    /// - Throws: SDKError if the query fails
    public func getBranchState(
        key: Data,
        depth: UInt32,
        expectedHash: Data,
        checkpointHeight: UInt64
    ) throws -> PlatformBranchState {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard expectedHash.count == 32 else {
            throw SDKError.invalidParameter("Expected hash must be exactly 32 bytes, got \(expectedHash.count)")
        }
        
        let result = key.withUnsafeBytes { (keyBuffer: UnsafeRawBufferPointer) -> DashSDKResult in
            expectedHash.withUnsafeBytes { (hashBuffer: UnsafeRawBufferPointer) -> DashSDKResult in
                let keyPtr = keyBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
                let hashPtr = hashBuffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
                return dash_sdk_address_fetch_branch_state(
                    handle,
                    keyPtr,
                    UInt(key.count),
                    depth,
                    hashPtr,
                    checkpointHeight
                )
            }
        }
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.invalidState("No branch state data returned")
        }
        
        // Parse DashSDKBranchState
        let statePtr = dataPtr.assumingMemoryBound(to: DashSDKBranchState.self)
        let ffiState = statePtr.pointee
        
        // Convert elements (same structure as trunk state)
        var elements: [TrunkStateElement] = []
        if ffiState.elements_count > 0 && ffiState.elements != nil {
            for i in 0..<ffiState.elements_count {
                let ffiElement = ffiState.elements![Int(i)]
                
                let keyData: Data
                if ffiElement.key != nil && ffiElement.key_len > 0 {
                    keyData = Data(bytes: ffiElement.key!, count: Int(ffiElement.key_len))
                } else {
                    continue
                }
                
                elements.append(TrunkStateElement(
                    key: keyData,
                    nonce: ffiElement.nonce,
                    balance: ffiElement.balance
                ))
            }
        }
        
        // Convert leaf boundaries
        var leafBoundaries: [LeafBoundary] = []
        if ffiState.leaf_boundaries_count > 0 && ffiState.leaf_boundaries != nil {
            for i in 0..<ffiState.leaf_boundaries_count {
                let ffiBoundary = ffiState.leaf_boundaries![Int(i)]
                
                let boundaryKeyData: Data
                if ffiBoundary.key != nil && ffiBoundary.key_len > 0 {
                    boundaryKeyData = Data(bytes: ffiBoundary.key!, count: Int(ffiBoundary.key_len))
                } else {
                    continue
                }
                
                // Convert fixed-size array to Data
                var hashArray = ffiBoundary.hash
                let hashData = Data(bytes: &hashArray, count: 32)
                
                leafBoundaries.append(LeafBoundary(
                    key: boundaryKeyData,
                    hash: hashData,
                    estimatedCount: ffiBoundary.estimated_count
                ))
            }
        }
        
        // Free the FFI struct
        dash_sdk_branch_state_free(statePtr)
        
        return PlatformBranchState(
            elements: elements,
            leafBoundaries: leafBoundaries
        )
    }
    
    // MARK: - Recent Balance Changes Query
    
    /// Fetch recent address balance changes starting from a specific block height.
    ///
    /// This returns all address balance changes that occurred since the specified start height.
    /// Useful for syncing wallet balances after the initial sync.
    ///
    /// - Parameter startHeight: Block height to start fetching changes from
    /// - Returns: RecentBalanceChanges containing block-by-block changes
    /// - Throws: SDKError if the query fails
    public func getRecentBalanceChanges(startHeight: UInt64) throws -> RecentBalanceChanges {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_address_fetch_recent_balance_changes(handle, startHeight)
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            // No changes found - return empty result
            return RecentBalanceChanges(blocks: [])
        }
        
        // Parse DashSDKRecentBalanceChanges
        let changesPtr = dataPtr.assumingMemoryBound(to: DashSDKRecentBalanceChanges.self)
        let ffiChanges = changesPtr.pointee
        
        // Convert blocks
        var blocks: [BlockBalanceChanges] = []
        if ffiChanges.blocks_count > 0 && ffiChanges.blocks != nil {
            for i in 0..<ffiChanges.blocks_count {
                let ffiBlock = ffiChanges.blocks![Int(i)]
                
                // Convert address changes within this block
                var addressChanges: [AddressBalanceChange] = []
                if ffiBlock.changes_count > 0 && ffiBlock.changes != nil {
                    for j in 0..<ffiBlock.changes_count {
                        let ffiChange = ffiBlock.changes![Int(j)]
                        
                        let addressData: Data
                        if ffiChange.address != nil && ffiChange.address_len > 0 {
                            addressData = Data(bytes: ffiChange.address!, count: Int(ffiChange.address_len))
                        } else {
                            continue
                        }
                        
                        // Map operation type: 0 = SetCredits, 1 = AddToCredits
                        let operation: CreditOperationType
                        if ffiChange.operation_type.rawValue == 0 {
                            operation = .setCredits(credits: ffiChange.credits)
                        } else {
                            operation = .addToCredits(credits: ffiChange.credits)
                        }
                        
                        addressChanges.append(AddressBalanceChange(
                            addressBytes: addressData,
                            operation: operation
                        ))
                    }
                }
                
                blocks.append(BlockBalanceChanges(
                    blockHeight: ffiBlock.block_height,
                    changes: addressChanges
                ))
            }
        }
        
        // Free the FFI struct
        dash_sdk_recent_balance_changes_free(changesPtr)
        
        return RecentBalanceChanges(blocks: blocks)
    }
    
    // MARK: - Compacted Balance Changes Query
    
    /// Fetch recent compacted address balance changes starting from a specific block height.
    ///
    /// This returns compacted (merged) address balance changes since the specified start height.
    /// Compacted changes merge multiple blocks into ranges, which is more efficient for syncing.
    /// The BlockAwareCreditOperation preserves per-block granularity for partial sync.
    ///
    /// - Parameter startBlockHeight: Block height to start fetching changes from
    /// - Returns: CompactedBalanceChanges containing range-by-range compacted changes
    /// - Throws: SDKError if the query fails
    public func getCompactedBalanceChanges(startBlockHeight: UInt64) throws -> CompactedBalanceChanges {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        let result = dash_sdk_address_fetch_compacted_balance_changes(handle, startBlockHeight)
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            // No changes found - return empty result
            return CompactedBalanceChanges(ranges: [])
        }
        
        // Parse DashSDKCompactedBalanceChanges
        let changesPtr = dataPtr.assumingMemoryBound(to: DashSDKCompactedBalanceChanges.self)
        let ffiChanges = changesPtr.pointee
        
        // Convert ranges
        var ranges: [CompactedBlockRange] = []
        if ffiChanges.ranges_count > 0 && ffiChanges.ranges != nil {
            for i in 0..<ffiChanges.ranges_count {
                let ffiRange = ffiChanges.ranges![Int(i)]
                
                // Convert address changes within this range
                var addressChanges: [CompactedAddressChange] = []
                if ffiRange.changes_count > 0 && ffiRange.changes != nil {
                    for j in 0..<ffiRange.changes_count {
                        let ffiChange = ffiRange.changes![Int(j)]
                        
                        let addressData: Data
                        if ffiChange.address != nil && ffiChange.address_len > 0 {
                            addressData = Data(bytes: ffiChange.address!, count: Int(ffiChange.address_len))
                        } else {
                            continue
                        }
                        
                        // Map operation type: 0 = BlockAwareSetCredits, 1 = BlockAwareAddToCreditsOperations
                        let operation: BlockAwareCreditOperation
                        if ffiChange.operation_type.rawValue == 0 { // BlockAwareSetCredits
                            operation = .setCredits(credits: ffiChange.set_credits_value)
                        } else { // BlockAwareAddToCreditsOperations
                            // Parse add entries
                            var entries: [(blockHeight: UInt64, credits: UInt64)] = []
                            if ffiChange.add_entries_count > 0 && ffiChange.add_entries != nil {
                                for k in 0..<ffiChange.add_entries_count {
                                    let entry = ffiChange.add_entries![Int(k)]
                                    entries.append((blockHeight: entry.block_height, credits: entry.credits))
                                }
                            }
                            operation = .addToCreditsOperations(entries: entries)
                        }
                        
                        addressChanges.append(CompactedAddressChange(
                            addressBytes: addressData,
                            operation: operation
                        ))
                    }
                }
                
                ranges.append(CompactedBlockRange(
                    startBlockHeight: ffiRange.start_block_height,
                    endBlockHeight: ffiRange.end_block_height,
                    changes: addressChanges
                ))
            }
        }
        
        // Free the FFI struct
        dash_sdk_compacted_balance_changes_free(changesPtr)
        
        return CompactedBalanceChanges(ranges: ranges)
    }
    
    // MARK: - Convenience Methods

    /// Get the balance for a single address
    ///
    /// - Parameter addressBytes: Address bytes (21 bytes)
    /// - Returns: Balance in credits, or nil if address not found
    /// - Throws: SDKError if the query fails
    public func getBalance(addressBytes: Data) throws -> UInt64? {
        return try getInfo(addressBytes: addressBytes)?.balance
    }

    /// Get the nonce for a single address
    ///
    /// - Parameter addressBytes: Address bytes (21 bytes)
    /// - Returns: Nonce value, or nil if address not found
    /// - Throws: SDKError if the query fails
    public func getNonce(addressBytes: Data) throws -> UInt32? {
        return try getInfo(addressBytes: addressBytes)?.nonce
    }

    /// Check if an address exists on Platform
    ///
    /// - Parameter addressBytes: Address bytes (21 bytes)
    /// - Returns: true if the address has been used on Platform
    /// - Throws: SDKError if the query fails
    public func exists(addressBytes: Data) throws -> Bool {
        return try getInfo(addressBytes: addressBytes) != nil
    }

    /// Get total balance across multiple addresses
    ///
    /// - Parameter addressesBytesList: Array of address bytes
    /// - Returns: Total balance in credits across all found addresses
    /// - Throws: SDKError if the query fails
    public func getTotalBalance(addressesBytesList: [Data]) throws -> UInt64 {
        let result = try getInfos(addressesBytesList: addressesBytesList)
        return result.totalBalance
    }
}
