#[cfg(feature = "state-transition-transformers")]
pub mod transformer;

use crate::data_contract::DataContract;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct DataContractCreateTransitionActionV0 {
    pub data_contract: DataContract,
}
