import CryptoKit
import Foundation

/// **Dev/QA tool only.** Client for the testnet DASH faucet at
/// `faucet.thepasta.org`, shared by SwiftExampleApp and dashwallet-ios
/// (promoted here from the example app so the apps don't carry drifting
/// copies). This is a plain `URLSession` HTTP client plus a pure-Swift
/// `cap.js` proof-of-work solver — it deliberately does **not** touch the
/// wallet / Platform FFI, the `platform-wallet` crate, or any SDK business
/// logic. It exists purely so QA can top up a fresh/drained testnet wallet
/// with one tap instead of round-tripping through the web faucet
/// (issue #3897).
///
/// The faucet enforces a server-side captcha (`cap.js`). The "soft"
/// challenge is a small proof-of-work (≈6.5M SHA-256 ops) that we solve
/// on-device off the main actor; the "hard" challenge returned on rate
/// limit is far too expensive to brute-force here, so on rate limit (or
/// any other failure) the caller falls back to opening the web faucet.

// MARK: - cap.js proof-of-work solver

enum CapSolver {
    /// cap.js seeded PRNG: FNV-1a-style seed + xorshift32. Emits `length`
    /// lowercase hex chars.
    ///
    /// Verified bit-identical to `@cap.js/widget` against the live server
    /// (cross-checked with `/tmp/cap_wrap_check.py` — both the wrapping and
    /// the masked int32 forms agree, and the wrapping form redeems live).
    static func prng(_ seed: String, _ length: Int) -> String {
        var state: UInt32 = 2166136261                 // 0x811C9DC5
        for u in seed.utf16 {                          // seeds are ASCII (hex token + digits [+ "d"])
            state ^= UInt32(u)
            let sh = (state << 1) &+ (state << 4) &+ (state << 7) &+ (state << 8) &+ (state << 24)
            state = state &+ sh
        }
        var out = ""
        while out.count < length {
            state ^= state << 13
            state ^= state >> 17
            state ^= state << 5
            out += String(format: "%08x", state)        // unsigned 32-bit, 8 hex chars
        }
        return String(out.prefix(length))
    }

    /// A single sub-challenge: brute-force the smallest non-negative
    /// integer `nonce` such that lowercase-hex(`SHA256(salt + String(nonce))`)
    /// starts with `target`.
    ///
    /// We never materialize the 64-char hex digest per attempt. `target`
    /// is parsed once into a byte prefix (plus an optional trailing high
    /// nibble when `target` has odd length) and compared directly against
    /// the raw `SHA256` digest bytes.
    ///
    /// Honors cooperative cancellation so a pathological / slow challenge
    /// (cancelled by the solve timeout) can't pin a core indefinitely;
    /// throws `CancellationError` when cancelled mid-search.
    static func solve(salt: String, target: String) throws -> Int {
        let targetChars = Array(target.utf8)
        let fullBytes = targetChars.count / 2
        let hasNibble = targetChars.count % 2 == 1

        // Parse the full leading bytes of the hex target.
        var prefix = [UInt8]()
        prefix.reserveCapacity(fullBytes)
        for i in 0..<fullBytes {
            let hi = hexNibble(targetChars[i * 2])
            let lo = hexNibble(targetChars[i * 2 + 1])
            prefix.append((hi << 4) | lo)
        }
        // Optional trailing high-nibble (odd-length target).
        let nibble: UInt8 = hasNibble ? hexNibble(targetChars[fullBytes * 2]) : 0

        let saltBytes = Array(salt.utf8)
        var nonce = 0
        while true {
            if nonce & 0xFFFF == 0 { try Task.checkCancellation() }
            var hasher = SHA256()
            hasher.update(data: saltBytes)
            hasher.update(data: Array(String(nonce).utf8))
            let digest = hasher.finalize()

            if digestHasPrefix(digest, prefix: prefix, nibble: nibble, hasNibble: hasNibble) {
                return nonce
            }
            nonce += 1
        }
    }

