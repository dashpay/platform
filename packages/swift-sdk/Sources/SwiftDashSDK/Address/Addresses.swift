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
