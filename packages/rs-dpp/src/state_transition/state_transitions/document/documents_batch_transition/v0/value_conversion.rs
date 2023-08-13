use std::collections::{BTreeMap, HashMap};

use platform_value::btreemap_extensions::{
    BTreeValueMapHelper, BTreeValueMapReplacementPathHelper,
};
use platform_value::{BinaryData, IntegerReplacementType, ReplacementType, Value};
use serde_json::Value as JsonValue;

use crate::{data_contract::DataContract, identity::KeyID, prelude::Identifier, ProtocolError};

use crate::state_transition::data_contract_update_transition::U32_FIELDS;
use crate::state_transition::documents_batch_transition::document_transition::DocumentTransition;
use crate::state_transition::documents_batch_transition::fields::property_names::{
    OWNER_ID, TRANSITIONS,
};
use crate::state_transition::documents_batch_transition::fields::*;
use crate::state_transition::documents_batch_transition::{
    document_base_transition, document_create_transition, DocumentsBatchTransitionV0,
};
use crate::state_transition::{StateTransitionFieldTypes, StateTransitionValueConvert};
use bincode::{config, Decode, Encode};
use platform_version::version::PlatformVersion;

impl<'a> StateTransitionValueConvert<'a> for DocumentsBatchTransitionV0 {}
