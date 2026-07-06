import Foundation

// MARK: - Address Transformer

/// Transforms and converts address data between different formats (hex, Data, bech32m).
public enum AddressTransformer {

    /// Converts a hex string to Data.
    /// - Parameter hex: Hex string (must have even number of characters).
    /// - Returns: Data if conversion succeeds, nil otherwise.
    public static func hexToData(_ hex: String) -> Data? {
        let trimmed = hex.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty, trimmed.count % 2 == 0 else { return nil }
        return Data(hexString: trimmed)
    }

    /// Converts Data to a lowercase hex string.
    /// - Parameter data: The data to convert.
    /// - Returns: Hex string representation.
    public static func dataToHex(_ data: Data) -> String {
        data.toHexString()
    }

    /// Normalizes an identity ID to Data, handling both base58 and hex formats.
    /// - Parameter id: Identity ID in either base58 or hex format.
    /// - Returns: 32-byte Data if valid, nil otherwise.
    public static func normalizeIdentityId(_ id: String) -> Data? {
        let trimmed = id.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }

        // Check if it's a valid 64-character hex string (32 bytes)
        if AddressValidator.isHexIdentityId(trimmed) {
            return hexToData(trimmed)
        }

        // Try base58 decoding
        if let data = Data.identifier(fromBase58: trimmed), data.count == 32 {
            return data
        }

        return nil
    }

    /// Parses a bech32m Platform address to its raw storage/FFI address bytes.
    /// - Parameter address: Bech32m address string (dash1... or tdash1...).
    /// - Returns: Storage-form address bytes (21 bytes, type byte 0x00/0x01) if
    ///   valid, nil otherwise.
    public static func parseBech32mAddress(_ address: String) -> Data? {
        let trimmed = address.trimmingCharacters(in: .whitespacesAndNewlines)
        guard let result = Bech32m.decode(trimmed),
              (result.hrp == Bech32m.platformHrpMainnet || result.hrp == Bech32m.platformHrpTestnet) else {
            return nil
        }
        return Bech32m.storageBytes(fromBech32mPayload: result.data)
    }

    /// Formats address bytes for display, optionally as bech32m.
    /// - Parameters:
    ///   - data: Address bytes (21 bytes for Platform address).
    ///   - asBech32m: If true, encodes as bech32m; otherwise returns hex.
    ///   - isTestnet: If asBech32m is true, determines testnet (tdash1) vs mainnet (dash1).
    /// - Returns: Formatted address string.
    public static func formatAddress(_ data: Data, asBech32m: Bool = false, isTestnet: Bool = true) -> String {
        if asBech32m, let payload = Bech32m.bech32mPayload(fromStorageBytes: data) {
            let hrp = Bech32m.platformHrp(mainnet: !isTestnet)
            return Bech32m.encode(hrp: hrp, data: payload) ?? dataToHex(data)
        }
        return dataToHex(data)
    }

    /// Parses an address string in either hex or bech32m format to Data.
    /// - Parameter address: Address string in hex (42 chars) or bech32m format.
    /// - Returns: Address bytes if valid, nil otherwise.
    public static func parseAddress(_ address: String) -> Data? {
        let trimmed = address.trimmingCharacters(in: .whitespacesAndNewlines)

        // Check if it's a bech32m address
        if Bech32m.looksLikePlatformAddress(trimmed) {
            return parseBech32mAddress(trimmed)
        }

        // Try hex format (42 characters = 21 bytes)
        if trimmed.count == 42 {
            return hexToData(trimmed)
        }

        return nil
    }
}

// MARK: - Number Transformer

/// Transforms and validates numeric strings for SDK operations.
public enum NumberTransformer {

    /// Parses a string to UInt64.
    /// - Parameter string: String representation of a number.
    /// - Returns: UInt64 if valid, nil otherwise.
    public static func parseUInt64(_ string: String) -> UInt64? {
        UInt64(string.trimmingCharacters(in: .whitespaces))
    }

    /// Parses a string to UInt32.
    /// - Parameter string: String representation of a number.
    /// - Returns: UInt32 if valid, nil otherwise.
    public static func parseUInt32(_ string: String) -> UInt32? {
        UInt32(string.trimmingCharacters(in: .whitespaces))
    }

    /// Parses an amount string, ensuring it's a positive number.
    /// - Parameter amount: Amount string.
    /// - Returns: UInt64 if valid positive number, nil otherwise.
    public static func parseAmount(_ amount: String) -> UInt64? {
        guard let value = parseUInt64(amount), value > 0 else {
            return nil
        }
        return value
    }

    /// Parses a fee string to UInt32.
    /// - Parameter fee: Fee string (e.g., fee per byte).
    /// - Returns: UInt32 if valid, nil otherwise.
    public static func parseFee(_ fee: String) -> UInt32? {
        parseUInt32(fee)
    }

    /// Formats an amount in duffs/credits for display.
    /// - Parameters:
    ///   - amount: Amount in smallest unit (duffs or credits).
    ///   - unit: Unit name for display (default: "credits").
    /// - Returns: Formatted string with unit.
    public static func formatAmount(_ amount: UInt64, unit: String = "credits") -> String {
        let formatter = NumberFormatter()
        formatter.numberStyle = .decimal
        formatter.groupingSeparator = ","
        let formatted = formatter.string(from: NSNumber(value: amount)) ?? "\(amount)"
        return "\(formatted) \(unit)"
    }

