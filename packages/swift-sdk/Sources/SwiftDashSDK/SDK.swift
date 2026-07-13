import Foundation
import DashSDKFFI

// MARK: - Data Extensions
extension Data {
  /// Convert Data to Base58 string
  public func toBase58() -> String {
    let alphabet = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    var bytes = Array(self)
    var encoded = ""
    var zeroCount = 0

    // Count leading zeros
    for byte in bytes {
      if byte == 0 {
        zeroCount += 1
      } else {
        break
      }
    }

    // Remove leading zeros for processing
    bytes = Array(bytes.dropFirst(zeroCount))

    // Convert bytes to base58
    while !bytes.isEmpty {
      var remainder: UInt = 0
      var newBytes: [UInt8] = []

      for byte in bytes {
        let temp = UInt(byte) + remainder * 256
        remainder = temp % 58
        let quotient = temp / 58
        if !newBytes.isEmpty || quotient > 0 {
          newBytes.append(UInt8(quotient))
        }
      }

      bytes = newBytes
      encoded = String(alphabet[alphabet.index(alphabet.startIndex, offsetBy: Int(remainder))]) + encoded
    }

    // Add '1' for each leading zero byte
    encoded = String(repeating: "1", count: zeroCount) + encoded

    return encoded
  }
}

/// Swift wrapper for the Dash Platform SDK
public final class SDK: @unchecked Sendable {
  public private(set) var handle: OpaquePointer?

  /// The network this SDK instance is connected to
  public private(set) var network: Network = .testnet

  /// Identities operations
  public lazy var identities = Identities(sdk: self)

  /// Address operations (balance, nonce queries)
  public lazy var addresses = Addresses(sdk: self)

  /// Initialize the SDK library (call once at app startup)
  public static func initialize() {
    dash_sdk_init()
  }

  /// Log levels for SDK debugging
  public enum LogLevel: UInt8 {
    case error = 0
    case warn = 1
    case info = 2
    case debug = 3
    case trace = 4
  }

  /// Enable logging for gRPC and SDK operations
  /// This will log all network requests, including endpoints being contacted
  public static func enableLogging(level: LogLevel = .debug) {
    dash_sdk_enable_logging(level.rawValue)
    print("🔵 SDK: Logging enabled at level: \(level)")
  }

  /// Route the global tracing subscriber to per-bucket files under
  /// `sessionRoot`. Returns `false` if a subscriber was already
  /// installed or the path couldn't be written.
  @discardableResult
  public static func enableFileLogging(
    level: LogLevel = .debug,
    sessionRoot: String
  ) -> Bool {
    let installed = sessionRoot.withCString { ptr in
      platform_wallet_enable_file_logging(level.rawValue, ptr)
    }

    return installed
  }

  /// Local Platform DAPI addresses; override via UserDefaults key "platformDAPIAddresses"
  private static var platformDAPIAddresses: String {
    if let override = UserDefaults.standard.string(forKey: "platformDAPIAddresses"), !override.isEmpty {
      return override
    }
    return "http://127.0.0.1:2443"
  }

  /// Optional caller-provided base URL for the trusted-context-provider's
  /// quorum lookups. Read from UserDefaults key `platformQuorumURL`.
  /// Required to connect to devnets (no built-in default exists on the
  /// Rust side); also usable to override mainnet/testnet for staging
  /// shards. Returns nil when unset/empty.
  private static var platformQuorumURL: String? {
    guard
      let value = UserDefaults.standard.string(forKey: "platformQuorumURL"),
      !value.isEmpty
    else { return nil }
    return value
  }

