mod v0;

pub use v0::*;

use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;

impl IdentityCreditTransferTransitionMethodsV0 for IdentityCreditTransferTransition {}