    /// Formats an amount converting from credits to Dash.
    /// 1 Dash = 100,000,000,000 (10^11) credits.
    /// - Parameter credits: Amount in credits.
    /// - Returns: Formatted string in Dash.
    public static func formatCreditsAsDash(_ credits: UInt64) -> String {
        let dash = Double(credits) / 100_000_000_000.0
        return String(format: "%.8f Dash", dash)
    }

    /// Formats an amount converting from duffs to Dash.
    /// 1 Dash = 100,000,000 (10^8) duffs.
    /// - Parameter duffs: Amount in duffs.
    /// - Returns: Formatted string in Dash.
    public static func formatDuffsAsDash(_ duffs: UInt64) -> String {
        let dash = Double(duffs) / 100_000_000.0
        return String(format: "%.8f Dash", dash)
    }
}

// MARK: - Response Parser

/// Parses and extracts data from SDK response objects.
public enum ResponseParser {

    /// Extracts address info from a PlatformAddressInfo result.
    /// - Parameter info: The platform address info object.
    /// - Returns: A tuple with balance and nonce.
    public static func parseAddressInfo(_ info: PlatformAddressInfo) -> (balance: UInt64, nonce: UInt32) {
        (balance: info.balance, nonce: info.nonce)
    }

    /// Extracts results from a PlatformAddressInfosResult.
    /// - Parameter result: The result object from a transfer or top-up operation.
    /// - Returns: Array of address info tuples (address hex, balance, nonce).
    public static func parseAddressInfosResult(_ result: PlatformAddressInfosResult) -> [(address: String, balance: UInt64, nonce: UInt32)] {
        result.infos.map { (addressData, info) in
            (
                address: AddressTransformer.dataToHex(addressData),
                balance: info.balance,
                nonce: info.nonce
            )
        }
    }

    /// Checks if a transfer result indicates success.
    /// - Parameter result: The result from a transfer operation.
    /// - Returns: True if the transfer was successful (has addresses).
    public static func isTransferSuccessful(_ result: PlatformAddressInfosResult) -> Bool {
        !result.infos.isEmpty
    }

    /// Formats a PlatformAddressInfo for display.
    /// - Parameters:
    ///   - info: The address info to format.
    ///   - includeAddress: Whether to include the address in the output.
    /// - Returns: Formatted string.
    public static func formatAddressInfo(_ info: PlatformAddressInfo, includeAddress: Bool = true) -> String {
        var parts: [String] = []
        if includeAddress {
            parts.append("Address: \(AddressTransformer.dataToHex(info.addressBytes))")
        }
        parts.append("Balance: \(NumberTransformer.formatAmount(info.balance))")
        parts.append("Nonce: \(info.nonce)")
        return parts.joined(separator: "\n")
    }
}

// MARK: - Transfer Input Builder

/// Helps build transfer inputs and outputs from form data.
public enum TransferInputBuilder {

    /// Creates an AddressTransferInput from string parameters.
    /// - Parameters:
    ///   - addressHex: Address in hex format (42 characters).
    ///   - amount: Amount as string.
    ///   - nonce: Nonce (defaults to 0).
    ///   - privateKeyHex: Private key in hex format (64 characters).
    /// - Returns: AddressTransferInput if all parameters are valid, nil otherwise.
    public static func createInput(
        addressHex: String,
        amount: String,
        nonce: UInt32 = 0,
        privateKeyHex: String
    ) -> Addresses.AddressTransferInput? {
        guard let addressData = AddressTransformer.hexToData(addressHex),
              let amountValue = NumberTransformer.parseUInt64(amount),
              let privateKeyData = AddressTransformer.hexToData(privateKeyHex)
        else {
            return nil
        }

        return Addresses.AddressTransferInput(
            addressBytes: addressData,
            amount: amountValue,
            nonce: nonce,
            privateKey: privateKeyData
        )
    }

    /// Creates an AddressTransferOutput from string parameters.
    /// - Parameters:
    ///   - addressHex: Address in hex format (42 characters).
    ///   - amount: Amount as string.
    /// - Returns: AddressTransferOutput if all parameters are valid, nil otherwise.
    public static func createOutput(
        addressHex: String,
        amount: String
    ) -> Addresses.AddressTransferOutput? {
        guard let addressData = AddressTransformer.hexToData(addressHex),
              let amountValue = NumberTransformer.parseUInt64(amount)
        else {
            return nil
        }

        return Addresses.AddressTransferOutput(
            addressBytes: addressData,
            amount: amountValue
        )
    }

    /// Creates an AddressTransferOutput from string parameters, supporting both hex and bech32m addresses.
    /// - Parameters:
    ///   - address: Address in hex (42 characters) or bech32m format.
    ///   - amount: Amount as string.
    /// - Returns: AddressTransferOutput if all parameters are valid, nil otherwise.
    public static func createOutputUniversal(
        address: String,
        amount: String
    ) -> Addresses.AddressTransferOutput? {
        guard let addressData = AddressTransformer.parseAddress(address),
              let amountValue = NumberTransformer.parseUInt64(amount)
        else {
            return nil
        }

        return Addresses.AddressTransferOutput(
            addressBytes: addressData,
            amount: amountValue
        )
    }
}