  /// Synchronously fetch `{quorumBase}/masternodes` and return the
  /// raw `data` array. Both the DAPI list and the SPV peer list are
  /// derived from this — DAPI takes `<ip>:<platformHTTPPort>`, SPV
  /// takes the verbatim `address` field (`<ip>:<CoreP2PPort>`).
  ///
  /// Returns nil on any failure (timeout, JSON shape mismatch, etc.).
  /// Filters to `status == "ENABLED" && version_check == "success"`
  /// to match the Rust trusted-context provider's active-node policy
  /// (see `rs-sdk-trusted-context-provider/src/provider.rs`). Without
  /// the `version_check` filter, nodes the quorum service has
  /// already flagged as incompatible would be seeded into both the
  /// DAPI fan-out and the SPV peer list, undermining the
  /// self-healing rebuild this enables.
  ///
  /// `public` because both the SDK init (DAPI fan-out) and the
  /// SwiftExampleApp's SPV start path call it independently against
  /// the same endpoint — each caller pays its own round-trip, with
  /// no shared cache. An SDK rebuild on devnet therefore performs
  /// two `/masternodes` fetches; if that becomes a problem, the
  /// expectation is that callers add a short-lived cache locally
  /// (or refactor to share one through the SDK).
  public static func discoverActiveMasternodes(
    quorumBase: String
  ) -> [(spvPeer: String, dapiUrl: String)]? {
    guard
      var components = URLComponents(string: quorumBase),
      let scheme = components.scheme,
      !scheme.isEmpty
    else { return nil }
    if components.path.hasSuffix("/") {
      components.path = String(components.path.dropLast())
    }
    components.path += "/masternodes"
    guard let url = components.url else { return nil }

    var request = URLRequest(url: url)
    request.timeoutInterval = 5.0
    request.httpMethod = "GET"

    // Reference-typed box for the response so the completion
    // handler can safely store into it from URLSession's worker
    // thread without violating Swift 6 strict-concurrency capture
    // rules (which forbid mutating a captured `var Data?` from a
    // concurrently-executing closure). The semaphore guarantees
    // we only read `box.data` after the closure has run to
    // completion, so the cross-thread access is data-race-free.
    final class ResponseBox: @unchecked Sendable {
      var data: Data?
    }
    let box = ResponseBox()
    let semaphore = DispatchSemaphore(value: 0)
    let task = URLSession.shared.dataTask(with: request) { data, _, _ in
      box.data = data
      semaphore.signal()
    }
    task.resume()
    _ = semaphore.wait(timeout: .now() + .seconds(6))
    guard let data = box.data else {
      task.cancel()
      return nil
    }

    struct Envelope: Decodable {
      let success: Bool
      let data: [Masternode]
    }
    struct Masternode: Decodable {
      let address: String          // "ip:CoreP2PPort"
      let status: String
      // Optional to match the Rust trusted-context provider, which
      // tolerates entries missing `platformHTTPPort` and substitutes
      // a per-network default. Requiring this would make a single
      // misbehaving JSON entry fail the whole decode (Decodable is
      // all-or-nothing per object), nuking devnet auto-discovery.
      //
      // Note JSON wire keys are camelCase (`platformHTTPPort`,
      // `versionCheck`) — Rust renames its snake_case fields with
      // `#[serde(rename = ...)]` to produce that on the wire. Swift's
      // default `Decodable` synthesis matches property name → JSON
      // key literally, so no `CodingKeys` is needed here as long as
      // these property names match the wire keys verbatim.
      let platformHTTPPort: UInt16?
      // Same `versionCheck` field the Rust provider filters on.
      // Optional because older quorum-list-server builds may omit it;
      // callers below treat missing as "not success" (i.e. excluded).
      let versionCheck: String?
    }

    guard
      let env = try? JSONDecoder().decode(Envelope.self, from: data),
      env.success
    else { return nil }

    // Conservative default — matches the Rust trusted-context
    // provider's fallback when the entry omits `platform_http_port`.
    let defaultDapiPort: UInt16 = 443
    let active: [(String, String)] = env.data.compactMap { mn in
      guard mn.status == "ENABLED", mn.versionCheck == "success" else { return nil }
      let host = mn.address.split(separator: ":").first.map(String.init) ?? mn.address
      let dapiPort = mn.platformHTTPPort ?? defaultDapiPort
      return (mn.address, "https://\(host):\(dapiPort)")
    }
    return active.isEmpty ? nil : active
  }

