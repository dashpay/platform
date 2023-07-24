use crate::platform_serialization::PlatformSignable;
use crate::serialization_traits::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_value::{BinaryData, ReplacementType, Value};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::convert::{TryFrom, TryInto};

use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, SignatureError,
};
use crate::consensus::ConsensusError;
use crate::identity::signer::Signer;
use crate::identity::{Identity, IdentityPublicKey};

use crate::identity::SecurityLevel::{CRITICAL, MASTER};
use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
use crate::version::FeatureVersion;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::{Identifier, Revision, TimestampMillis},
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    version::LATEST_VERSION,
    ProtocolError,
};
use platform_serialization::{PlatformDeserialize, PlatformSerialize};

pub trait IdentityCreditTransferTransitionV0Methods {
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityCreditTransfer
    }

    fn set_amount(&mut self, amount: u64);
    fn amount(&self) -> u64;
    fn identity_id(&self) -> Identifier;
    fn set_identity_id(&mut self, identity_id: Identifier);
    fn recipient_id(&self) -> Identifier;
    fn set_recipient_id(&mut self, recipient_id: Identifier);
    fn security_level_requirement(&self) -> Vec<SecurityLevel>;
}

impl IdentityCreditTransferTransitionV0Methods for IdentityCreditTransferTransitionV0 {
    fn set_identity_id(&mut self, identity_id: Identifier) {
        self.identity_id = identity_id;
    }

    fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    fn set_recipient_id(&mut self, recipient_id: Identifier) {
        self.recipient_id = recipient_id;
    }

    fn recipient_id(&self) -> Identifier {
        self.recipient_id
    }

    fn amount(&self) -> u64 {
        self.amount
    }

    fn set_amount(&mut self, amount: u64) {
        self.amount = amount;
    }

    fn security_level_requirement(&self) -> Vec<SecurityLevel> {
        vec![CRITICAL]
    }
}
