import Foundation
import DashSDKFFI

// MARK: - DPNS contested-username browsing
//
// Typed reads for the *voter's* view of DPNS username contests: every open
// contest on the network, and the full vote state of one contest.
//
// These are deliberately separate from `ManagedPlatformWallet
// .fetchContestVoteState(identityId:label:)`, which is identity-scoped — it
// answers "is MY pending name being contested" and returns nothing for a
// label this identity is not contending in. A masternode operator browsing
// contests to vote on needs the network-wide view below.
//
// They are also separate from the untyped `dpnsGetContestedNonResolvedUsernames`
// / `dpnsGetContestedVoteState` dictionary wrappers in
// `FFI/PlatformQueryExtensions.swift`: those stringify each contender's tally
// into `"ResourceVote { … strength: N }"`, forcing callers to parse a number
// back out of a debug description. The FFI already provides `vote_count` as a
// `uint32_t`, so the typed reads here take it straight from the C struct.

@MainActor
extension SDK {

  // MARK: - Label normalization

  /// Homograph-normalize a DPNS label the way Platform does before indexing
  /// it (`o` → `0`, `i`/`l` → `1`, lowercased): `"Alice"` → `"a11ce"`.
  ///
  /// Every read and write that addresses a vote poll by label must use the
  /// normalized form — Platform stores nothing under the raw form.
  ///
  /// Normalization is protocol behavior and is owned by Rust
  /// (`dash_sdk::platform::dpns_usernames::convert_to_homograph_safe_chars`,
  /// reached through `dash_sdk_dpns_normalize_username`). This throws rather
  /// than falling back to a Swift reimplementation: a second copy of the
  /// mapping would drift from the canonical one and silently address a poll
  /// that does not exist.
  public func dpnsNormalizeLabel(_ label: String) throws -> String {
    let result = label.withCString { dash_sdk_dpns_normalize_username($0) }

    if let error = result.error {
      let sdkError = SDKError.fromDashSDKError(error.pointee)
      dash_sdk_error_free(error)
      throw sdkError
    }
    guard let dataPtr = result.data else {
      throw SDKError.internalError("DPNS normalization returned no data")
    }

    let normalized = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
    dash_sdk_string_free(dataPtr.assumingMemoryBound(to: CChar.self))
    return normalized
  }

  // MARK: - Active contests

  /// Every DPNS username contest that is still open, with its contenders,
  /// tallies and end time.
  ///
  /// One round trip. Reads the FFI's `DashSDKContestedNamesList` structure
  /// directly, so contender tallies arrive as integers.
  ///
  /// - Parameter limit: Maximum contests to return. The FFI has no
  ///   start-after cursor on this query, so this is a hard ceiling, not a
  ///   page size — raise it rather than trying to page.
  /// - Returns: Contests sorted by normalized label. Empty when nothing is
  ///   contested.
  public func dpnsActiveContests(limit: UInt32 = 200) throws -> [DPNSContest] {
    guard let handle = handle else {
      throw SDKError.invalidState("SDK not initialized")
    }
    guard let listPtr = dash_sdk_dpns_get_contested_non_resolved_usernames(handle, limit) else {
      throw SDKError.internalError("Failed to fetch contested DPNS usernames")
    }
    return Self.consumeContestedNamesList(listPtr)
  }

  /// The open contests **this identity is contending in** — the "how are my
  /// own username requests doing" view, where ``dpnsActiveContests(limit:)`` is
  /// the network-wide one.
  ///
  /// - Parameters:
  ///   - identityId: Base58 identity id.
  ///   - limit: Maximum contests to return.
  public func dpnsContestsForIdentity(
    identityId: String,
    limit: UInt32 = 100
  ) throws -> [DPNSContest] {
    guard let handle = handle else {
      throw SDKError.invalidState("SDK not initialized")
    }
    guard let listPtr = identityId.withCString({ idPtr in
      dash_sdk_dpns_get_non_resolved_contests_for_identity(handle, idPtr, limit)
    }) else {
      throw SDKError.internalError("Failed to fetch contested DPNS usernames for identity")
    }
    return Self.consumeContestedNamesList(listPtr)
  }