  /// Synchronously fetch `{quorumBase}/masternodes` and build a
  /// comma-separated DAPI URL list (`https://<ip>:<platformHTTPPort>,…`).
  /// Returns nil on any error (network failure, JSON shape mismatch,
  /// timeout). Used by `init(network:)` to auto-populate the DAPI
  /// fan-out list on devnet when the user hasn't supplied one
  /// manually — saves the "you must paste 13 URLs" UX.
  ///
  /// Filters to `status == "ENABLED"` so down / banned nodes don't
  /// pollute the AddressList (the DAPI client would ban them on
  /// first request anyway, but skipping them up front speeds the
  /// first sync).
  private static func discoverDAPIAddresses(quorumBase: String) -> String? {
    guard let active = discoverActiveMasternodes(quorumBase: quorumBase) else {
      return nil
    }
    return active.map(\.dapiUrl).joined(separator: ",")
  }

  /// Create a new SDK instance with trusted setup
  ///
  /// This uses a trusted context provider that fetches quorum keys and
  /// data contracts from trusted HTTP endpoints instead of requiring proof verification.
  /// This is suitable for mobile applications where proof verification would be resource-intensive.
  ///
  /// `platformVersion`:
  /// - `0` (default) — let the Rust SDK seed at the per-network minimum
  ///   protocol version (mainnet 11, testnet/devnet/regtest 12) with
  ///   auto-detect on, ratcheting up as the network reports newer
  ///   versions. The per-network floor encodes the V0/V1 `getDocuments`
  ///   wire format (mainnet floor 11 = V0 until mainnet upgrades;
  ///   testnet floor 12 = V1), so this picks the right wire without a
  ///   Swift-side network→version map.
  /// - non-zero — pin the SDK to this exact `PlatformVersion`.
  public init(network: Network, platformVersion: UInt32 = 0) throws {
    var config = DashSDKConfig()
    config.network = network.ffiValue
    config.dapi_addresses = nil
    config.quorum_url = nil
    config.skip_asset_lock_proof_verification = false
    config.request_retry_count = 1
    config.request_timeout_ms = 8000 // 8 seconds
    // `0` is passed straight through: the FFI `apply_version(builder, 0)`
    // returns the builder unchanged, so `SdkBuilder::build()` seeds at the
    // per-network `min_protocol_version` floor with auto-detect on
    // (version_pinned=false) and ratchets up as the network reports newer
    // versions. A non-zero value is an explicit pin via `with_version`.
    config.platform_version = platformVersion

    // Create SDK with trusted setup. DAPI / quorum-URL overrides come from
    // UserDefaults and apply on:
    //
    //   * Regtest unconditionally — the Rust side has no built-in DAPI
    //     defaults for it, so we must supply addresses every time
    //     (otherwise SDK creation panics with `DAPI addresses not
    //     available for network: Regtest`, which would stall orphan-
    //     mnemonic recovery if it ran from a non-regtest active state).
    //   * Devnet unconditionally — same reason; additionally needs an
    //     explicit `quorum_url` because the default quorum endpoint
    //     `https://quorums.devnet.<name>.networks.dash.org` is template-
    //     interpolated from a devnet name we don't carry across FFI.
    //   * Mainnet/testnet only when the user opted in via
    //     `useDockerSetup` (existing dashmate-on-localhost flow). When
    //     that toggle is off, the Rust side picks the canonical seed
    //     addresses for the network.
    //
    // `quorum_url` is gated identically: applied for devnet/regtest and
    // under `useDockerSetup`, but NOT for plain mainnet/testnet. The
    // `platformQuorumURL` UserDefault is only ever populated by the
    // devnet-only Quorum URL field in Options, so forwarding it to a
    // mainnet/testnet build leaked a devnet (often http) endpoint into a
    // network whose Rust provider requires https — refusing to build the
    // SDK. With the gate off, mainnet/testnet use the canonical quorum
    // endpoints automatically.
    let result: DashSDKResult
    let useOverrideAddresses = network == .regtest
        || network == .devnet
        || UserDefaults.standard.bool(forKey: "useDockerSetup")
    let overrideQuorumURL: String? = useOverrideAddresses ? Self.platformQuorumURL : nil

    // Resolve the DAPI address list. Two paths:
    //
    //   * Devnet → ALWAYS auto-discover from `{quorumURL}/masternodes`
    //     fresh on every SDK build. The user input surface for devnet
    //     is just the quorum URL — DAPI nodes are an implementation
    //     detail of which masternodes happen to be ENABLED right now.
    //     Doing this every init is what makes the path self-healing
    //     when a node goes down on the chain. Cheap: one HTTP round-
    //     trip (~200ms) at network-switch cadence, which the user
    //     pays for explicitly anyway.
    //
    //   * Regtest / `useDockerSetup` → respect the existing
    //     `platformDAPIAddresses` UserDefaults override (default
    //     `http://127.0.0.1:2443`). This is the dashmate-local flow
    //     that's been stable; it has no /masternodes service to
    //     consult.
    //
    //   * Mainnet/testnet without overrides → Rust side picks seeds.
    let overrideAddresses: String?
    if network == .devnet {
      if let quorum = overrideQuorumURL,
         let discovered = Self.discoverDAPIAddresses(quorumBase: quorum) {
        overrideAddresses = discovered
      } else {
        // Quorum URL unset, or /masternodes unreachable / wrong shape.
        // Fall through with nil; Rust will refuse to build the SDK
        // and the resulting error surfaces in the iOS UI as
        // "Disconnected", prompting the user to fix the Quorum URL.
        overrideAddresses = nil
      }
    } else if useOverrideAddresses {
      overrideAddresses = Self.platformDAPIAddresses
    } else {
      overrideAddresses = nil
    }

    result = SDK.withOptionalCStrings(
      overrideAddresses,
      overrideQuorumURL
    ) { addressesCStr, quorumCStr in
      var mutableConfig = config
      if let addressesCStr { mutableConfig.dapi_addresses = addressesCStr }
      if let quorumCStr { mutableConfig.quorum_url = quorumCStr }
      return dash_sdk_create_trusted(&mutableConfig)
    }

    // Check for errors
    if result.error != nil {
      let error = result.error!.pointee
      let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
      defer {
        dash_sdk_error_free(result.error)
      }

      throw SDKError.internalError("Failed to create SDK: \(errorMessage)")
    }

    guard result.data != nil else {
      throw SDKError.internalError("No SDK handle returned")
    }

    // Store the handle and network
    handle = OpaquePointer(result.data)
    self.network = network
  }