    private static func digestHasPrefix(
        _ digest: SHA256.Digest,
        prefix: [UInt8],
        nibble: UInt8,
        hasNibble: Bool
    ) -> Bool {
        var idx = 0
        for byte in digest {
            if idx < prefix.count {
                if byte != prefix[idx] { return false }
                idx += 1
            } else if hasNibble {
                // Compare only the high nibble of the next digest byte.
                return (byte >> 4) == nibble
            } else {
                return true
            }
            if idx == prefix.count && !hasNibble {
                return true
            }
        }
        // Digest exhausted: matched iff we consumed the whole prefix and
        // there's no dangling nibble to check.
        return idx == prefix.count && !hasNibble
    }

    private static func hexNibble(_ c: UInt8) -> UInt8 {
        switch c {
        case 0x30...0x39: return c - 0x30            // '0'-'9'
        case 0x61...0x66: return c - 0x61 + 10       // 'a'-'f'
        case 0x41...0x46: return c - 0x41 + 10       // 'A'-'F'
        default: return 0
        }
    }
}

// MARK: - Outcome

/// Result of a faucet funding attempt. The view turns `.failed` /
/// `.rateLimited` into the clipboard + web-faucet fallback.
public enum TestnetFaucetOutcome: Sendable {
    /// Funds were sent. `txid` is the Core transaction id; `amount` is the
    /// tDASH amount reported by `/api/status` (`coreFaucetAmount`).
    case sent(txid: String, amount: Double)
    /// Server rate-limited the request (429). Carries a user-facing message.
    case rateLimited(message: String)
    /// Any other failure (network error, solve failure, non-200 response).
    case failed(reason: String)
}

// MARK: - HTTP client

/// Thin async client for the testnet faucet. Stateless; create one per use.
public struct TestnetFaucet {
    /// Faucet web host. Used both for the JSON API and as the web fallback URL.
    public static let webURL = URL(string: "https://faucet.thepasta.org/")!

    private let host = URL(string: "https://faucet.thepasta.org")!
    private let session: URLSession

    public init(session: URLSession = .shared) {
        self.session = session
    }

    /// Request 1 tDASH to `address`. Solves the soft captcha on-device and
    /// posts to `/api/core-faucet`. Safe to call from the main actor — the
    /// proof-of-work runs on a detached background-priority task.
    public func requestCoreDash(address: String) async -> TestnetFaucetOutcome {
        let trimmed = address.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else {
            return .failed(reason: "No address available")
        }

        // 1. Read status for the (rotatable) cap endpoint + faucet amount.
        let status: FaucetStatus
        do {
            status = try await fetchStatus()
        } catch {
            return .failed(reason: "Faucet status unavailable: \(error.localizedDescription)")
        }

        // Defense-in-depth: only solve against an HTTPS endpoint on the
        // faucet's own registrable domain. A faucet response that returned an
        // http:// or attacker-controlled URL would otherwise downgrade the
        // exchange (leaking the redeemed capToken) or repoint the POSTs —
        // which also carry the receive-address-correlated request — off-host.
        guard let capBase = URL(string: status.capEndpoint),
              capBase.scheme == "https",
              let capHost = capBase.host,
              let faucetHost = host.host,
              Self.sameRegistrableDomain(capHost, faucetHost) else {
            return .failed(reason: "Untrusted captcha endpoint")
        }

        // 2. Solve the soft captcha proof-of-work.
        let capToken: String
        do {
            capToken = try await solveCaptcha(capBase: capBase)
        } catch {
            return .failed(reason: "Captcha solve failed: \(error.localizedDescription)")
        }

        // 3. Post the funding request.
        return await postFaucet(
            address: trimmed,
            capToken: capToken,
            amount: status.coreFaucetAmount
        )
    }

    // MARK: status

    /// Subset of `/api/status` we use: the (rotatable) captcha base endpoint
    /// and the per-request faucet amount in DASH.
    private struct FaucetStatus: Decodable {
        let capEndpoint: String
        let coreFaucetAmount: Double
    }

    /// Fetches faucet config from `/api/status`. Surfaces a non-200 as a
    /// clear error rather than letting it fall through to an opaque
    /// JSON-decode failure.
    private func fetchStatus() async throws -> FaucetStatus {
        let url = host.appendingPathComponent("api/status")
        var request = URLRequest(url: url)
        request.httpMethod = "GET"
        request.timeoutInterval = 30
        let (data, response) = try await session.data(for: request)
        if let http = response as? HTTPURLResponse, http.statusCode != 200 {
            throw FaucetError.message("status \(http.statusCode)")
        }
        return try JSONDecoder().decode(FaucetStatus.self, from: data)
    }

