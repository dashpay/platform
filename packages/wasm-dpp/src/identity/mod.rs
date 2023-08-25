mod factory_utils;
pub mod identity_facade;
mod identity_factory;
mod identity_public_key;
// mod validation;
//
// use js_sys::Array;
// use serde_json::Value;
// use wasm_bindgen::prelude::*;
//
// use dpp::identity::state_transition::asset_lock_proof::AssetLockProof;
// use dpp::identity::IdentityPublicKey;
// use dpp::identity::{Identity, KeyID};
// use dpp::metadata::Metadata;
// use dpp::serialization::serialization_traits::{PlatformDeserializable, PlatformSerializable};
// use dpp::{ ProtocolError};
//
// use crate::identifier::IdentifierWrapper;
// use crate::utils::{to_vec_of_serde_values, IntoWasm, WithJsError};
// use crate::MetadataWasm;
// use crate::{utils, with_js_error};
pub use identity_public_key::*;
//
// use crate::buffer::Buffer;
// use crate::errors::from_dpp_err;
// pub use state_transition::*;
//
// pub mod credits_converter;
pub mod errors;
mod identity;
pub mod state_transition;

pub use identity::*;