  /// Run `body` with two optional C-string pointers. Each input string,
  /// when non-nil, is materialized into a NUL-terminated C buffer that is
  /// valid for the duration of the call; nil inputs pass through as nil
  /// pointers. Mirrors `String.withCString` for the two-optional-string
  /// case so the SDK init can hand both `dapi_addresses` and
  /// `quorum_url` into a single FFI call without nested withCString
  /// closures.
  private static func withOptionalCStrings<R>(
    _ a: String?,
    _ b: String?,
    _ body: (UnsafePointer<CChar>?, UnsafePointer<CChar>?) -> R
  ) -> R {
    switch (a, b) {
    case (nil, nil):
      return body(nil, nil)
    case (.some(let sa), nil):
      return sa.withCString { body($0, nil) }
    case (nil, .some(let sb)):
      return sb.withCString { body(nil, $0) }
    case (.some(let sa), .some(let sb)):
      return sa.withCString { aPtr in
        sb.withCString { bPtr in
          body(aPtr, bPtr)
        }
      }
    }
  }

  /// Load known contracts into the trusted context provider
  /// This avoids network calls for these contracts when they're needed
  public func loadKnownContracts(_ contracts: [(id: String, data: Data)]) throws {
    guard let handle = handle else {
      throw SDKError.invalidState("SDK not initialized")
    }

    guard !contracts.isEmpty else {
      return // Nothing to do
    }

    // Prepare contract IDs as comma-separated string
    let contractIds = contracts.map { $0.id }.joined(separator: ",")

    // Prepare arrays of contract data
    let contractDataPointers = contracts.map { contract in
      contract.data.withUnsafeBytes { bytes in
        bytes.baseAddress?.assumingMemoryBound(to: UInt8.self)
      }
    }

    let contractLengths = contracts.map { $0.data.count }

    // Call the FFI function
    let result = contractIds.withCString { idsCStr in
      contractDataPointers.withUnsafeBufferPointer { dataPointers in
        contractLengths.withUnsafeBufferPointer { lengths in
          dash_sdk_add_known_contracts(
            handle,
            idsCStr,
            dataPointers.baseAddress,
            lengths.baseAddress,
            UInt(contracts.count)
          )
        }
      }
    }

    // Check for errors
    if result.error != nil {
      let error = result.error!.pointee
      let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
      defer {
        dash_sdk_error_free(result.error)
      }

      throw SDKError.internalError("Failed to add known contracts: \(errorMessage)")
    }

    print("✅ Successfully loaded \(contracts.count) known contracts into SDK")
  }

