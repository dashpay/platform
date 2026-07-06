import Foundation

/// Minimal JSON-RPC client for dashd, covering only the calls the
/// integration framework needs.
struct CoreRPCClient {
    private(set) var url: URL
    let username: String
    let password: String

    init(port: Int, username: String, password: String) {
        var components = URLComponents()
        components.scheme = "http"
        components.host = "127.0.0.1"
        components.port = port
        guard let url = components.url else {
            preconditionFailure("Invalid Core RPC port: \(port)")
        }
        self.url = url
        self.username = username
        self.password = password
    }

    // MARK: - Typed methods

    /// Throws RPC error -32601 if dashd was built without the miner.
    @discardableResult
    func generateToAddress(count: Int, address: String) async throws -> [String] {
        try await call("generatetoaddress", params: [.int(count), .string(address)])
    }

    func getNewAddress(walletName: String = "main") async throws -> String {
        try await wallet(walletName).call("getnewaddress")
    }

    /// `amount` is in DASH (not duffs). Returns the txid.
    @discardableResult
    func sendToAddress(amount: Double, address: String, walletName: String = "main") async throws -> String {
        try await wallet(walletName).call(
            "sendtoaddress",
            params: [.string(address), .double(amount)]
        )
    }

    /// Current best-known chain tip height.
    func getBlockCount() async throws -> Int {
        try await call("getblockcount")
    }

    func getRawTransaction(_ txid: String) async throws -> String {
        try await call("getrawtransaction", params: [.string(txid)])
    }

    struct RawTxInfo: Decodable {
        let instantlock: Bool
    }

    func getRawTransactionInfo(_ txid: String) async throws -> RawTxInfo {
        try await call("getrawtransaction", params: [.string(txid), .int(1)])
    }

    @discardableResult
    func sendRawTransaction(_ hex: String) async throws -> String {
        try await call("sendrawtransaction", params: [.string(hex)])
    }

    private struct MasternodeEntry: Decodable {
        let address: String
    }

    /// Masternode P2P endpoints as `127.0.0.1:<port>`
    func masternodeP2PEndpoints() async throws -> [String] {
        let list: [String: MasternodeEntry] = try await call(
            "masternodelist", params: [.string("json")]
        )

        return list.values
            .compactMap { $0.address.split(separator: ":").last.map { "127.0.0.1:\($0)" } }
            .sorted()
    }

    /// Switches to the per-wallet RPC endpoint (`/wallet/<name>`).
    func wallet(_ name: String) -> CoreRPCClient {
        var copy = self
        copy.url = url.appendingPathComponent("wallet").appendingPathComponent(name)
        return copy
    }

    // MARK: - Generic JSON-RPC

    enum Param: Sendable {
        case string(String)
        case int(Int)
        case double(Double)
    }

    enum RPCError: Error, CustomStringConvertible {
        case http(Int)
        case rpc(code: Int, message: String)
        case decode(String)

        var description: String {
            switch self {
            case let .http(code): return "HTTP \(code)"
            case let .rpc(code, msg): return "RPC error \(code): \(msg)"
            case let .decode(msg): return "Decode error: \(msg)"
            }
        }
    }

    func call<T: Decodable>(_ method: String, params: [Param] = []) async throws -> T {
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")

        let credentials = "\(username):\(password)".data(using: .utf8)!.base64EncodedString()
        request.setValue("Basic \(credentials)", forHTTPHeaderField: "Authorization")

        let body: [String: Any] = [
            "jsonrpc": "1.0",
            "id": "swift-integration",
            "method": method,
            "params": encodeParams(params),
        ]
        request.httpBody = try JSONSerialization.data(withJSONObject: body)

        let (data, response) = try await URLSession.shared.data(for: request)
        // dashd returns the RPC error in the body even on 500; only
        // bail on HTTP errors if we can't parse the JSON-RPC body.
        if let http = response as? HTTPURLResponse, !(200...299).contains(http.statusCode) {
            if let decoded = try? decodeRPCResponse(data, T.self) {
                return try unwrap(decoded)
            }
            throw RPCError.http(http.statusCode)
        }
        let decoded = try decodeRPCResponse(data, T.self)
        return try unwrap(decoded)
    }

    private func decodeRPCResponse<T: Decodable>(_ data: Data, _: T.Type) throws -> RPCResponse<T> {
        do {
            return try JSONDecoder().decode(RPCResponse<T>.self, from: data)
        } catch {
            let raw = String(data: data, encoding: .utf8) ?? "<non-utf8>"
            throw RPCError.decode("\(error) — body: \(raw)")
        }
    }

    private func unwrap<T: Decodable>(_ response: RPCResponse<T>) throws -> T {
        if let error = response.error {
            throw RPCError.rpc(code: error.code, message: error.message)
        }
        guard let result = response.result else {
            throw RPCError.decode("RPC response had neither `result` nor `error`")
        }
        return result
    }

    private func encodeParams(_ params: [Param]) -> [Any] {
        params.map { param -> Any in
            switch param {
            case let .string(s): return s
            case let .int(i): return i
            case let .double(d): return d
            }
        }
    }

    private struct RPCResponse<T: Decodable>: Decodable {
        let result: T?
        let error: RPCErrorBody?
    }

    private struct RPCErrorBody: Decodable {
        let code: Int
        let message: String
    }
}
