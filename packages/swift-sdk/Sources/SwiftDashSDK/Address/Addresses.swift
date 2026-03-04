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
    
    // MARK: - State Transitions
    
    /// Input for address transfer operation
    public struct AddressTransferInput {
        /// Address bytes (21 bytes: type byte + 20-byte hash)
        public let addressBytes: Data
        /// Amount to spend from this address in credits
        public let amount: UInt64
        /// Nonce for this address (0 = auto-fetch, used for identity transitions)
        public let nonce: UInt32
        /// Private key for signing (32 bytes)
        public let privateKey: Data
        
        public init(addressBytes: Data, amount: UInt64, nonce: UInt32 = 0, privateKey: Data) {
            self.addressBytes = addressBytes
            self.amount = amount
            self.nonce = nonce
            self.privateKey = privateKey
        }
    }
    
    /// Output for address transfer operation
    public struct AddressTransferOutput {
        /// Address bytes (21 bytes: type byte + 20-byte hash)
        public let addressBytes: Data
        /// Amount to receive at this address in credits
        public let amount: UInt64
        
        public init(addressBytes: Data, amount: UInt64) {
            self.addressBytes = addressBytes
            self.amount = amount
        }
    }
    
    /// Transfer funds between Platform addresses
    ///
    /// This is a state transition that moves credits from input addresses to output addresses.
    /// Each input address must have a corresponding private key for signing.
    ///
    /// - Parameters:
    ///   - inputs: Array of input addresses with amounts and private keys
    ///   - outputs: Array of output addresses with amounts
    ///   - feeFromInputIndex: Which input to deduct fees from (0-based, default 0)
    /// - Returns: PlatformAddressInfosResult containing updated address balances after transfer
    /// - Throws: SDKError if the transfer fails
    public func transferFunds(
        inputs: [AddressTransferInput],
        outputs: [AddressTransferOutput],
        feeFromInputIndex: UInt16 = 0
    ) throws -> PlatformAddressInfosResult {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard !inputs.isEmpty else {
            throw SDKError.invalidParameter("Inputs array is empty")
        }
        
        guard !outputs.isEmpty else {
            throw SDKError.invalidParameter("Outputs array is empty")
        }
        
        guard feeFromInputIndex < inputs.count else {
            throw SDKError.invalidParameter("Fee input index \(feeFromInputIndex) is out of bounds (inputs count: \(inputs.count))")
        }
        
        // Validate inputs
        for (index, input) in inputs.enumerated() {
            guard input.addressBytes.count == 21 else {
                throw SDKError.invalidParameter("Input address at index \(index) must be 21 bytes, got \(input.addressBytes.count)")
            }
            guard input.privateKey.count == 32 else {
                throw SDKError.invalidParameter("Private key at index \(index) must be 32 bytes, got \(input.privateKey.count)")
            }
        }
        
        // Validate outputs
        for (index, output) in outputs.enumerated() {
            guard output.addressBytes.count == 21 else {
                throw SDKError.invalidParameter("Output address at index \(index) must be 21 bytes, got \(output.addressBytes.count)")
            }
        }
        
        // Create FFI input structs
        var ffiInputs: [DashSDKAddressTransferInput] = []
        var inputData: [(address: Data, privateKey: Data)] = [] // Keep data alive
        
        for input in inputs {
            inputData.append((address: input.addressBytes, privateKey: input.privateKey))
        }
        
        for (index, data) in inputData.enumerated() {
            let addressPtr = data.address.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            let privateKeyPtr = data.privateKey.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            ffiInputs.append(DashSDKAddressTransferInput(
                address: addressPtr,
                address_len: UInt(data.address.count),
                amount: inputs[index].amount,
                nonce: inputs[index].nonce,
                private_key: privateKeyPtr
            ))
        }
        
        // Create FFI output structs
        var ffiOutputs: [DashSDKAddressTransferOutput] = []
        var outputData: [Data] = [] // Keep data alive
        
        for output in outputs {
            outputData.append(output.addressBytes)
        }
        
        for (index, data) in outputData.enumerated() {
            let addressPtr = data.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            ffiOutputs.append(DashSDKAddressTransferOutput(
                address: addressPtr,
                address_len: UInt(data.count),
                amount: outputs[index].amount
            ))
        }
        
        // Call FFI
        let result = ffiInputs.withUnsafeMutableBufferPointer { inputsBuffer -> DashSDKResult in
            ffiOutputs.withUnsafeMutableBufferPointer { outputsBuffer -> DashSDKResult in
                return dash_sdk_address_transfer_funds(
                    handle,
                    inputsBuffer.baseAddress,
                    UInt(inputs.count),
                    outputsBuffer.baseAddress,
                    UInt(outputs.count),
                    feeFromInputIndex
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
    
    /// Pooling strategy for withdrawals
    public enum PoolingStrategy {
        /// Never pool withdrawals
        case never
        /// Pool if available
        case ifAvailable
        /// Standard pooling
        case standard
        
        var ffiValue: DashSDKPooling {
            switch self {
            // DashSDKPooling is a C enum; Swift doesn't always import named cases.
            case .never: return DashSDKPooling(rawValue: 0)
            case .ifAvailable: return DashSDKPooling(rawValue: 1)
            case .standard: return DashSDKPooling(rawValue: 2)
            }
        }
    }
    
    /// Withdraw credits from Platform addresses to a Core (L1) Dash address
    ///
    /// This is a state transition that moves credits from Platform addresses to a Dash Core (L1) address.
    /// Each input address must have a corresponding private key for signing.
    ///
    /// - Parameters:
    ///   - inputs: Array of input addresses with amounts and private keys
    ///   - coreAddress: Base58-encoded Dash Core address to withdraw to (e.g., "y...")
    ///   - coreFeePerByte: Core network fee per byte (0 means use default of 1)
    ///   - pooling: Pooling strategy for the withdrawal (default: .never)
    ///   - feeFromInputIndex: Which input to deduct fees from (0-based, default 0)
    ///   - changeAddress: Optional Platform address for change (nil if not used)
    /// - Returns: PlatformAddressInfosResult containing updated address balances after withdrawal
    /// - Throws: SDKError if the withdrawal fails
    public func withdrawFunds(
        inputs: [AddressTransferInput],
        coreAddress: String,
        coreFeePerByte: UInt32 = 0,
        pooling: PoolingStrategy = .never,
        feeFromInputIndex: UInt16 = 0,
        changeAddress: Data? = nil
    ) throws -> PlatformAddressInfosResult {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard !inputs.isEmpty else {
            throw SDKError.invalidParameter("Inputs array is empty")
        }
        
        guard !coreAddress.isEmpty else {
            throw SDKError.invalidParameter("Core address is empty")
        }
        
        guard feeFromInputIndex < inputs.count else {
            throw SDKError.invalidParameter("Fee input index \(feeFromInputIndex) is out of bounds (inputs count: \(inputs.count))")
        }
        
        // Validate inputs
        for (index, input) in inputs.enumerated() {
            guard input.addressBytes.count == 21 else {
                throw SDKError.invalidParameter("Input address at index \(index) must be 21 bytes, got \(input.addressBytes.count)")
            }
            guard input.privateKey.count == 32 else {
                throw SDKError.invalidParameter("Private key at index \(index) must be 32 bytes, got \(input.privateKey.count)")
            }
        }
        
        // Validate change address if provided
        if let change = changeAddress {
            guard change.count == 21 else {
                throw SDKError.invalidParameter("Change address must be 21 bytes, got \(change.count)")
            }
        }
        
        // Create FFI input structs (same as transfer)
        var ffiInputs: [DashSDKAddressTransferInput] = []
        var inputData: [(address: Data, privateKey: Data)] = [] // Keep data alive
        
        for input in inputs {
            inputData.append((address: input.addressBytes, privateKey: input.privateKey))
        }
        
        for (index, data) in inputData.enumerated() {
            let addressPtr = data.address.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            let privateKeyPtr = data.privateKey.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            ffiInputs.append(DashSDKAddressTransferInput(
                address: addressPtr,
                address_len: UInt(data.address.count),
                amount: inputs[index].amount,
                nonce: inputs[index].nonce,
                private_key: privateKeyPtr
            ))
        }
        
        // Convert core address to C string
        let coreAddressCString = coreAddress.utf8CString
        
        // Prepare change address pointer
        var changeAddressPtr: UnsafePointer<UInt8>? = nil
        var changeAddressLen: UInt = 0
        if let change = changeAddress {
            changeAddressPtr = change.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            changeAddressLen = UInt(change.count)
        }
        
        // Call FFI
        let result = ffiInputs.withUnsafeMutableBufferPointer { inputsBuffer -> DashSDKResult in
            coreAddressCString.withUnsafeBufferPointer { coreAddressBuffer -> DashSDKResult in
                let coreAddressPtr = coreAddressBuffer.baseAddress
                return dash_sdk_address_withdraw_funds(
                    handle,
                    inputsBuffer.baseAddress,
                    UInt(inputs.count),
                    coreAddressPtr,
                    coreFeePerByte,
                    pooling.ffiValue,
                    feeFromInputIndex,
                    changeAddressPtr,
                    changeAddressLen
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
        
        // Parse DashSDKAddressInfoMap (same as transfer)
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
    
    /// Asset lock proof type
    public enum AssetLockProofType {
        /// Instant lock proof
        case instant
        /// Chain lock proof
        case chain
        
        var ffiValue: DashSDKAssetLockProofType {
            switch self {
            case .instant: return DashSDKAssetLockProofType(rawValue: 0)
            case .chain: return DashSDKAssetLockProofType(rawValue: 1)
            }
        }
    }
    
    /// Top up Platform addresses using an asset lock proof
    ///
    /// This is a state transition that funds Platform addresses from a Dash Core asset lock (instant or chain lock).
    ///
    /// - Parameters:
    ///   - proofType: Type of asset lock proof (.instant or .chain)
    ///   - instantLockData: For instant lock: instant lock bytes
    ///   - transactionData: For instant lock: transaction bytes
    ///   - outputIndex: For instant lock: output index in the transaction
    ///   - coreChainLockedHeight: For chain lock: core chain locked height
    ///   - outPoint: For chain lock: out point bytes (36 bytes: 32 txid + 4 vout)
    ///   - assetLockPrivateKey: Private key for the asset lock (32 bytes)
    ///   - outputs: Array of output addresses with optional amounts (nil means SDK calculates)
    ///   - feeFromInputIndex: Which input index to deduct fees from (0-based, default 0)
    /// - Returns: PlatformAddressInfosResult containing updated address balances after top-up
    /// - Throws: SDKError if the top-up fails
    public func topUpAddressFromAssetLock(
        proofType: AssetLockProofType,
        instantLockData: Data?,
        transactionData: Data?,
        outputIndex: UInt32,
        coreChainLockedHeight: UInt32,
        outPoint: Data?,
        assetLockPrivateKey: Data,
        outputs: [AddressTransferOutput],
        feeFromInputIndex: UInt16 = 0
    ) throws -> PlatformAddressInfosResult {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard assetLockPrivateKey.count == 32 else {
            throw SDKError.invalidParameter("Asset lock private key must be 32 bytes, got \(assetLockPrivateKey.count)")
        }
        
        guard !outputs.isEmpty else {
            throw SDKError.invalidParameter("Outputs array is empty")
        }
        
        // Validate proof type specific parameters
        switch proofType {
        case .instant:
            guard let instantLock = instantLockData, !instantLock.isEmpty else {
                throw SDKError.invalidParameter("Instant lock requires instantLockData")
            }
            guard let transaction = transactionData, !transaction.isEmpty else {
                throw SDKError.invalidParameter("Instant lock requires transactionData")
            }
        case .chain:
            guard let outPointData = outPoint, outPointData.count == 36 else {
                throw SDKError.invalidParameter("Chain lock requires outPoint with exactly 36 bytes (32 txid + 4 vout)")
            }
        }
        
        // Validate outputs
        for (index, output) in outputs.enumerated() {
            guard output.addressBytes.count == 21 else {
                throw SDKError.invalidParameter("Output address at index \(index) must be 21 bytes, got \(output.addressBytes.count)")
            }
        }
        
        // Prepare proof parameters
        let instantLockPtr: UnsafePointer<UInt8>?
        let instantLockLen: UInt
        let transactionPtr: UnsafePointer<UInt8>?
        let transactionLen: UInt
        
        if proofType == .instant, let instantLock = instantLockData, let transaction = transactionData {
            instantLockPtr = instantLock.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            instantLockLen = UInt(instantLock.count)
            transactionPtr = transaction.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            transactionLen = UInt(transaction.count)
        } else {
            instantLockPtr = nil
            instantLockLen = 0
            transactionPtr = nil
            transactionLen = 0
        }
        
        // Prepare chain lock parameters - C fixed-size arrays become tuples in Swift
        var outPointTuple: (UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                            UInt8, UInt8, UInt8, UInt8)?
        if proofType == .chain, let outPointData = outPoint {
            guard outPointData.count == 36 else {
                throw SDKError.invalidParameter("Out point must be exactly 36 bytes")
            }
            let bytes = Array(outPointData)
            outPointTuple = (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
                            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
                            bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
                            bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31],
                            bytes[32], bytes[33], bytes[34], bytes[35])
        }
        let outPointPtr = outPointTuple.map { withUnsafePointer(to: $0) { $0 } }
        
        // Create FFI output structs
        var ffiOutputs: [DashSDKAddressTransferOutput] = []
        var outputData: [Data] = [] // Keep data alive
        
        for output in outputs {
            outputData.append(output.addressBytes)
        }
        
        for (index, data) in outputData.enumerated() {
            let addressPtr = data.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            ffiOutputs.append(DashSDKAddressTransferOutput(
                address: addressPtr,
                address_len: UInt(data.count),
                amount: outputs[index].amount
            ))
        }
        
        // Prepare private key - C fixed-size arrays become tuples in Swift
        let privateKeyBytes = Array(assetLockPrivateKey)
        let privateKeyTuple = (privateKeyBytes[0], privateKeyBytes[1], privateKeyBytes[2], privateKeyBytes[3],
                               privateKeyBytes[4], privateKeyBytes[5], privateKeyBytes[6], privateKeyBytes[7],
                               privateKeyBytes[8], privateKeyBytes[9], privateKeyBytes[10], privateKeyBytes[11],
                               privateKeyBytes[12], privateKeyBytes[13], privateKeyBytes[14], privateKeyBytes[15],
                               privateKeyBytes[16], privateKeyBytes[17], privateKeyBytes[18], privateKeyBytes[19],
                               privateKeyBytes[20], privateKeyBytes[21], privateKeyBytes[22], privateKeyBytes[23],
                               privateKeyBytes[24], privateKeyBytes[25], privateKeyBytes[26], privateKeyBytes[27],
                               privateKeyBytes[28], privateKeyBytes[29], privateKeyBytes[30], privateKeyBytes[31])
        let privateKeyPtr = withUnsafePointer(to: privateKeyTuple) { $0 }
        
        // Call FFI
        let result = ffiOutputs.withUnsafeMutableBufferPointer { outputsBuffer -> DashSDKResult in
            dash_sdk_address_top_up_from_asset_lock(
                handle,
                proofType.ffiValue,
                instantLockPtr,
                instantLockLen,
                transactionPtr,
                transactionLen,
                outputIndex,
                coreChainLockedHeight,
                outPointPtr,
                privateKeyPtr,
                outputsBuffer.baseAddress,
                UInt(outputs.count),
                feeFromInputIndex,
                nil // put_settings
            )
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
        
        // Parse DashSDKAddressInfoMap (same as transfer)
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
    
    // MARK: - Identity State Transitions (Address-Related)
    
    /// Top up an identity using Platform address balances
    ///
    /// - Parameters:
    ///   - identityId: Base58-encoded identity ID
    ///   - inputs: Array of input addresses with amounts and private keys
    /// - Returns: Tuple of (updated identity balance, updated address infos)
    /// - Throws: SDKError if the operation fails
    /// - Note: This requires fetching the identity first to get a handle
    @MainActor
    public func topUpIdentityFromAddresses(
        identityId: String,
        inputs: [AddressTransferInput]
    ) async throws -> (identityBalance: UInt64, addressInfos: PlatformAddressInfosResult) {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard !inputs.isEmpty else {
            throw SDKError.invalidParameter("Inputs array is empty")
        }
        
        // Fetch identity JSON and parse to get handle
        let identityJSON = try await sdk.identityGet(identityId: identityId)
        guard let identityJSONString = try? JSONSerialization.data(withJSONObject: identityJSON),
              let jsonString = String(data: identityJSONString, encoding: .utf8) else {
            throw SDKError.serializationError("Failed to serialize identity JSON")
        }
        
        // Parse identity JSON to get handle
        let parseResult = jsonString.withCString { cString in
            dash_sdk_identity_parse_json(cString)
        }
        
        guard parseResult.error == nil else {
            let error = parseResult.error!.pointee
            defer { dash_sdk_error_free(parseResult.error) }
            throw SDKError.fromDashSDKError(error)
        }
        
        guard let identityHandlePtr = parseResult.data else {
            throw SDKError.internalError("Failed to parse identity")
        }
        
        let identityHandle = identityHandlePtr.assumingMemoryBound(to: IdentityHandle.self)
        defer { dash_sdk_identity_destroy(identityHandle) }
        
        // Prepare FFI inputs
        var ffiInputs: [DashSDKAddressTransferInput] = []
        var inputData: [Data] = [] // Keep data alive
        
        for input in inputs {
            inputData.append(input.addressBytes)
            inputData.append(input.privateKey)
        }
        
        for (index, input) in inputs.enumerated() {
            let addressPtr = inputData[index * 2].withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            let privateKeyPtr = inputData[index * 2 + 1].withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                guard buffer.count == 32 else { return nil }
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            guard let pkPtr = privateKeyPtr else {
                throw SDKError.invalidParameter("Invalid private key at index \(index)")
            }
            
            ffiInputs.append(DashSDKAddressTransferInput(
                address: addressPtr,
                address_len: UInt(input.addressBytes.count),
                amount: input.amount,
                nonce: input.nonce,
                private_key: pkPtr
            ))
        }
        
        // Call FFI
        let result = ffiInputs.withUnsafeMutableBufferPointer { inputsBuffer -> DashSDKResult in
            dash_sdk_identity_top_up_from_addresses(
                handle,
                UnsafePointer(identityHandle),
                inputsBuffer.baseAddress,
                UInt(inputs.count),
                nil // put_settings
            )
        }
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.internalError("No result data returned")
        }
        
        // Parse result
        let resultPtr = dataPtr.assumingMemoryBound(to: DashSDKIdentityTopUpFromAddressesResult.self)
        let ffiResult = resultPtr.pointee
        
        // Parse address info map
        var infos: [Data: PlatformAddressInfo] = [:]
        if ffiResult.address_info_map.count > 0 && ffiResult.address_info_map.entries != nil {
            for i in 0..<ffiResult.address_info_map.count {
                let entry = ffiResult.address_info_map.entries![Int(i)]
                
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
        
        // Free the result
        dash_sdk_identity_top_up_from_addresses_result_free(resultPtr)
        
        return (ffiResult.identity_balance, PlatformAddressInfosResult(infos: infos))
    }
    
    /// Transfer credits from an identity to Platform addresses
    ///
    /// - Parameters:
    ///   - identityId: Base58-encoded identity ID
    ///   - outputs: Array of output addresses with amounts
    ///   - identityPrivateKey: Private key for the identity (32 bytes) - used to create signer
    ///   - publicKeyId: ID of the public key to use for signing (0 = auto-select TRANSFER key)
    /// - Returns: Tuple of (updated identity balance, updated address infos)
    /// - Throws: SDKError if the operation fails
    @MainActor
    public func transferCreditsToAddresses(
        identityId: String,
        outputs: [AddressTransferOutput],
        identityPrivateKey: Data,
        publicKeyId: UInt32 = 0
    ) async throws -> (identityBalance: UInt64, addressInfos: PlatformAddressInfosResult) {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard identityPrivateKey.count == 32 else {
            throw SDKError.invalidParameter("Identity private key must be 32 bytes, got \(identityPrivateKey.count)")
        }
        
        guard !outputs.isEmpty else {
            throw SDKError.invalidParameter("Outputs array is empty")
        }
        
        // Fetch identity JSON and parse to get handle
        let identityJSON = try await sdk.identityGet(identityId: identityId)
        guard let identityJSONString = try? JSONSerialization.data(withJSONObject: identityJSON),
              let jsonString = String(data: identityJSONString, encoding: .utf8) else {
            throw SDKError.serializationError("Failed to serialize identity JSON")
        }
        
        // Parse identity JSON to get handle
        let parseResult = jsonString.withCString { cString in
            dash_sdk_identity_parse_json(cString)
        }
        
        guard parseResult.error == nil else {
            let error = parseResult.error!.pointee
            defer { dash_sdk_error_free(parseResult.error) }
            throw SDKError.fromDashSDKError(error)
        }
        
        guard let identityHandlePtr = parseResult.data else {
            throw SDKError.internalError("Failed to parse identity")
        }
        
        let identityHandle = identityHandlePtr.assumingMemoryBound(to: IdentityHandle.self)
        defer { dash_sdk_identity_destroy(identityHandle) }
        
        // Create signer from private key
        let signerResult = identityPrivateKey.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(identityPrivateKey.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            if let error = signerResult.error {
                let sdkError = SDKError.fromDashSDKError(error.pointee)
                dash_sdk_error_free(error)
                throw sdkError
            }
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(signer.assumingMemoryBound(to: SignerHandle.self))
        }
        
        // Prepare FFI outputs
        var ffiOutputs: [DashSDKAddressTransferOutput] = []
        var outputData: [Data] = [] // Keep data alive
        
        for output in outputs {
            outputData.append(output.addressBytes)
        }
        
        for (index, data) in outputData.enumerated() {
            let addressPtr = data.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            ffiOutputs.append(DashSDKAddressTransferOutput(
                address: addressPtr,
                address_len: UInt(data.count),
                amount: outputs[index].amount
            ))
        }
        
        // Call FFI
        let result = ffiOutputs.withUnsafeMutableBufferPointer { outputsBuffer -> DashSDKResult in
            dash_sdk_identity_transfer_credits_to_addresses(
                handle,
                UnsafePointer(identityHandle),
                outputsBuffer.baseAddress,
                UInt(outputs.count),
                publicKeyId,
                signer.assumingMemoryBound(to: SignerHandle.self),
                nil // put_settings
            )
        }
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.internalError("No result data returned")
        }
        
        // Parse result
        let resultPtr = dataPtr.assumingMemoryBound(to: DashSDKIdentityTransferToAddressesResult.self)
        let ffiResult = resultPtr.pointee
        
        // Parse address info map
        var infos: [Data: PlatformAddressInfo] = [:]
        if ffiResult.address_info_map.count > 0 && ffiResult.address_info_map.entries != nil {
            for i in 0..<ffiResult.address_info_map.count {
                let entry = ffiResult.address_info_map.entries![Int(i)]
                
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
        
        // Free the result
        dash_sdk_identity_transfer_to_addresses_result_free(resultPtr)
        
        return (ffiResult.identity_balance, PlatformAddressInfosResult(infos: infos))
    }
    
    /// Create an identity funded by Platform addresses
    ///
    /// - Parameters:
    ///   - identityId: Base58-encoded identity ID (must be prepared with public keys)
    ///   - inputs: Array of input addresses with amounts, nonces, and private keys
    ///   - output: Optional output address with amount (for change)
    ///   - identityPrivateKey: Private key for the identity (32 bytes) - used to create signer
    /// - Returns: Tuple of (created identity handle, updated address infos)
    /// - Throws: SDKError if the operation fails
    /// - Note: The returned identity handle must be freed using dash_sdk_identity_destroy
    @MainActor
    public func createIdentityFromAddresses(
        identityId: String,
        inputs: [AddressTransferInput],
        output: AddressTransferOutput?,
        identityPrivateKey: Data
    ) async throws -> (identityHandle: OpaquePointer, addressInfos: PlatformAddressInfosResult) {
        guard let sdk = sdk, let handle = sdk.handle else {
            throw SDKError.invalidState("SDK not initialized")
        }
        
        guard identityPrivateKey.count == 32 else {
            throw SDKError.invalidParameter("Identity private key must be 32 bytes, got \(identityPrivateKey.count)")
        }
        
        guard !inputs.isEmpty else {
            throw SDKError.invalidParameter("Inputs array is empty")
        }
        
        // Fetch identity JSON and parse to get handle
        let identityJSON = try await sdk.identityGet(identityId: identityId)
        guard let identityJSONString = try? JSONSerialization.data(withJSONObject: identityJSON),
              let jsonString = String(data: identityJSONString, encoding: .utf8) else {
            throw SDKError.serializationError("Failed to serialize identity JSON")
        }
        
        // Parse identity JSON to get handle
        let parseResult = jsonString.withCString { cString in
            dash_sdk_identity_parse_json(cString)
        }
        
        guard parseResult.error == nil else {
            let error = parseResult.error!.pointee
            defer { dash_sdk_error_free(parseResult.error) }
            throw SDKError.fromDashSDKError(error)
        }
        
        guard let identityHandlePtr = parseResult.data else {
            throw SDKError.internalError("Failed to parse identity")
        }
        
        let identityHandle = identityHandlePtr.assumingMemoryBound(to: IdentityHandle.self)
        // Note: We don't destroy this handle here because it will be replaced by the created identity
        
        // Create signer from private key
        let signerResult = identityPrivateKey.withUnsafeBytes { keyBytes in
            dash_sdk_signer_create_from_private_key(
                keyBytes.bindMemory(to: UInt8.self).baseAddress!,
                UInt(identityPrivateKey.count)
            )
        }
        
        guard signerResult.error == nil,
              let signer = signerResult.data else {
            dash_sdk_identity_destroy(identityHandle) // Clean up identity handle
            if let error = signerResult.error {
                let sdkError = SDKError.fromDashSDKError(error.pointee)
                dash_sdk_error_free(error)
                throw sdkError
            }
            throw SDKError.internalError("Failed to create signer")
        }
        
        defer {
            dash_sdk_signer_destroy(signer.assumingMemoryBound(to: SignerHandle.self))
        }
        
        // Prepare FFI inputs
        var ffiInputs: [DashSDKAddressTransferInput] = []
        var inputData: [(address: Data, privateKey: Data)] = [] // Keep data alive
        
        for input in inputs {
            inputData.append((address: input.addressBytes, privateKey: input.privateKey))
        }
        
        for (index, data) in inputData.enumerated() {
            let addressPtr = data.address.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            let privateKeyPtr = data.privateKey.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                guard buffer.count == 32 else { return nil }
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            guard let pkPtr = privateKeyPtr else {
                dash_sdk_identity_destroy(identityHandle) // Clean up
                throw SDKError.invalidParameter("Invalid private key at index \(index)")
            }
            
            ffiInputs.append(DashSDKAddressTransferInput(
                address: addressPtr,
                address_len: UInt(data.address.count),
                amount: inputs[index].amount,
                nonce: inputs[index].nonce,
                private_key: pkPtr
            ))
        }
        
        // Prepare optional output
        var ffiOutput: UnsafePointer<DashSDKAddressTransferOutput>? = nil
        var outputData: Data? = nil
        var outputPtr: UnsafePointer<UInt8>? = nil
        
        if let output = output {
            outputData = output.addressBytes
            outputPtr = outputData!.withUnsafeBytes { buffer -> UnsafePointer<UInt8>? in
                return buffer.baseAddress?.assumingMemoryBound(to: UInt8.self)
            }
            
            var outputStruct = DashSDKAddressTransferOutput(
                address: outputPtr,
                address_len: UInt(output.addressBytes.count),
                amount: output.amount
            )
            ffiOutput = withUnsafePointer(to: &outputStruct) { $0 }
        }
        
        // Call FFI
        let result = ffiInputs.withUnsafeMutableBufferPointer { inputsBuffer -> DashSDKResult in
            dash_sdk_identity_create_from_addresses(
                handle,
                UnsafePointer(identityHandle),
                inputsBuffer.baseAddress,
                UInt(inputs.count),
                ffiOutput,
                signer.assumingMemoryBound(to: SignerHandle.self),
                nil // put_settings
            )
        }
        
        // Clean up the original identity handle (it will be replaced)
        dash_sdk_identity_destroy(identityHandle)
        
        // Check for errors
        if let error = result.error {
            let sdkError = SDKError.fromDashSDKError(error.pointee)
            dash_sdk_error_free(error)
            throw sdkError
        }
        
        guard let dataPtr = result.data else {
            throw SDKError.internalError("No result data returned")
        }
        
        // Parse result
        let resultPtr = dataPtr.assumingMemoryBound(to: DashSDKIdentityCreateFromAddressesResult.self)
        let ffiResult = resultPtr.pointee
        
        // Parse address info map
        var infos: [Data: PlatformAddressInfo] = [:]
        if ffiResult.address_info_map.count > 0 && ffiResult.address_info_map.entries != nil {
            for i in 0..<ffiResult.address_info_map.count {
                let entry = ffiResult.address_info_map.entries![Int(i)]
                
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
        
        // The identity handle is in the result - extract it before freeing
        guard let identityHandlePtr = ffiResult.identity_handle else {
            dash_sdk_identity_create_from_addresses_result_free(resultPtr)
            throw SDKError.internalError("No identity handle returned from create operation")
        }
        
        // Free the result (but keep the identity handle - caller must free it)
        dash_sdk_identity_create_from_addresses_result_free(resultPtr)
        
        // Convert UnsafeMutablePointer<IdentityHandle> to OpaquePointer
        // OpaquePointer initializer returns optional, so we force unwrap since we know it's valid
        let createdIdentityHandle = OpaquePointer(UnsafeRawPointer(identityHandlePtr))!
        
        return (createdIdentityHandle, PlatformAddressInfosResult(infos: infos))
    }
}