  /// Copy a `DashSDKContestedNamesList` into Swift values and free it.
  ///
  /// Shared by both list queries — they return the same C shape, and a second
  /// hand-rolled reader would be one more place for the ownership rules and
  /// the null-label handling to drift.
  ///
  /// Takes ownership: the list is freed before returning, on every path.
  private static func consumeContestedNamesList(
    _ listPtr: UnsafeMutablePointer<DashSDKContestedNamesList>
  ) -> [DPNSContest] {
    defer { dash_sdk_contested_names_list_free(listPtr) }

    let list = listPtr.pointee
    guard list.count > 0, let namesPtr = list.names else { return [] }

    var contests: [DPNSContest] = []
    contests.reserveCapacity(Int(list.count))

    for index in 0..<Int(list.count) {
      let entry = namesPtr[index]
      guard let namePtr = entry.name else { continue }
      let normalizedLabel = String(cString: namePtr)

      let info = entry.contest_info
      var contenders: [DPNSContender] = []
      if info.contender_count > 0, let contendersPtr = info.contenders {
        contenders.reserveCapacity(Int(info.contender_count))
        for contenderIndex in 0..<Int(info.contender_count) {
          let contender = contendersPtr[contenderIndex]
          guard let idPtr = contender.identity_id else { continue }
          // `label` is null when the FFI could not decode that contender's
          // document — a missing display name, not a reason to drop the row.
          let displayLabel = contender.label.map { String(cString: $0) }
          contenders.append(
            DPNSContender(
              identityId: String(cString: idPtr),
              voteTally: contender.vote_count,
              displayLabel: displayLabel?.isEmpty == false ? displayLabel : nil))
        }
      }

      // Ordered and de-duplicated on the Rust side; copied verbatim.
      var requestedLabels: [String] = []
      if info.requested_label_count > 0, let labelsPtr = info.requested_labels {
        requestedLabels.reserveCapacity(Int(info.requested_label_count))
        for labelIndex in 0..<Int(info.requested_label_count) {
          guard let labelPtr = labelsPtr[labelIndex] else { continue }
          requestedLabels.append(String(cString: labelPtr))
        }
      }

      contests.append(
        DPNSContest(
          normalizedLabel: normalizedLabel,
          // `end_time` is milliseconds since epoch; the FFI reports 0 when it
          // has none. Keep that as `nil` rather than rendering 1970.
          endTime: info.end_time > 0
            ? Date(timeIntervalSince1970: Double(info.end_time) / 1000)
            : nil,
          hasWinner: info.has_winner,
          abstainVotes: info.abstain_votes,
          lockVotes: info.lock_votes,
          contenders: contenders,
          requestedLabels: requestedLabels))
    }

    return contests.sorted { $0.normalizedLabel < $1.normalizedLabel }
  }

  // MARK: - One contest's vote state

  /// Full vote state for a single contest: contenders with tallies, the
  /// abstain and lock tallies, and the resolution outcome.
  ///
  /// - Parameters:
  ///   - normalizedLabel: Homograph-normalized label — pass the value from
  ///     ``dpnsNormalizeLabel(_:)`` or a label returned by
  ///     ``dpnsActiveContests(limit:)`` (already normalized).
  ///   - limit: Maximum contenders to return.
  /// - Returns: `nil` when Platform has no contenders on record for this
  ///   label — either it was never contested or the poll has been pruned.
  ///   Distinguishing "no contest" from "empty contest" is not possible at
  ///   this layer, so the absence is surfaced as `nil` rather than as an
  ///   empty state that reads like a live contest with zero votes.
  public func dpnsContestVoteState(
    normalizedLabel: String,
    limit: UInt32 = 100
  ) throws -> DPNSContestVoteState? {
    guard let handle = handle else {
      throw SDKError.invalidState("SDK not initialized")
    }

    let indexValues = DPNSVotePoll.indexValues(normalizedLabel: normalizedLabel)
    let indexValuesData = try JSONSerialization.data(withJSONObject: indexValues)
    guard let indexValuesJson = String(data: indexValuesData, encoding: .utf8) else {
      throw SDKError.serializationError("Failed to encode index values")
    }

    // result_type 2 = DocumentsAndVoteTally: documents are what carry the
    // winner info, tallies are what the UI shows.
    let result = dash_sdk_contested_resource_get_vote_state(
      handle,
      DPNSVotePoll.contractId,
      DPNSVotePoll.documentTypeName,
      DPNSVotePoll.indexName,
      indexValuesJson,
      2,
      true,
      limit)

    if let error = result.error {
      let sdkError = SDKError.fromDashSDKError(error.pointee)
      dash_sdk_error_free(error)
      throw sdkError
    }
    // The FFI returns NoData (not an error) when there are no contenders.
    guard let dataPtr = result.data else { return nil }

    let jsonString = String(cString: dataPtr.assumingMemoryBound(to: CChar.self))
    dash_sdk_string_free(dataPtr)

    guard let data = jsonString.data(using: .utf8),
          let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
      throw SDKError.serializationError("Failed to parse contested resource vote state")
    }

