use crate::state_transition::documents_batch_transition::DocumentsBatchTransition;
use crate::state_transition::documents_batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;

pub mod v0;

impl DocumentsBatchTransitionMethodsV0 for DocumentsBatchTransition {}