use crate::fee::Credits;

pub struct StorageAndProcessingPoolCredits {
    pub storage_pool_credits: Credits,
    pub processing_pool_credits: Credits,
    pub total_credits: Credits,
}
