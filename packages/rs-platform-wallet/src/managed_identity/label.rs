//! Label management for ManagedIdentity

use super::ManagedIdentity;

impl ManagedIdentity {
    /// Set the label for this identity
    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    /// Clear the label for this identity
    pub fn clear_label(&mut self) {
        self.label = None;
    }
}