    return Self.decodeVoteState(json: json, normalizedLabel: normalizedLabel)
  }

  /// Decode the `dash_sdk_contested_resource_get_vote_state` payload:
  /// `{"abstain_vote_tally":N,"lock_vote_tally":N,
  ///   "winner_info":"NoWinner"|"Locked"|{"type":"WonByIdentity","identity_id":"…"},
  ///   "block_info":{"height":…,"core_height":…,"timestamp":…},
  ///   "contenders":[{"identity_id":"…","vote_count":N,"document":"hex"|null,
  ///                  "label":"pizza"}]}`
  ///
  /// `label` is the requester's own spelling, decoded FFI-side from the
  /// contender document; it is omitted when that decode was not possible.
  ///
  /// `winner_info` and `block_info` are absent while voting is open; when
  /// either is present the poll has finished (see ``DPNSContestOutcome``).
  ///
  /// `internal` rather than `private` so the hermetic decoder tests can drive
  /// it without a live SDK handle.
  nonisolated static func decodeVoteState(
    json: [String: Any],
    normalizedLabel: String
  ) -> DPNSContestVoteState {
    let contenders: [DPNSContender] = (json["contenders"] as? [[String: Any]] ?? [])
      .compactMap { entry in
        guard let identityId = entry["identity_id"] as? String else { return nil }
        let tally = (entry["vote_count"] as? NSNumber)?.uint32Value ?? 0
        // `label` is absent when the FFI could not decode the document; that
        // is a missing display name, not a reason to drop the contender.
        let label = (entry["label"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        return DPNSContender(identityId: identityId, voteTally: tally, displayLabel: label)
      }

    let outcome: DPNSContestOutcome = {
      if let winner = json["winner_info"] as? String {
        switch winner {
        case "Locked": return .locked
        case "NoWinner": return .noWinner
        default: return .ongoing
        }
      }
      if let winner = json["winner_info"] as? [String: Any],
         winner["type"] as? String == "WonByIdentity",
         let identityId = winner["identity_id"] as? String {
        return .wonBy(identityId)
      }
      return .ongoing
    }()

    let resolvedAt: Date? = {
      guard outcome != .ongoing,
            let blockInfo = json["block_info"] as? [String: Any],
            let timestamp = (blockInfo["timestamp"] as? NSNumber)?.uint64Value,
            timestamp > 0 else { return nil }
      return Date(timeIntervalSince1970: Double(timestamp) / 1000)
    }()

    return DPNSContestVoteState(
      normalizedLabel: normalizedLabel,
      contenders: contenders,
      abstainVotes: (json["abstain_vote_tally"] as? NSNumber)?.uint32Value ?? 0,
      lockVotes: (json["lock_vote_tally"] as? NSNumber)?.uint32Value ?? 0,
      outcome: outcome,
      resolvedAt: resolvedAt)
  }

  // MARK: - Pre-flight

  /// Whether Platform has an open, unresolved vote poll for
  /// `normalizedLabel` — i.e. whether a vote cast now can be accepted.
  ///
  /// Broadcasting against a closed poll produces a long, opaque retry before
  /// failing, so the caster checks this first and reports a precise error
  /// instead. Implemented over ``dpnsContestVoteState(normalizedLabel:limit:)``
  /// (one round trip) because that query distinguishes all three states the
  /// caller cares about: no poll (`nil`), resolved, and open.
  public func dpnsContestIsOpen(normalizedLabel: String) throws -> Bool {
    guard let state = try dpnsContestVoteState(normalizedLabel: normalizedLabel, limit: 1)
    else { return false }
    return !state.isResolved
  }
}
