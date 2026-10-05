//! Filters for the `subscribeToStateTransitions` stream.
//!
//! DAPI evaluates these filters against committed state transitions; the SDK builds them and
//! re-checks every transition the stream delivers with the same code, so a node cannot pass
//! off a transition that does not match.
//!
//! - [`StateTransitionFilter`] is the transport-free form of one request filter, convertible
//!   to and from its wire message ([`StateTransitionFilter::to_proto`],
//!   `TryFrom<proto::StateTransitionFilter>`).
//! - [`ResolvedFilters::resolve`] validates a request's filters and binds every document
//!   filter to the data contract it names; [`ResolvedFilters::matches`] then tells which
//!   filters a transition matches.
//!
//! Matching reads only what a transition says. A transfer's amount is the amount it asks
//! for, not what fees leave to credit, and of a document before the transition changed it
//! only the `$id` is known: clauses on the original document are limited to `$id`, except for
//! the delete of an indexOnly document, which carries the document's values.

mod participants;
mod proto;
mod resolved;
#[cfg(test)]
mod tests;

use dpp::address_funds::PlatformAddress;
use dpp::prelude::Identifier;
use drive::query::{ValueClause, WhereClause};

pub use proto::subscribe_request;
pub use resolved::{canonical_operands, FilterMatch, ResolvedFilters};

/// Most filters one subscription may carry.
pub const MAX_FILTERS: usize = 16;
/// Most addresses one address filter may watch.
pub const MAX_ADDRESSES_PER_FILTER: usize = 256;
/// Most identities, tokens or data contracts one filter may list.
pub const MAX_IDS_PER_FILTER: usize = 64;
/// Most action matches one document filter may carry.
pub const MAX_ACTIONS_PER_DOCUMENT_FILTER: usize = 12;
/// Most where clauses one action match may carry, per document side.
pub const MAX_CLAUSES_PER_ACTION: usize = 16;

/// Which side of a transition a watched identity or address must be on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum Role {
    /// Either side.
    #[default]
    Any,
    /// The identity that owns (signs) the transition, or an address input it spends from.
    Sender,
    /// An identity or address the transition names as beneficiary or target.
    Recipient,
}

impl Role {
    /// Whether a party on side `side` satisfies this role.
    pub fn admits(self, side: Role) -> bool {
        self == Role::Any || self == side
    }
}

/// A document transition's action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DocumentAction {
    /// A document is created.
    Create,
    /// A document is replaced.
    Replace,
    /// A document is deleted, including the delete of an indexOnly document.
    Delete,
    /// A document is transferred to another identity.
    Transfer,
    /// A document's price is set.
    UpdatePrice,
    /// A document is purchased.
    Purchase,
}

/// One action of a [`DocumentFilter`], optionally narrowed by clauses (ANDed together).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DocumentActionMatch {
    /// The action; `None` only while building, rejected when resolved.
    pub action: Option<DocumentAction>,
    /// Create and replace: clauses on the document data the transition carries.
    pub new_document_where: Vec<WhereClause>,
    /// Clauses on the document as it was before the transition: `$id` only, except for the
    /// delete of an indexOnly document.
    pub original_document_where: Vec<WhereClause>,
    /// Transfer: the recipient, purchase: the buyer, must be one of these identities.
    pub owner_ids: Vec<Identifier>,
    /// Update price: constraint on the new price (the clause's field is ignored).
    pub price: Option<ValueClause>,
}

impl DocumentActionMatch {
    /// Match every transition with `action`.
    pub fn new(action: DocumentAction) -> Self {
        Self {
            action: Some(action),
            ..Default::default()
        }
    }

    /// Add a clause on the new document data.
    pub fn with_new_document_where(mut self, clause: WhereClause) -> Self {
        self.new_document_where.push(clause);
        self
    }

    /// Add a clause on the document before the transition.
    pub fn with_original_document_where(mut self, clause: WhereClause) -> Self {
        self.original_document_where.push(clause);
        self
    }

