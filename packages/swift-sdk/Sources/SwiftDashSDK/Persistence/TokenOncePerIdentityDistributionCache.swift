import Foundation

/// Memo for the once-per-identity distribution blocks a contract's JSON
/// declares, keyed by token position (protocol version 14).
///
/// `PersistentToken.oncePerIdentityDistribution` has no column to read: the
/// value is derived from the owning contract's `serializedContract`, and
/// `PersistentDataContract.parsedContract` runs `JSONSerialization` over the
/// whole contract on every access. The property is read per token row per
/// paint: the token list badge, the in-memory "has distribution" filter, the
/// claim form and the claim permission resolver all reach for it, and the
/// common token declares no distribution at all, so every one of those reads
/// would otherwise decode a full contract to learn nothing. This decodes once
/// per distinct contract payload instead and answers the rest from memory.
///
/// The contract JSON already persists the answer. A process-wide memo
/// avoids adding a redundant column to the model graph or changing any
/// released schema. Stored values also let frozen model copies use this cache.
///
/// Thread-safe: SwiftData rows are read from whichever actor owns their
/// context, so the map is guarded by a lock rather than pinned to the main
/// actor.
final class TokenOncePerIdentityDistributionCache: @unchecked Sendable {
    static let shared = TokenOncePerIdentityDistributionCache()

    /// Cached contract payloads to retain. Entries are small (one optional
    /// amount per token position) but the JSON they came from is not, and a
    /// device can hold far more contracts than any one screen looks at, so
    /// the oldest insertion is dropped past this many.
    private static let capacity = 32

    /// Identifies one contract payload. `id` alone is not enough: a contract
    /// row is re-created on every download and SwiftData merges the new
    /// values into the existing row under the unique-`id` constraint, so the
    /// same id can carry different JSON over time. Nothing assigns
    /// `serializedContract` outside `init` today, but relying on that
    /// silently would make a future assignment serve stale amounts, so the
    /// payload's byte count and the row's `lastUpdated` stamp are part of the
    /// key.
    private struct Key: Hashable {
        let contractId: Data
        let byteCount: Int
        let lastUpdated: Date
    }

    private let lock = NSLock()
    /// Token position -> parsed block, holding only the positions that
    /// declare one. A position missing from the map declares none, which is
    /// what makes a cache hit able to answer nil without re-decoding.
    private var entries: [Key: [Int: TokenOncePerIdentityDistribution]] = [:]
    /// Insertion order of `entries`' keys, oldest first, for eviction.
    private var insertionOrder: [Key] = []
    private var decodes: Int = 0

    /// How many times contract JSON has actually been decoded. Exposed for
    /// tests, which assert that repeated reads of the same contract do not
    /// move it; nothing in the SDK's public surface depends on it.
    var decodeCount: Int {
        lock.withLock { decodes }
    }

    /// Drop every cached payload. Tests use it to isolate cases; production
    /// code never needs it, because a changed payload changes the key.
    func removeAll() {
        lock.withLock {
            entries.removeAll()
            insertionOrder.removeAll()
            decodes = 0
        }
    }

    /// The once-per-identity distribution declared by the token at
    /// `position` of a contract payload, or nil when it declares none (or when the
    /// contract's JSON cannot be read at all).
    func distribution(
        contractId: Data,
        serializedContract: Data,
        lastUpdated: Date,
        position: Int
    ) -> TokenOncePerIdentityDistribution? {
        let key = Key(
            contractId: contractId,
            byteCount: serializedContract.count,
            lastUpdated: lastUpdated
        )

        return lock.withLock {
            if let cached = entries[key] {
                return cached[position]
            }

            // The decode runs under the lock so two callers racing on a cold
            // contract decode it once between them rather than twice each.
            let parsed = Self.parseAllPositions(serializedContract)
            decodes += 1
            entries[key] = parsed
            insertionOrder.append(key)
            if insertionOrder.count > Self.capacity {
                let evicted = insertionOrder.removeFirst()
                entries.removeValue(forKey: evicted)
            }
            return parsed[position]
        }
    }

    /// Decode the contract payload once and collect every token position
    /// that declares a once-per-identity distribution. Positions whose block
    /// is absent or malformed are left out, so they read back as nil.
    private static func parseAllPositions(
        _ serialized: Data
    ) -> [Int: TokenOncePerIdentityDistribution] {
        guard let root = try? JSONSerialization.jsonObject(with: serialized, options: []),
              let contract = root as? [String: Any],
              let tokens = contract["tokens"] as? [String: Any] else {
            return [:]
        }

        var parsed: [Int: TokenOncePerIdentityDistribution] = [:]
        for (positionKey, tokenValue) in tokens {
            // Skips `$formatVersion` and any other non-numeric key rs-dpp
            // puts alongside the positions.
            guard let position = Int(positionKey),
                  let tokenDict = tokenValue as? [String: Any],
                  let distributionRules = tokenDict["distributionRules"] as? [String: Any],
                  let distribution = DataContractParser.parseOncePerIdentityDistribution(
                      distributionRules["oncePerIdentityDistribution"]
                  ) else {
                continue
            }
            parsed[position] = distribution
        }
        return parsed
    }
}
