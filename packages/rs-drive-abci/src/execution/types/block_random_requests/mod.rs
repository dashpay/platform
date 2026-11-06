use drive::drive::document::draw_random_document_properties::RandomDocumentRequest;
use drive::drive::tokens::settle_random_token_amounts::RandomTokenRequest;

/// What a block's random beacon fills in once the block's prevotes combine into it (protocol
/// version 15): every document the block created, whose `sys.random` properties are drawn, and
/// every random token transfer and mint, whose amount is drawn and settled.
///
/// `process_raw_state_transitions` collects them from the transitions the block executed
/// successfully, in the order it executed them, and the block execution context keeps them
/// until `ApplyBlockRandomness`, or `FinalizeBlock` with the commit's beacon, hands the beacon
/// over. Empty before protocol version 15, where nothing is collected.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BlockRandomRequests {
    /// The documents the block created, whose `sys.random` properties are drawn; a document
    /// whose type declares none is left alone
    pub documents: Vec<RandomDocumentRequest>,
    /// The random token transfers and mints of the block, whose amounts are drawn and settled
    pub tokens: Vec<RandomTokenRequest>,
}

impl BlockRandomRequests {
    /// Whether the block asked for no random value.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty() && self.tokens.is_empty()
    }

    /// Appends the requests of `other` after these, keeping the order of each list.
    pub fn append(&mut self, mut other: BlockRandomRequests) {
        self.documents.append(&mut other.documents);
        self.tokens.append(&mut other.tokens);
    }
}
