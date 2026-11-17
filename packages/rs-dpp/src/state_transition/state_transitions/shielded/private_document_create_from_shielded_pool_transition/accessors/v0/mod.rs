use crate::shielded::SerializedAction;
use platform_value::{BinaryData, Identifier};

/// Accessors for the fields of a `PrivateDocumentCreateFromShieldedPoolTransition`.
pub trait PrivateDocumentCreateFromShieldedPoolTransitionAccessorsV0 {
    /// The contract that defines the private document type.
    fn data_contract_id(&self) -> Identifier;
    /// Set the contract.
    fn set_data_contract_id(&mut self, data_contract_id: Identifier);

    /// The private document type the entries are appended to.
    fn document_type_name(&self) -> &str;
    /// Set the document type name.
    fn set_document_type_name(&mut self, document_type_name: String);

    /// The entries, in the order they are appended.
    fn entries(&self) -> &[BinaryData];
    /// Replace the entries.
    fn set_entries(&mut self, entries: Vec<BinaryData>);

    /// Get the serialized Orchard actions (spend/output pairs).
    fn actions(&self) -> &[SerializedAction];
    /// Replace the serialized Orchard actions.
    fn set_actions(&mut self, actions: Vec<SerializedAction>);

    /// Credits leaving the credit pool: the bundle's value balance, which is the fee.
    fn value_balance(&self) -> u64;
    /// Set the value balance.
    fn set_value_balance(&mut self, value_balance: u64);

    /// Get the Orchard anchor (Sinsemilla root of the note commitment tree).
    fn anchor(&self) -> [u8; 32];
    /// Set the Orchard anchor.
    fn set_anchor(&mut self, anchor: [u8; 32]);

    /// Get the Halo2 proof bytes.
    fn proof(&self) -> &[u8];
    /// Set the Halo2 proof bytes.
    fn set_proof(&mut self, proof: Vec<u8>);

    /// Get the RedPallas binding signature.
    fn binding_signature(&self) -> [u8; 64];
    /// Set the RedPallas binding signature.
    fn set_binding_signature(&mut self, binding_signature: [u8; 64]);

    /// Extract nullifier bytes from each action.
    fn nullifiers<N: From<[u8; 32]>>(&self) -> Vec<N> {
        self.actions()
            .iter()
            .map(|a| N::from(a.nullifier))
            .collect()
    }
}
