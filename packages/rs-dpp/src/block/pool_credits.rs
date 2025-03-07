use crate::fee::Credits;
use std::fmt;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct StorageAndProcessingPoolCredits {
    pub storage_pool_credits: Credits,
    pub processing_pool_credits: Credits,
    pub total_credits: Credits,
}

impl fmt::Display for StorageAndProcessingPoolCredits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Storage: {}, Processing: {}, Total: {}",
            self.storage_pool_credits, self.processing_pool_credits, self.total_credits
        )
    }
}
