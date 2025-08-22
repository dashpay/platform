use crate::types::RetrievedObjects;
use dpp::identifier::Identifier;
use dpp::tokens::status::TokenStatus;

/// Token statuses (i.e. is token paused or not)
/// Token ID to token status
pub type TokenStatuses = RetrievedObjects<Identifier, TokenStatus>;
