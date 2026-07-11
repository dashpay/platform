import XCTest
@testable import SwiftExampleApp

/// Tests for the contracts-tab Register flow's pasteboard classifier.
///
/// `PasteboardContractCandidate.classify` is the purest piece of the
/// register-from-pasteboard pipeline: input is one `String`, output is
/// a 3-state `PasteboardClassification`. The branching is busy enough
/// (empty / non-JSON / JSON-not-object / 3 contract-shape detections /
/// reason-bearing rejection paths) that it's worth pinning behaviorally
/// instead of clicking through the simulator on every change.
final class PasteboardContractCandidateTests: XCTestCase {

    // MARK: - Empty / unclassifiable inputs

    func test_emptyString_isEmpty() {
        let result = PasteboardContractCandidate.classify("")
        guard case .empty = result else {
            return XCTFail("Expected .empty, got \(result)")
        }
    }

    func test_whitespaceOnly_isEmpty() {
        let result = PasteboardContractCandidate.classify("   \n\t  \n")
        guard case .empty = result else {
            return XCTFail("Expected .empty for whitespace, got \(result)")
        }
    }

    func test_plainText_isInvalidWithReason() {
        let result = PasteboardContractCandidate.classify("just some random text")
        guard case .invalid(let reason) = result else {
            return XCTFail("Expected .invalid for plain text, got \(result)")
        }
        XCTAssertTrue(
            reason.contains("isn’t valid JSON") || reason.contains("isn't valid JSON"),
            "Reason should explain the JSON-parse failure, got: \(reason)"
        )
        XCTAssertTrue(
            reason.contains("just some random text"),
            "Reason should include a preview of the offending text"
        )
    }

