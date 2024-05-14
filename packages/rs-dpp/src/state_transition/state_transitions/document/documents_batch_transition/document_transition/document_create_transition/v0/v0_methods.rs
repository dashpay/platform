use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;

use platform_value::Value;

use crate::fee::Credits;
use std::collections::BTreeMap;

pub trait DocumentCreateTransitionV0Methods {
    /// Returns a reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base(&self) -> &DocumentBaseTransition;

    /// Returns a mut reference to the `base` field of the `DocumentCreateTransitionV0`.
    fn base_mut(&mut self) -> &mut DocumentBaseTransition;

    /// Sets the value of the `base` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `base` - A value of type `DocumentBaseTransition` to set.
    fn set_base(&mut self, base: DocumentBaseTransition);

    /// Returns a reference to the `entropy` field of the `DocumentCreateTransitionV0`.
    fn entropy(&self) -> [u8; 32];

    /// Sets the value of the `entropy` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `entropy` - An array of 32 bytes to set.
    fn set_entropy(&mut self, entropy: [u8; 32]);

    /// Returns an optional reference to the `data` field of the `DocumentCreateTransitionV0`.
    fn data(&self) -> &BTreeMap<String, Value>;

    /// Returns an optional mutable reference to the `data` field of the `DocumentCreateTransitionV0`.
    fn data_mut(&mut self) -> &mut BTreeMap<String, Value>;

    /// Sets the value of the `data` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `data` - An `Option` containing a `BTreeMap<String, Value>` to set.
    fn set_data(&mut self, data: BTreeMap<String, Value>);
    fn prefunded_voting_balance(&self) -> &Option<(String, Credits)>;
    fn prefunded_voting_balances_mut(&mut self) -> &mut Option<(String, Credits)>;
    fn set_prefunded_voting_balance(&mut self, index_name: String, amount: Credits);
    fn clear_prefunded_voting_balance(&mut self);
}

impl DocumentCreateTransitionV0Methods for DocumentCreateTransitionV0 {
    fn base(&self) -> &DocumentBaseTransition {
        &self.base
    }

    fn base_mut(&mut self) -> &mut DocumentBaseTransition {
        &mut self.base
    }

    fn set_base(&mut self, base: DocumentBaseTransition) {
        self.base = base;
    }

    fn entropy(&self) -> [u8; 32] {
        self.entropy
    }

    fn set_entropy(&mut self, entropy: [u8; 32]) {
        self.entropy = entropy;
    }

    fn data(&self) -> &BTreeMap<String, Value> {
        &self.data
    }

    fn data_mut(&mut self) -> &mut BTreeMap<String, Value> {
        &mut self.data
    }

    fn set_data(&mut self, data: BTreeMap<String, Value>) {
        self.data = data;
    }

    fn prefunded_voting_balance(&self) -> &Option<(String, Credits)> {
        &self.prefunded_voting_balance
    }

    fn prefunded_voting_balances_mut(&mut self) -> &mut Option<(String, Credits)> {
        &mut self.prefunded_voting_balance
    }

    fn set_prefunded_voting_balance(&mut self, index_name: String, amount: Credits) {
        self.prefunded_voting_balance = Some((index_name, amount));
    }
    fn clear_prefunded_voting_balance(&mut self) {
        self.prefunded_voting_balance = None;
    }
}
