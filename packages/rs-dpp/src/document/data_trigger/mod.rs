mod data_trigger_execution_context;
pub use data_trigger_execution_context::*;

mod data_trigger_execution_result;
pub use data_trigger_execution_result::*;

mod reject_data_trigger;
pub use reject_data_trigger::*;

use crate::prelude::Identifier;

use super::document_transition::{Action, DocumentTransition};

// TODO
#[derive(Debug)]
pub struct DataTrigger {}
