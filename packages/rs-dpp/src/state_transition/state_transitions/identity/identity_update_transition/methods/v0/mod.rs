use crate::serialization::{PlatformDeserializable, Signable};
use bincode::{config, Decode, Encode};
use platform_serialization_derive::PlatformSignable;
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
use crate::state_transition::StateTransition;
use crate::version::FeatureVersion;
use crate::{
    identity::{KeyID, SecurityLevel},
    prelude::{Identifier, Revision, TimestampMillis},
    state_transition::{StateTransitionFieldTypes, StateTransitionLike, StateTransitionType},
    version::LATEST_VERSION,
    ProtocolError,
};

pub trait IdentityUpdateTransitionMethodsV0 {
    #[cfg(feature = "state-transition-signing")]
    fn try_from_identity_with_signer<S: Signer>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        public_keys_disabled_at: Option<u64>,
        signer: &S,
        version: FeatureVersion,
    ) -> Result<StateTransition, ProtocolError>;
    /// Get State Transition Type
    fn get_type() -> StateTransitionType {
        StateTransitionType::IdentityUpdate
    }
}