    // MARK: captcha

    /// cap.js challenge descriptor returned by `POST {capEndpoint}challenge`.
    private struct ChallengeResponse: Decodable {
        struct Challenge: Decodable {
            let c: Int   // number of sub-challenges
            let s: Int   // salt hex length
            let d: Int   // target hex length
        }
        let challenge: Challenge
        let token: String
    }

    /// Result of `POST {capEndpoint}redeem`; `token` is the capToken on success.
    private struct RedeemResponse: Decodable {
        let success: Bool
        let token: String?
    }

    /// Runs the full cap.js handshake: request a challenge, brute-force the
    /// `c` sub-challenges (parallelized across cores), redeem for a capToken.
    private func solveCaptcha(capBase: URL) async throws -> String {
        // challenge
        let challengeURL = capBase.appendingPathComponent("challenge")
        let challenge: ChallengeResponse = try await postJSON(
            url: challengeURL,
            body: [:] as [String: String]
        )

        let c = challenge.challenge.c
        let s = challenge.challenge.s
        let d = challenge.challenge.d
        let token = challenge.token

        // Reject pathological or unsupported challenge sizes up front. `c`
        // bounds the number of spawned tasks, `s` the PRNG salt length, and
        // `d` the PoW difficulty (expected work ≈ 16^d per sub-challenge). A
        // misbehaving / compromised / future faucet response outside these
        // bounds would otherwise pin the device's cores with no in-app abort;
        // throwing here routes the caller to the web-faucet fallback instead.
        guard (1...256).contains(c), (1...256).contains(s), (1...6).contains(d) else {
            throw FaucetError.message("Unsupported captcha challenge (c=\(c), s=\(s), d=\(d))")
        }

        // The per-field bounds alone still admit c=256, d=6 ≈ 4.3B expected
        // attempts — enough to pin the CPU until the 30 s timeout on every
        // tap. Cap the AGGREGATE expected work (c × 16^d). The live faucet's
        // soft challenge is c=100, d=4 ≈ 6.6M attempts; 64M gives ~10×
        // headroom for server-side tuning while keeping the worst case to a
        // few seconds of parallel hashing.
        let expectedWork = Double(c) * pow(16, Double(d))
        guard expectedWork <= 64_000_000 else {
            throw FaucetError.message(
                "Captcha challenge too expensive (c=\(c), d=\(d), "
                    + "expectedWork=\(Int(expectedWork)) > 64000000)")
        }

        // Solve all c sub-challenges off the main actor, parallelized, under
        // an overall timeout so even an in-bounds-but-slow challenge can't
        // hang the UI indefinitely (the timeout cancels the solve tasks).
        let solutions: [Int] = try await withSolveTimeout(seconds: 30) {
            try await withThrowingTaskGroup(of: (Int, Int).self) { group in
                for i in 1...c {
                    group.addTask(priority: .background) {
                        let salt = CapSolver.prng("\(token)\(i)", s)
                        let target = CapSolver.prng("\(token)\(i)d", d)
                        let nonce = try CapSolver.solve(salt: salt, target: target)
                        return (i, nonce)
                    }
                }
                var collected = Array(repeating: 0, count: c)
                for try await (i, nonce) in group {
                    collected[i - 1] = nonce   // preserve sub-challenge order
                }
                return collected
            }
        }

        // redeem
        let redeemURL = capBase.appendingPathComponent("redeem")
        let redeem: RedeemResponse = try await postJSON(
            url: redeemURL,
            body: RedeemRequest(token: token, solutions: solutions)
        )
        guard redeem.success, let capToken = redeem.token else {
            throw FaucetError.message("Captcha redeem rejected")
        }
        return capToken
    }

    /// Body for `POST {capEndpoint}redeem`: the challenge token and the
    /// per-sub-challenge solution nonces, in order.
    private struct RedeemRequest: Encodable {
        let token: String
        let solutions: [Int]
    }

