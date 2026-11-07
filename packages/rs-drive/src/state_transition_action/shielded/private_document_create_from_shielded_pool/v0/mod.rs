mod transformer;

use crate::state_transition_action::shielded::ShieldedActionNote;
use dpp::fee::Credits;
use dpp::platform_value::Identifier;

/// Entries appended to a private document type's store, paid from the credit shielded pool,
/// version 0.
#[derive(Debug, Clone)]
pub struct PrivateDocumentCreateFromShieldedPoolTransitionActionV0 {
    /// The contract that defines the private document type
    pub data_contract_id: Identifier,
    /// The private document type the entries are appended to
    pub document_type_name: String,
    /// The entries, in the order they are appended, each the type's `entrySize`
    pub entries: Vec<Vec<u8>>,
    /// Notes built 1:1 from the on-wire Orchard actions
    pub notes: Vec<ShieldedActionNote>,
    /// The anchor the spend was proven against
    pub anchor: [u8; 32],
    /// The fee, the bundle's whole value balance, routed to the fee pools
    pub fee_amount: Credits,
    /// Pool total balance read at transform time
    pub current_total_balance: Credits,
}
