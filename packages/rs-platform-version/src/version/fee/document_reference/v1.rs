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
///
/// A `refersTo` lookup query is an equality query over a unique index, limit 1, which follows
/// the index entry's reference to the document: through a one-property index of a few preorders
/// it measured 44,260 credits. Each layer of its path, the contracts, the contract's documents,
/// the type, a property name and value layer for each of the at most 10 properties an index
/// holds, the terminal and the type's primary key tree, at most about 30 layers, opens the
/// subtree and reads the key's node directly by its key (GroveDB's `Merk::get`), not by walking
/// the tree: two seeks (2,000 credits each) and two nodes' bytes (20 a byte). A node of an index
/// layer holds a key of at most 256 bytes and about 100 bytes of hashes and flags, so a layer
/// costs at most about 18,500 credits; the primary key tree's root node and the document read
/// each hold a document of at most 20 KiB, about 412,000 credits each. That is at most about
/// 30 x 18,500 + 2 x 412,000, 1,400,000 credits. Check tx reserves 12,000,000 credits
/// (0.00012 DASH), well above it, for each lookup query a create's references may make. It
/// bounds the balance a signer is held to, not the gas the transition is counted as wanting,
/// which check tx reports without it. The field is added in place: only check tx reads it, and
/// no lookup parses before protocol version 14.
pub const FEE_DOCUMENT_REFERENCE_VERSION1: FeeDocumentReferenceVersion =
    FeeDocumentReferenceVersion {
        choose_first_credits_per_candidate: 100_000,
        where_list_credits_per_element: 100,
        requirement_credits_per_comparison: 100,
        lookup_query_credits_reserved: 12_000_000,
    };
