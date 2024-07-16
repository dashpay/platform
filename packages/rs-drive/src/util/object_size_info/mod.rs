//! Object Size Info
//!
//! This module defines enums and implements functions relevant to the sizes of objects.
//!

#[cfg(any(feature = "server", feature = "verify"))]
mod contract_info;
#[cfg(feature = "server")]
mod document_and_contract_info;
#[cfg(feature = "server")]
mod document_info;
#[cfg(feature = "server")]
mod drive_key_info;
#[cfg(feature = "server")]
mod element_info;
#[cfg(feature = "server")]
mod key_element_info;
#[cfg(feature = "server")]
mod key_value_info;
#[cfg(feature = "server")]
mod owned_document_info;
#[cfg(feature = "server")]
mod path_info;
#[cfg(feature = "server")]
mod path_key_element_info;
#[cfg(feature = "server")]
mod path_key_info;

#[cfg(feature = "server")]
pub use contract_info::*;
#[cfg(all(feature = "verify", not(feature = "server")))]
pub use contract_info::{DataContractOwnedResolvedInfo, DataContractResolvedInfo};
#[cfg(feature = "server")]
pub use document_and_contract_info::*;
#[cfg(feature = "server")]
pub use document_info::*;
#[cfg(feature = "server")]
pub use drive_key_info::*;
#[cfg(feature = "server")]
pub use element_info::*;
#[cfg(feature = "server")]
pub use key_element_info::*;
#[cfg(feature = "server")]
pub use key_value_info::*;
#[cfg(feature = "server")]
pub use owned_document_info::*;
#[cfg(feature = "server")]
pub use path_info::*;
#[cfg(feature = "server")]
pub use path_key_element_info::*;
#[cfg(feature = "server")]
pub use path_key_info::*;
