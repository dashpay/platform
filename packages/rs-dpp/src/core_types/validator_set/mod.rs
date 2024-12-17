use crate::bls_signatures::{Bls12381G2Impl, PublicKey as BlsPublicKey};
use crate::core_types::validator::v0::ValidatorV0;
use crate::core_types::validator_set::v0::{
    ValidatorSetV0, ValidatorSetV0Getters, ValidatorSetV0Setters,
};
#[cfg(feature = "core-types-serialization")]
use crate::ProtocolError;
#[cfg(feature = "core-types-serialization")]
use bincode::{Decode, Encode};
use dashcore::{ProTxHash, QuorumHash};
#[cfg(feature = "core-types-serialization")]
use platform_serialization_derive::{PlatformDeserialize, PlatformSerialize};
#[cfg(feature = "core-types-serde-conversion")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};

/// Version 0
pub mod v0;

/// The validator set is only slightly different from a quorum as it does not contain non valid
/// members
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(
    feature = "core-types-serde-conversion",
    derive(Serialize, Deserialize)
)]
#[cfg_attr(
    feature = "core-types-serialization",
    derive(Encode, Decode, PlatformDeserialize, PlatformSerialize),
    platform_serialize(limit = 15000, unversioned)
)]
pub enum ValidatorSet {
    /// Version 0
    V0(ValidatorSetV0),
}

impl Display for ValidatorSet {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ValidatorSet::V0(v0) => write!(f, "{}", v0),
        }
    }
}

impl ValidatorSetV0Getters for ValidatorSet {
    fn quorum_hash(&self) -> &QuorumHash {
        match self {
            ValidatorSet::V0(v0) => v0.quorum_hash(),
        }
    }

    fn quorum_index(&self) -> Option<u32> {
        match self {
            ValidatorSet::V0(v0) => v0.quorum_index(),
        }
    }

    fn core_height(&self) -> u32 {
        match self {
            ValidatorSet::V0(v0) => v0.core_height(),
        }
    }

    fn members(&self) -> &BTreeMap<ProTxHash, ValidatorV0> {
        match self {
            ValidatorSet::V0(v0) => v0.members(),
        }
    }

    fn members_mut(&mut self) -> &mut BTreeMap<ProTxHash, ValidatorV0> {
        match self {
            ValidatorSet::V0(v0) => v0.members_mut(),
        }
    }

    fn members_owned(self) -> BTreeMap<ProTxHash, ValidatorV0> {
        match self {
            ValidatorSet::V0(v0) => v0.members_owned(),
        }
    }

    fn threshold_public_key(&self) -> &BlsPublicKey<Bls12381G2Impl> {
        match self {
            ValidatorSet::V0(v0) => v0.threshold_public_key(),
        }
    }
}

impl ValidatorSetV0Setters for ValidatorSet {
    fn set_quorum_hash(&mut self, quorum_hash: QuorumHash) {
        match self {
            ValidatorSet::V0(v0) => v0.set_quorum_hash(quorum_hash),
        }
    }

    fn set_quorum_index(&mut self, index: Option<u32>) {
        match self {
            ValidatorSet::V0(v0) => v0.set_quorum_index(index),
        }
    }

    fn set_core_height(&mut self, core_height: u32) {
        match self {
            ValidatorSet::V0(v0) => v0.set_core_height(core_height),
        }
    }

    fn set_members(&mut self, members: BTreeMap<ProTxHash, ValidatorV0>) {
        match self {
            ValidatorSet::V0(v0) => v0.set_members(members),
        }
    }

    fn set_threshold_public_key(&mut self, threshold_public_key: BlsPublicKey<Bls12381G2Impl>) {
        match self {
            ValidatorSet::V0(v0) => v0.set_threshold_public_key(threshold_public_key),
        }
    }
}
