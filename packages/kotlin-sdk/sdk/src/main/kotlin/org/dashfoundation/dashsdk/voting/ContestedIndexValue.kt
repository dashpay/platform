package org.dashfoundation.dashsdk.voting

import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonPrimitive
import org.dashfoundation.dashsdk.persistence.toHex

/**
 * Encodes one index value of a contested-resource vote poll as the string the
 * contested-resource FFI reads. Kotlin mirror of Swift's `ContestedIndexValue`.
 *
 * [VoteCasting.castVote] and the contested-resource queries in
 * [org.dashfoundation.dashsdk.queries.Voting] take index values as strings,
 * read in this order:
 *
 * 1. A string starting with a backslash (`\`) is text: the one leading
 *    backslash is removed and the rest is taken verbatim.
 * 2. Otherwise a string starting with `0x` is hex bytes.
 * 3. Otherwise the string is text, taken verbatim.
 *
 * Encode text with [text] rather than passing it raw: DPNS homograph
 * normalization maps `o` to `0`, so the label `oxford` is indexed as
 * `0xf0rd`, which a raw string would send as hex.
 *
 * The `id` and `index_values` the contested resources query returns are
 * already encoded this way. They may be passed back unchanged only when they
 * are text or bytes and the path covers every property of the index (for
 * DPNS, `["dash", label]`); an integer value comes back as display text and
 * does not address the poll.
 */
object ContestedIndexValue {

    /**
     * Encode a text value: one backslash is prepended when [value] starts
     * with `0x` or with a backslash, otherwise [value] is returned unchanged.
     * `0xf0rd` becomes `\0xf0rd`, `a11ce` stays `a11ce`.
     */
    fun text(value: String): String =
        if (value.startsWith("0x") || value.startsWith("\\")) "\\" + value else value

    /** Encode a byte value: `0x` followed by lowercase hex. */
    fun bytes(value: ByteArray): String = "0x" + value.toHex()
}

/**
 * Encode already-encoded index values (see [ContestedIndexValue]) as the JSON
 * string array the contested-resource FFI calls take. A real JSON serializer,
 * so control characters and quotes in a text value are escaped too.
 */
internal fun encodeIndexValuesJson(values: List<String>): String =
    JsonArray(values.map(::JsonPrimitive)).toString()