  deinit {
    if let handle = handle {
      dash_sdk_destroy(handle)
    }
  }

  /// Get SDK status including mode and quorum count
  public func getStatus() throws -> SDKStatus {
    guard let handle = handle else {
      throw SDKError.invalidState("SDK not initialized")
    }

    let result = dash_sdk_get_status(handle)

    // Check for error
    if result.error != nil {
      let error = result.error!.pointee
      let errorMessage = error.message != nil ? String(cString: error.message!) : "Unknown error"
      defer {
        dash_sdk_error_free(result.error)
      }
      throw SDKError.internalError("Failed to get SDK status: \(errorMessage)")
    }

    // Parse the JSON result
    guard result.data != nil else {
      throw SDKError.internalError("No status data returned")
    }

    let jsonCStr = result.data.assumingMemoryBound(to: CChar.self)
    let jsonStr = String(cString: jsonCStr)
    defer {
      dash_sdk_string_free(jsonCStr)
    }

    guard let data = jsonStr.data(using: String.Encoding.utf8) else {
      throw SDKError.serializationError("Invalid JSON data")
    }

    do {
      let decoder = JSONDecoder()
      return try decoder.decode(SDKStatus.self, from: data)
    } catch {
      throw SDKError.serializationError("Failed to decode status: \(error)")
    }
  }

  /// Refresh this SDK's protocol version from the connected network.
  ///
  /// Issues a proven `getEpochsInfo` query on the Rust side and ratchets the
  /// SDK's auto-detected protocol version up to the network's version through
  /// the proof + quorum-signature-verified path (no unverified fallback). The
  /// new version is shared across every clone of the underlying `Sdk`
  /// (including the clone held by a `PlatformWalletManager`), so fee-sensitive
  /// flows pick it up automatically.
  ///
  /// Call on app start and after every network switch. For an SDK pinned to a
  /// fixed protocol version (version updating disabled) this is a no-op: no
  /// network request is made and the pinned version is returned. Bridges
  /// `dash_sdk_refresh_protocol_version`.
  ///
  /// - Returns: the SDK's protocol version number after the (possible) ratchet.
  @discardableResult
  public func refreshProtocolVersion() throws -> UInt32 {
    guard let handle = handle else {
      throw SDKError.invalidState("SDK not initialized")
    }

    let result = dash_sdk_refresh_protocol_version(handle)

    if result.error != nil {
      let error = result.error!.pointee
      defer {
        dash_sdk_error_free(result.error)
      }
      throw SDKError.fromDashSDKError(error)
    }

    guard result.data != nil else {
      throw SDKError.internalError("No protocol version returned")
    }

    let cStr = result.data.assumingMemoryBound(to: CChar.self)
    let versionStr = String(cString: cStr)
    defer {
      dash_sdk_string_free(cStr)
    }

    guard let version = UInt32(versionStr) else {
      throw SDKError.serializationError("Invalid protocol version: \(versionStr)")
    }

    return version
  }