    /// Require the transfer recipient or the buyer to be one of `owner_ids`.
    pub fn with_owner_ids(mut self, owner_ids: impl IntoIterator<Item = Identifier>) -> Self {
        self.owner_ids.extend(owner_ids);
        self
    }

    /// Constrain the new price of an update-price.
    pub fn with_price(mut self, price: ValueClause) -> Self {
        self.price = Some(price);
        self
    }

    fn has_constraints(&self) -> bool {
        !self.new_document_where.is_empty()
            || !self.original_document_where.is_empty()
            || !self.owner_ids.is_empty()
            || self.price.is_some()
    }
}

/// Document transitions, in batch transitions, on one data contract.
#[derive(Debug, Clone, PartialEq)]
pub struct DocumentFilter {
    /// The data contract.
    pub data_contract_id: Identifier,
    /// `None`: every document type of the contract; the actions may then carry no clauses.
    pub document_type_name: Option<String>,
    /// Empty: every action.
    pub actions: Vec<DocumentActionMatch>,
    /// The identity that owns (signs) the batch.
    pub batch_owner_id: Option<Identifier>,
}

impl DocumentFilter {
    /// Every document transition on `data_contract_id`.
    pub fn new(data_contract_id: Identifier) -> Self {
        Self {
            data_contract_id,
            document_type_name: None,
            actions: Vec::new(),
            batch_owner_id: None,
        }
    }

    /// Only documents of `document_type_name`.
    pub fn with_document_type(mut self, document_type_name: impl Into<String>) -> Self {
        self.document_type_name = Some(document_type_name.into());
        self
    }

    /// Add an action alternative.
    pub fn with_action(mut self, action: DocumentActionMatch) -> Self {
        self.actions.push(action);
        self
    }

    /// Only batches owned (signed) by `batch_owner_id`.
    pub fn with_batch_owner(mut self, batch_owner_id: Identifier) -> Self {
        self.batch_owner_id = Some(batch_owner_id);
        self
    }
}

/// One filter of a state transition subscription.
#[derive(Debug, Clone, PartialEq)]
pub enum StateTransitionFilter {
    /// Document transitions on one data contract.
    Documents(DocumentFilter),
    /// Transitions that name one of these platform addresses on the `role` side.
    Addresses {
        /// The watched addresses.
        addresses: Vec<PlatformAddress>,
        /// The side they must be on.
        role: Role,
    },
    /// Transitions that name one of these identities on the `role` side.
    Identities {
        /// The watched identities.
        identity_ids: Vec<Identifier>,
        /// The side they must be on.
        role: Role,
    },
    /// Token transitions, in batch transitions.
    Tokens {
        /// Empty: every token.
        token_ids: Vec<Identifier>,
        /// Empty: any party; otherwise one must be on the `role` side.
        identity_ids: Vec<Identifier>,
        /// The side `identity_ids` must be on.
        role: Role,
    },
    /// Creation and updates of these data contracts, and moderation and fee claims on them.
    DataContracts {
        /// The watched data contracts.
        data_contract_ids: Vec<Identifier>,
    },
}

impl StateTransitionFilter {
    /// The data contract a document filter names.
    pub fn document_data_contract_id(&self) -> Option<Identifier> {
        match self {
            StateTransitionFilter::Documents(filter) => Some(filter.data_contract_id),
            _ => None,
        }
    }
}

/// Why a subscription's filters cannot be served.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SubscriptionFilterError {
    /// The request as a whole is malformed.
    #[error("invalid subscription request: {0}")]
    InvalidRequest(String),
    /// One filter is malformed or cannot be decided from a transition.
    #[error("invalid subscription filter {index}: {message}")]
    InvalidFilter {
        /// Position of the filter in the request.
        index: usize,
        /// What is wrong.
        message: String,
    },
    /// A document filter names a data contract that does not exist.
    #[error("data contract {0} not found")]
    DataContractNotFound(Identifier),
}

impl SubscriptionFilterError {
    fn filter(index: usize, message: impl Into<String>) -> Self {
        Self::InvalidFilter {
            index,
            message: message.into(),
        }
    }
}