    // MARK: faucet POST

    /// Posts the funding request to `/api/core-faucet` and maps the response
    /// to a `TestnetFaucetOutcome` (200 → `.sent`, 429 → `.rateLimited`,
    /// everything else → `.failed`).
    private func postFaucet(
        address: String,
        capToken: String,
        amount: Double
    ) async -> TestnetFaucetOutcome {
        let url = host.appendingPathComponent("api/core-faucet")
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.timeoutInterval = 30
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        let body = ["address": address, "capToken": capToken]
        guard let payload = try? JSONSerialization.data(withJSONObject: body) else {
            return .failed(reason: "Failed to encode request")
        }
        request.httpBody = payload

        do {
            let (data, response) = try await session.data(for: request)
            guard let http = response as? HTTPURLResponse else {
                return .failed(reason: "Invalid response")
            }

            switch http.statusCode {
            case 200:
                if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                   let txid = json["txid"] as? String {
                    return .sent(txid: txid, amount: amount)
                }
                return .failed(reason: "Unexpected success payload")
            case 429:
                return .rateLimited(
                    message: "Rate limited (3/hour per IP). Try the web faucet."
                )
            default:
                let detail = parseErrorDetail(from: data)
                return .failed(
                    reason: "Faucet error \(http.statusCode): \(detail)"
                )
            }
        } catch {
            return .failed(reason: "Network error: \(error.localizedDescription)")
        }
    }

    /// Pulls the human-readable error out of a `{"detail":{"error":"..."}}`
    /// (or a flat `{"detail":"..."}`) body, falling back to a truncated raw
    /// body.
    private func parseErrorDetail(from data: Data) -> String {
        if let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] {
            if let detail = json["detail"] as? [String: Any],
               let err = detail["error"] as? String {
                return err
            }
            if let detail = json["detail"] as? String {
                return detail
            }
        }
        return String(decoding: data.prefix(80), as: UTF8.self)
    }

    // MARK: JSON helpers

    /// Internal error carrying a human-readable message (surfaced in the
    /// fallback toast via `localizedDescription`).
    private enum FaucetError: Error, LocalizedError {
        case message(String)
        var errorDescription: String? {
            switch self { case .message(let m): return m }
        }
    }

    /// POSTs `body` as JSON and decodes the response, throwing on a non-200.
    private func postJSON<Body: Encodable, Out: Decodable>(
        url: URL,
        body: Body
    ) async throws -> Out {
        var request = URLRequest(url: url)
        request.httpMethod = "POST"
        request.timeoutInterval = 30
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try JSONEncoder().encode(body)
        let (data, response) = try await session.data(for: request)
        if let http = response as? HTTPURLResponse, http.statusCode != 200 {
            throw FaucetError.message("HTTP \(http.statusCode)")
        }
        return try JSONDecoder().decode(Out.self, from: data)
    }

    // MARK: misc helpers

    /// Runs `op`, but throws `FaucetError.message` if it doesn't finish within
    /// `seconds`. Cancelling the losing task propagates to the cooperative
    /// PoW solve so it stops promptly.
    private func withSolveTimeout<T: Sendable>(
        seconds: Double,
        _ op: @escaping @Sendable () async throws -> T
    ) async throws -> T {
        try await withThrowingTaskGroup(of: T.self) { group in
            group.addTask { try await op() }
            group.addTask {
                try await Task.sleep(nanoseconds: UInt64(seconds * 1_000_000_000))
                throw FaucetError.message("Captcha solve timed out")
            }
            guard let result = try await group.next() else {
                throw FaucetError.message("Captcha solve produced no result")
            }
            group.cancelAll()
            return result
        }
    }

    /// True when two hosts share their registrable domain (last two DNS
    /// labels), e.g. `cap.thepasta.org` and `faucet.thepasta.org`.
    private static func sameRegistrableDomain(_ a: String, _ b: String) -> Bool {
        func base(_ h: String) -> String {
            h.split(separator: ".").suffix(2).joined(separator: ".")
        }
        let ba = base(a)
        return ba.contains(".") && ba == base(b)
    }
}