  // TODO: Re-enable when CDashSDKFFI module is working
  // /// Test the new FFI connection
  // public func testNewFFI() -> Bool {
  //     guard let newHandle = newFFIHandle else {
  //         print("No new FFI handle available")
  //         return false
  //     }
  //
  //     // Try to get the network from the new FFI
  //     let sdkHandle = UnsafePointer<dash_sdk_SDKHandle>(OpaquePointer(newHandle))
  //     let network = dash_sdk_get_network(sdkHandle)
  //
  //     print("New FFI network: \(network)")
  //     return true
  // }

}

/// SDK Status information
public struct SDKStatus: Codable {
  public let version: String
  public let network: String
  public let mode: String
  public let quorumCount: Int
}

/// SDK Error handling
public enum SDKError: Error {
  case invalidParameter(String)
  case invalidState(String)
  case networkError(String)
  case serializationError(String)
  case protocolError(String)
  case cryptoError(String)
  case notFound(String)
  case timeout(String)
  case notImplemented(String)
  case internalError(String)
  case unknown(String)

  public static func fromDashSDKError(_ error: DashSDKError) -> SDKError {
    let message = error.message != nil ? String(cString: error.message!) : "Unknown error"

    switch error.code {
    case DashSDKErrorCode(rawValue: 1): // Invalid parameter
      return .invalidParameter(message)
    case DashSDKErrorCode(rawValue: 2): // Invalid state
      return .invalidState(message)
    case DashSDKErrorCode(rawValue: 3): // Network error
      return .networkError(message)
    case DashSDKErrorCode(rawValue: 4): // Serialization error
      return .serializationError(message)
    case DashSDKErrorCode(rawValue: 5): // Protocol error
      return .protocolError(message)
    case DashSDKErrorCode(rawValue: 6): // Crypto error
      return .cryptoError(message)
    case DashSDKErrorCode(rawValue: 7): // Not found
      return .notFound(message)
    case DashSDKErrorCode(rawValue: 8): // Timeout
      return .timeout(message)
    case DashSDKErrorCode(rawValue: 9): // Not implemented
      return .notImplemented(message)
    case DashSDKErrorCode(rawValue: 99): // Internal error
      return .internalError(message)
    default:
      return .unknown(message)
    }
  }
}

extension SDKError: LocalizedError {
  public var errorDescription: String? {
    switch self {
    case .invalidParameter(let message):
      return "Invalid Parameter: \(message)"
    case .invalidState(let message):
      return "Invalid State: \(message)"
    case .networkError(let message):
      return "Network Error: \(message)"
    case .serializationError(let message):
      return "Serialization Error: \(message)"
    case .protocolError(let message):
      return "Protocol Error: \(message)"
    case .cryptoError(let message):
      return "Cryptographic Error: \(message)"
    case .notFound(let message):
      return "Not Found: \(message)"
    case .timeout(let message):
      return "Operation Timed Out: \(message)"
    case .notImplemented(let message):
      return "Feature Not Implemented: \(message)"
    case .internalError(let message):
      return "Internal Error: \(message)"
    case .unknown(let message):
      return "Unknown Error: \(message)"
    }
  }
}


/// Identities operations
public class Identities {
  private weak var sdk: SDK?

  init(sdk: SDK) {
    self.sdk = sdk
  }

  /// Get a single identity balance
  public func getBalance(id: Data) throws -> UInt64 {
    guard let sdk = sdk, let handle = sdk.handle else {
      throw SDKError.invalidState("SDK not initialized")
    }

    guard id.count == 32 else {
      throw SDKError.invalidParameter("Identity ID must be exactly 32 bytes")
    }

    // Convert Data to Base58 string (the FFI expects string IDs)
    let idString = id.toBase58()

    let result = idString.withCString { cString in
      // Handle is OpaquePointer which Swift should convert automatically
      return dash_sdk_identity_fetch_balance(handle, cString)
    }

    // Check for errors
    if result.error != nil {
      let error = result.error!.pointee
      defer {
        dash_sdk_error_free(result.error)
      }
      throw SDKError.fromDashSDKError(error)
    }

    guard result.data != nil else {
      throw SDKError.internalError("No balance data returned")
    }

    // Parse the balance from result
    let balancePtr = result.data.assumingMemoryBound(to: UInt64.self)
    let balance = balancePtr.pointee

    // Free the result data
    dash_sdk_bytes_free(result.data)

    return balance
  }

