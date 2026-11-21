import Foundation

/// Encodes one index value of a contested-resource vote poll as the string
/// the contested-resource FFI reads.
///
/// The FFI calls behind
/// ``SDK/getContestedResources(documentTypeName:dataContractId:indexName:resultType:allowIncludeLockedAndAbstainingVoteTally:startIndexValues:endIndexValues:limit:offset:orderAscending:)``,
/// ``SDK/getContestedResourceVoteState(dataContractId:documentTypeName:indexName:indexValues:resultType:allowIncludeLockedAndAbstainingVoteTally:startAtIdentifierInfo:count:orderAscending:)``,
/// ``SDK/getContestedResourceVotersForIdentity(dataContractId:documentTypeName:indexName:indexValues:contestantId:startAtIdentifierInfo:count:orderAscending:)``
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
/// already encoded this way and may be passed back unchanged.
public enum ContestedIndexValue {
    /// Encode a text value: one backslash is prepended when `value` starts
    /// with `0x` or with a backslash, otherwise `value` is returned
    /// unchanged. `"0xf0rd"` becomes `"\0xf0rd"`, `"a11ce"` stays `"a11ce"`.
    public static func text(_ value: String) -> String {
        if value.hasPrefix("0x") || value.hasPrefix("\\") {
            return "\\" + value
        }
        return value
    }

    /// Encode a byte value: `"0x"` followed by lowercase hex.
    public static func bytes(_ data: Data) -> String {
        "0x" + data.toHexString()
    }
}
