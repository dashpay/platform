use crate::version::fee::document_reference::FeeDocumentReferenceVersion;

/// Introduced with protocol version 14 (4.2), the first version that parses `chooseFirst`;
/// every fee schedule references it, and none before 14 reads it.
///
/// A competitor costs about 50,000 credits to read and hash for a commitment of a few
/// fixed-size hashes (the document load and its index walk, plus a two-block double SHA-256),
/// so 100,000 covers it twice over. At the limit of 1,000 competitors the budget is 100,000,000
/// credits (0.001 DASH), which a signer must hold beyond the create's estimated fee to be
/// admitted, and which a reveal is charged only as far as its scan actually costs.
pub const FEE_DOCUMENT_REFERENCE_VERSION1: FeeDocumentReferenceVersion =
    FeeDocumentReferenceVersion {
        choose_first_credits_per_candidate: 100_000,
    };