    func test_topLevelArray_isInvalidWithReason() {
        // Arrays are valid JSON but a contract must be a JSON object.
        let result = PasteboardContractCandidate.classify(#"["a","b"]"#)
        guard case .invalid(let reason) = result else {
            return XCTFail("Expected .invalid for top-level array, got \(result)")
        }
        XCTAssertTrue(
            reason.contains("array"),
            "Reason should name the actual top-level type, got: \(reason)"
        )
    }

    func test_topLevelString_isInvalidWithReason() {
        let result = PasteboardContractCandidate.classify(#""just a string""#)
        guard case .invalid(let reason) = result else {
            return XCTFail("Expected .invalid for top-level string, got \(result)")
        }
        XCTAssertTrue(
            reason.contains("string"),
            "Reason should name the top-level type, got: \(reason)"
        )
    }

    // MARK: - Object that doesn't match any contract shape

    func test_objectWithoutContractKeys_isInvalidWithKeyHint() {
        // Looks JSON-objecty but doesn't match docs / tokens / docSchemas /
        // tokenSchemas. Reason should help the user spot it (lists keys).
        let raw = #"{"identity":{"id":"abc"},"random":42}"#
        let result = PasteboardContractCandidate.classify(raw)
        guard case .invalid(let reason) = result else {
            return XCTFail("Expected .invalid for non-contract object, got \(result)")
        }
        XCTAssertTrue(
            reason.contains("doesn’t look like a contract")
                || reason.contains("doesn't look like a contract"),
            "Reason should explain the shape mismatch, got: \(reason)"
        )
        XCTAssertTrue(
            reason.contains("identity") && reason.contains("random"),
            "Reason should list the keys we did see, got: \(reason)"
        )
    }

    // MARK: - Bare schema-shaped objects

    func test_bareDocumentSchemasShape_isValid() {
        // Every value has a `properties` key — the marker for a document
        // schema. The classifier should pick this up as Case 2.
        let raw = """
        {
          "note": {
            "type": "object",
            "properties": {
              "message": { "type": "string", "position": 0 }
            },
            "required": ["message"]
          }
        }
        """
        let result = PasteboardContractCandidate.classify(raw)
        guard case .valid(let candidate) = result else {
            return XCTFail("Expected .valid for bare document-schemas, got \(result)")
        }
        XCTAssertNotNil(
            candidate.formInputs["documentSchemas"],
            "documentSchemas should be wired into formInputs"
        )
        XCTAssertNil(
            candidate.formInputs["tokenSchemas"],
            "no token schemas in this payload"
        )
        XCTAssertTrue(
            candidate.summary.contains("doc"),
            "summary should mention doc types, got: \(candidate.summary)"
        )
    }

    func test_bareTokenSchemasShape_isValid() {
        // All keys parse as integers, all values are objects → Case 3.
        let raw = #"{"0":{"baseSupply":1000000}}"#
        let result = PasteboardContractCandidate.classify(raw)
        guard case .valid(let candidate) = result else {
            return XCTFail("Expected .valid for bare token-schemas, got \(result)")
        }
        XCTAssertNotNil(
            candidate.formInputs["tokenSchemas"],
            "tokenSchemas should be wired into formInputs"
        )
        XCTAssertNil(
            candidate.formInputs["documentSchemas"],
            "no document schemas in this payload"
        )
        XCTAssertTrue(
            candidate.summary.contains("token"),
            "summary should mention tokens, got: \(candidate.summary)"
        )
    }

    // MARK: - Full contract-dict shape

    func test_fullContractDict_tokenOnly_isValid() {
        // Mirrors the on-chain V1 shape the user pastes in: top-level
        // `tokens`, an empty `documentSchemas`, `config` block, and
        // metadata. The classifier should NOT trip over the empty
        // `documentSchemas` since `tokens` carries content.
        let raw = """
        {
          "id": "HtQNfXBZJu3WnvjvCFJKgbvfgWYJxWxaFWy23TKoFjg9",
          "ownerId": "BmKTJeLL3GfH8FxEx7SUbTog4eAKj8vJRDi97gYkxB9p",
          "version": 1,
          "documentSchemas": {},
          "tokens": {
            "0": {
              "baseSupply": 100,
              "conventions": {
                "decimals": 8
              }
            }
          },
          "config": {
            "canBeDeleted": false,
            "documentsMutableContractDefault": true
          },
          "keywords": ["utility"],
          "description": "A token to test out new features."
        }
        """
        let result = PasteboardContractCandidate.classify(raw)
        guard case .valid(let candidate) = result else {
            return XCTFail("Expected .valid for full token-only contract, got \(result)")
        }

        XCTAssertNotNil(
            candidate.formInputs["tokenSchemas"],
            "tokenSchemas should be present on a token-only contract"
        )
        XCTAssertNil(
            candidate.formInputs["documentSchemas"],
            "empty documentSchemas should be filtered out so the form's 'at least one schema' guard isn't tripped by an empty `{}` string"
        )
        XCTAssertEqual(
            candidate.formInputs["keywords"],
            "utility",
            "keywords should round-trip as the comma-joined string the form expects"
        )
        XCTAssertEqual(
            candidate.formInputs["description"],
            "A token to test out new features."
        )
        // `config` flags split into both formInputs (string-encoded
        // booleans) AND checkboxInputs (native Bool) so whichever path
        // the form reads picks up the value. Both should be wired.
        XCTAssertEqual(
            candidate.formInputs["documentsMutableContractDefault"],
            "true"
        )
        XCTAssertEqual(
            candidate.checkboxInputs["documentsMutableContractDefault"],
            true
        )
        XCTAssertEqual(candidate.formInputs["canBeDeleted"], "false")
        XCTAssertEqual(candidate.checkboxInputs["canBeDeleted"], false)
    }

    func test_fullContractDict_topLevelFlags_winOverConfig() {
        // Belt-and-suspenders: when both top-level AND nested `config`
        // carry the same flag, top-level wins. Pins the precedence
        // behaviour `fromFullContractDict` documents.
        let raw = """
        {
          "documentSchemas": {"x": {"properties": {}}},
          "canBeDeleted": true,
          "config": { "canBeDeleted": false }
        }
        """
        let result = PasteboardContractCandidate.classify(raw)
        guard case .valid(let candidate) = result else {
            return XCTFail("Expected .valid, got \(result)")
        }
        XCTAssertEqual(
            candidate.checkboxInputs["canBeDeleted"],
            true,
            "Top-level flag should override the nested config flag"
        )
    }

    func test_fullContractDict_bothSchemasEmpty_isInvalid() {
        // The contract-shape keys exist but everything inside is empty.
        // We treat this as "not registerable" so the user gets a clear
        // error instead of a downstream validator failure.
        let raw = #"{"documentSchemas":{},"tokens":{}}"#
        let result = PasteboardContractCandidate.classify(raw)
        guard case .invalid = result else {
            return XCTFail("Empty schemas should be rejected, got \(result)")
        }
    }

    // MARK: - Summary string

    func test_summary_countsArePluralizedCorrectly() {
        // One token / two doc types: the summary should use the
        // singular vs plural noun forms correctly. Cosmetic but the
        // string is rendered prominently on the source-picker row.
        let raw = """
        {
          "documentSchemas": {
            "note": {"properties": {}},
            "memo": {"properties": {}}
          },
          "tokens": {
            "0": {"baseSupply": 1}
          }
        }
        """
        let result = PasteboardContractCandidate.classify(raw)
        guard case .valid(let candidate) = result else {
            return XCTFail("Expected .valid, got \(result)")
        }
        XCTAssertTrue(
            candidate.summary.contains("2 doc types"),
            "Plural doc types, got: \(candidate.summary)"
        )
        XCTAssertTrue(
            candidate.summary.contains("1 token"),
            "Singular token, got: \(candidate.summary)"
        )
        XCTAssertFalse(
            candidate.summary.contains("1 tokens"),
            "Should not pluralize a single token"
        )
    }
}
