use crate::version::fee::document_reference::FeeDocumentReferenceVersion;

/// Introduced with protocol version 14 (4.2), the first version that parses `chooseFirst`;
/// every fee schedule references it, and none before 14 reads it.
///
/// A competitor costs about 50,000 credits to read and hash for a commitment of a few
/// fixed-size hashes (the document load and its index walk, plus a two-block double SHA-256),
/// so 100,000 covers it twice over. At the limit of 1,000 competitors the budget is 100,000,000
/// credits (0.001 DASH), which a signer must hold beyond the create's estimated fee to be
/// admitted, and which a reveal is charged only as far as its scan actually costs.
///
/// A list element a `where` entry reads costs a comparison, or a set insertion and a lookup
/// for `containsAll`: about a BLAKE3 block's work, which `blake3_per_block` bills at 300, so
/// 100 an element covers it. Two lists of 1,024 elements bill 204,800 credits.
///
/// A comparison judging a document type against `documentTypeRequirements` is a match on a
/// property's type or a lookup of a short string, no more than a list element's
/// comparison, so it bills the same 100. A type of 100 properties under four `"$any"`
/// slots, each with a `when` of 16 nodes, is 6,800 comparisons: 680,000 credits, the most
/// one reference can bill. The field is added in place: nothing reads it before protocol
/// version 15.
pub const FEE_DOCUMENT_REFERENCE_VERSION1: FeeDocumentReferenceVersion =
    FeeDocumentReferenceVersion {
        choose_first_credits_per_candidate: 100_000,
        where_list_credits_per_element: 100,
        requirement_credits_per_comparison: 100,
    };
