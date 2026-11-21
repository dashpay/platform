import Foundation

/// Encodes one index value of a contested-resource vote poll as the string
/// the contested-resource FFI reads.
///
/// The FFI calls behind
/// ``SDK/getContestedResources(documentTypeName:dataContractId:indexName:startIndexValues:endIndexValues:limit:orderAscending:)``,
/// ``SDK/getContestedResourceVoteState(dataContractId:documentTypeName:indexName:indexValues:resultType:allowIncludeLockedAndAbstainingVoteTally:count:)``,
/// ``SDK/getContestedResourceVotersForIdentity(dataContractId:documentTypeName:indexName:indexValues:contestantId:count:orderAscending:)``
/// and
/// ``SDK/castContestedResourceVote(dataContractId:documentTypeName:indexName:indexValues:choice:proTxHash:votingPrivateKey:)``
/// take index values as strings, read in this order:
///
/// 1. A string starting with a backslash (`\`) is text: the one leading
///    backslash is removed and the rest is taken verbatim.
/// 2. Otherwise a string starting with `0x` is hex bytes.
/// 3. Otherwise the string is text, taken verbatim.
///
/// Encode text with ``text(_:)`` rather than passing it raw: DPNS
/// homograph normalization maps `o` to `0`, so the label `oxford` is
/// indexed as `0xf0rd`, which a raw string would send as hex.
///
/// The `id` and `index_values` the contested resources query returns are
/// already encoded this way. They may be passed back unchanged only when
/// they are text or bytes and the path covers every property of the index
/// (for DPNS, `["dash", label]`); an integer value comes back as display
/// text and does not address the poll.
public enum ContestedIndexValue {
    /// Encode a text value: one backslash is prepended when `value` starts
    /// with `0x` or with a backslash, otherwise `value` is returned
    /// unchanged. `"0xf0rd"` becomes `"\0xf0rd"`, `"a11ce"` stays `"a11ce"`.
    ///
    /// The prefixes are compared as UTF-8 bytes, as Rust reads them, not as
    /// grapheme clusters: `"0x\u{301}"` starts with `0x` on the wire even
    /// though `String.hasPrefix("0x")` is false for it.
    public static func text(_ value: String) -> String {
        if value.utf8.starts(with: "0x".utf8) || value.utf8.starts(with: "\\".utf8) {
            return "\\" + value
        }
        return value
    }

    /// Encode a byte value: `"0x"` followed by lowercase hex.
    public static func bytes(_ data: Data) -> String {
        "0x" + data.toHexString()
    }
}