  /// Fetch balances for multiple identities using Data (32-byte arrays)
  /// - Parameter ids: Array of identity IDs as Data objects (must be exactly 32 bytes each)
  /// - Returns: Dictionary mapping identity IDs (as Data) to their balances (nil if identity not found)
  public func fetchBalances(ids: [Data]) throws -> [Data: UInt64?] {
    guard let sdk = sdk, let handle = sdk.handle else {
      throw SDKError.invalidState("SDK not initialized")
    }

    guard !ids.isEmpty else {
      return [:]
    }

    // Validate all IDs are 32 bytes
    for id in ids {
      guard id.count == 32 else {
        throw SDKError.invalidParameter("Identity ID must be exactly 32 bytes, got \(id.count)")
      }
    }

    // Convert Data to byte arrays
    let idByteArrays: [[UInt8]] = ids.map { Array($0) }

    // Create array of 32-byte arrays for FFI
    let idArrays: [(UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8,
                    UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8, UInt8)] =
    idByteArrays.map { bytes in
      (bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
       bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
       bytes[16], bytes[17], bytes[18], bytes[19], bytes[20], bytes[21], bytes[22], bytes[23],
       bytes[24], bytes[25], bytes[26], bytes[27], bytes[28], bytes[29], bytes[30], bytes[31])
    }

    let result = idArrays.withUnsafeBufferPointer { buffer -> DashSDKResult in
      let idsPtr = buffer.baseAddress
      // The handle is already the correct type for the C function
      return dash_sdk_identities_fetch_balances(handle, idsPtr, UInt(ids.count))
    }

    // Check for errors
    if result.error != nil {
      let error = result.error!.pointee
      defer {
        dash_sdk_error_free(result.error)
      }
      throw SDKError.fromDashSDKError(error)
    }

    guard result.data != nil else {
      throw SDKError.internalError("No data returned from fetch balances")
    }

    // Parse the identity balance map
    let mapPtr = result.data.assumingMemoryBound(to: DashSDKIdentityBalanceMap.self)
    let map = mapPtr.pointee

    var balances: [Data: UInt64?] = [:]

    if map.count > 0 && map.entries != nil {
      for i in 0..<map.count {
        let entry = map.entries[Int(i)]
        let idData = withUnsafeBytes(of: entry.identity_id) { Data($0) }

        // Check if balance is u64::MAX (which means not found)
        if entry.balance == UInt64.max {
          balances[idData] = nil
        } else {
          balances[idData] = entry.balance
        }
      }
    }

    // Free the result
    dash_sdk_identity_balance_map_free(mapPtr)

    // Make sure all requested IDs are in the result
    for id in ids {
      if balances[id] == nil {
        balances[id] = nil
      }
    }

    return balances
  }

  // Helper function to convert hex string to bytes
  private func hexToBytes(_ hex: String) -> [UInt8]? {
    let hex = hex.trimmingCharacters(in: .whitespacesAndNewlines)
    guard hex.count == 64 else { return nil } // 32 bytes = 64 hex chars

    var bytes = [UInt8]()
    var index = hex.startIndex

    while index < hex.endIndex {
      let nextIndex = hex.index(index, offsetBy: 2)
      let byteString = hex[index..<nextIndex]

      if let byte = UInt8(byteString, radix: 16) {
        bytes.append(byte)
      } else {
        return nil
      }

      index = nextIndex
    }

    return bytes.count == 32 ? bytes : nil
  }

  // Helper function to convert bytes to hex string
  private func bytesToHex(_ bytes: [UInt8]) -> String {
    return bytes.map { String(format: "%02x", $0) }.joined()
  }
}
