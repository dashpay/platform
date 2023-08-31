use crate::identity::TimestampMillis;

use crate::state_transition::documents_batch_transition::document_base_transition::DocumentBaseTransition;
use crate::state_transition::documents_batch_transition::document_create_transition::DocumentCreateTransitionV0;

use platform_value::Value;

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

    /// Returns a reference to the `created_at` field of the `DocumentCreateTransitionV0`.
    fn created_at(&self) -> Option<TimestampMillis>;

    /// Sets the value of the `created_at` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `created_at` - An `Option` containing a `TimestampMillis` value to set.
    fn set_created_at(&mut self, created_at: Option<TimestampMillis>);

    /// Returns a reference to the `updated_at` field of the `DocumentCreateTransitionV0`.
    fn updated_at(&self) -> Option<TimestampMillis>;

    /// Sets the value of the `updated_at` field in the `DocumentCreateTransitionV0`.
    ///
    /// # Arguments
    ///
    /// * `updated_at` - An `Option` containing a `TimestampMillis` value to set.
    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>);

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

    fn created_at(&self) -> Option<TimestampMillis> {
        self.created_at
    }

    fn set_created_at(&mut self, created_at: Option<TimestampMillis>) {
        self.created_at = created_at;
    }

    fn updated_at(&self) -> Option<TimestampMillis> {
        self.updated_at
    }

    fn set_updated_at(&mut self, updated_at: Option<TimestampMillis>) {
        self.updated_at = updated_at;
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
}
