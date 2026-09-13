#[cfg(feature = "state-transition-signing")]
use crate::serialization::Signable;

use platform_version::version::PlatformVersion;

use crate::consensus::basic::identity::{
    DisablingKeyIdAlsoBeingAddedInSameTransitionError, DuplicatedIdentityPublicKeyIdBasicError,
    InvalidIdentityUpdateTransitionEmptyError,
};
#[cfg(feature = "state-transition-signing")]
use crate::consensus::signature::{
    InvalidSignaturePublicKeySecurityLevelError, MissingPublicKeyError, SignatureError,
};
use crate::consensus::state::identity::max_identity_public_key_limit_reached_error::MaxIdentityPublicKeyLimitReachedError;
use crate::consensus::ConsensusError;
#[cfg(feature = "state-transition-signing")]
use crate::identity::signer::Signer;
#[cfg(feature = "state-transition-signing")]
use crate::identity::{Identity, IdentityPublicKey};
#[cfg(feature = "state-transition-signing")]
use crate::serialization::PlatformMessageSignable;
use crate::validation::SimpleConsensusValidationResult;
use crate::ProtocolError;

#[cfg(feature = "state-transition-signing")]
use crate::identity::accessors::IdentityGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
#[cfg(feature = "state-transition-signing")]
use crate::identity::SecurityLevel;
use crate::prelude::IdentityNonce;
#[cfg(feature = "state-transition-signing")]
use crate::prelude::UserFeeIncrease;
use crate::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use crate::state_transition::identity_update_transition::methods::IdentityUpdateTransitionMethodsV0;
use crate::state_transition::identity_update_transition::v0::IdentityUpdateTransitionV0;
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Getters;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::public_key_in_creation::accessors::IdentityPublicKeyInCreationV0Setters;
use crate::state_transition::public_key_in_creation::IdentityPublicKeyInCreation;
#[cfg(feature = "state-transition-signing")]
use crate::state_transition::{
    consensus_errors_as_protocol_error, GetDataContractSecurityLevelRequirementFn, StateTransition,
};
#[cfg(feature = "state-transition-signing")]
use crate::version::FeatureVersion;
use crate::{
    identity::KeyID,
    prelude::{Identifier, Revision},
};

/// Maximum number of identity public keys that may be disabled in a single
/// [`IdentityUpdateTransitionV0`]. Shared by client-side construction and
/// drive-abci basic-structure validation so both paths apply the same limit.
pub const MAX_IDENTITY_PUBLIC_KEYS_TO_DISABLE: usize = 10;

impl IdentityUpdateTransitionV0 {
    /// Dispatches basic-structure validation to the appropriate versioned
    /// implementation based on the active [`PlatformVersion`].
    ///
    /// The version source is the DPP-owned field
    /// `platform_version.dpp.state_transitions.identities.identity_update.basic_structure`
    /// — drive-abci's basic-structure dispatcher reads the same field, so the
    /// client and server cannot drift apart. This intentionally avoids having
    /// DPP depend on drive-abci-side version routing for a check whose
    /// definition lives in DPP.
    ///
    /// IMPORTANT: when a future v1 basic-structure check is introduced, both
    /// this wrapper and the drive-abci dispatcher must be updated in lockstep,
    /// and the SDK constructor [`try_from_identity_with_signer`] (which calls
    /// this method) must also be reviewed so it remains consistent with the
    /// server.
    pub fn validate_basic_structure(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        match platform_version
            .dpp
            .state_transitions
            .identities
            .identity_update
            .basic_structure
        {
            Some(0) => self.validate_basic_structure_v0(platform_version),
            Some(version) => Err(ProtocolError::UnknownVersionMismatch {
                method: "IdentityUpdateTransitionV0::validate_basic_structure".to_string(),
                known_versions: vec![0],
                received: version,
            }),
            // `None` represents "basic-structure validation is not active at
            // this PlatformVersion". Surface this with the dedicated
            // [`ProtocolError::VersionNotActive`] variant, which mirrors
            // drive-abci's `ExecutionError::VersionNotActive` semantics.
            None => Err(ProtocolError::VersionNotActive {
                method: "IdentityUpdateTransitionV0::validate_basic_structure".to_string(),
                known_versions: vec![0],
            }),
        }
    }

    /// Validates the basic structural invariants of this update transition.
    ///
    /// This mirrors the server-side basic-structure check used by drive-abci
    /// (`IdentityUpdateStateTransitionStructureValidationV0::validate_basic_structure_v0`)
    /// and is reused by the client-side constructor so that invalid transitions
    /// are caught before any signing work is performed.
    ///
    /// Checks performed (matching the server's v0 behavior):
    /// 1. Update is not empty (must add or disable at least one key); on
    ///    failure this returns immediately.
    /// 2. If `disable_public_keys.len() > MAX_IDENTITY_PUBLIC_KEYS_TO_DISABLE`,
    ///    a [`MaxIdentityPublicKeyLimitReachedError`] is **accumulated** —
    ///    not short-circuited — so duplicate-id and disable-also-added checks
    ///    still run against the same input. Any errors collected from the
    ///    disable-keys block cause an early return before structural
    ///    validation of `add_public_keys`.
    /// 3. Disabled key IDs are unique (first duplicate breaks the loop).
    /// 4. No disabled key ID is also being added in the same transition
    ///    (first overlap breaks the loop).
    /// 5. Public-key structure (counts, duplicates, purpose/security-level
    ///    constraints) via
    ///    [`IdentityPublicKeyInCreation::validate_identity_public_keys_structure`].
    pub fn validate_basic_structure_v0(
        &self,
        platform_version: &PlatformVersion,
    ) -> Result<SimpleConsensusValidationResult, ProtocolError> {
        let mut result = SimpleConsensusValidationResult::default();

        // 1. Ensure that either disablePublicKeys or addPublicKeys is present
        if self.disable_public_keys.is_empty() && self.add_public_keys.is_empty() {
            result.add_error(ConsensusError::from(
                InvalidIdentityUpdateTransitionEmptyError::new(),
            ));
            return Ok(result);
        }

        // 2-4. Validate public keys to disable
        if !self.disable_public_keys.is_empty() {
            // 2. Ensure max items — accumulate (don't return) so we still
            // surface duplicate-id and disable-also-added issues with the
            // same input, matching the old server-side behavior.
            if self.disable_public_keys.len() > MAX_IDENTITY_PUBLIC_KEYS_TO_DISABLE {
                result.add_error(ConsensusError::from(
                    MaxIdentityPublicKeyLimitReachedError::new(MAX_IDENTITY_PUBLIC_KEYS_TO_DISABLE),
                ));
            }

            // 3-4. Check key id duplicates and overlap with added keys
            let mut ids = std::collections::HashSet::new();
            for key_id in &self.disable_public_keys {
                if ids.contains(key_id) {
                    result.add_error(ConsensusError::from(
                        DuplicatedIdentityPublicKeyIdBasicError::new(vec![*key_id]),
                    ));
                    break;
                }

                if self
                    .add_public_keys
                    .iter()
                    .any(|public_key_in_creation| public_key_in_creation.id() == *key_id)
                {
                    result.add_error(ConsensusError::from(
                        DisablingKeyIdAlsoBeingAddedInSameTransitionError::new(*key_id),
                    ));
                    break;
                }

                ids.insert(key_id);
            }

            if !result.is_valid() {
                return Ok(result);
            }
        }

        // 5. Validate public-key structure (purpose/security level, duplicates, count)
        IdentityPublicKeyInCreation::validate_identity_public_keys_structure(
            &self.add_public_keys,
            false,
            platform_version,
        )
    }
}

impl IdentityUpdateTransitionMethodsV0 for IdentityUpdateTransitionV0 {
    #[cfg(feature = "state-transition-signing")]
    async fn try_from_identity_with_signer<S: Signer<IdentityPublicKey>>(
        identity: &Identity,
        master_public_key_id: &KeyID,
        add_public_keys: Vec<IdentityPublicKey>,
        disable_public_keys: Vec<KeyID>,
        nonce: IdentityNonce,
        user_fee_increase: UserFeeIncrease,
        signer: &S,
        platform_version: &PlatformVersion,
        _version: Option<FeatureVersion>,
    ) -> Result<StateTransition, ProtocolError> {
        let add_public_keys_in_creation: Vec<IdentityPublicKeyInCreation> = add_public_keys
            .iter()
            .map(|public_key| public_key.into())
            .collect();

        let mut identity_update_transition = IdentityUpdateTransitionV0 {
            signature: Default::default(),
            signature_public_key_id: 0,
            identity_id: identity.id(),
            revision: identity.revision(),
            nonce,
            add_public_keys: add_public_keys_in_creation,
            disable_public_keys,
            user_fee_increase,
        };

        // Fail-fast: verify the master public key exists on the identity and
        // has `SecurityLevel::MASTER` *before* doing any POP signing work for
        // added unique keys. Catching this here matches the final signing
        // contract and avoids spending signer cycles on a transition we
        // already know cannot be signed.
        let master_public_key = identity
            .public_keys()
            .get(master_public_key_id)
            .ok_or::<ConsensusError>(
                SignatureError::MissingPublicKeyError(MissingPublicKeyError::new(
                    *master_public_key_id,
                ))
                .into(),
            )?;
        if master_public_key.security_level() != SecurityLevel::MASTER {
            return Err(ProtocolError::from(ConsensusError::from(
                InvalidSignaturePublicKeySecurityLevelError::new(
                    master_public_key.security_level(),
                    vec![SecurityLevel::MASTER],
                ),
            )));
        }

        // Run the same basic-structure checks as the server-side
        // `IdentityUpdateStateTransitionStructureValidationV0` impl, going
        // through the shared version-dispatching wrapper so client and server
        // pick the same versioned check. When a future v1 basic-structure
        // check is introduced, the server dispatcher, the wrapper, and this
        // constructor must be updated in lockstep.
        let basic_structure_result =
            identity_update_transition.validate_basic_structure(platform_version)?;
        if let Some(error) = consensus_errors_as_protocol_error(basic_structure_result) {
            return Err(error);
        }

        let state_transition: StateTransition = identity_update_transition.clone().into();

        let key_signable_bytes = state_transition.signable_bytes()?;

        // Sign all the keys
        for (public_key_with_witness, public_key) in identity_update_transition
            .add_public_keys
            .iter_mut()
            .zip(add_public_keys.iter())
        {
            if public_key.key_type().is_unique_key_type() {
                let signature = signer.sign(public_key, &key_signable_bytes).await?;
                public_key_with_witness.set_signature(signature);
            }
        }

        // Verify proof-of-possession signatures we just produced before
        // returning, matching the server-side
        // `IdentityUpdateStateTransitionIdentityAndSignaturesValidationV0`
        // check. Only keys with unique types were signed above, so verify
        // those exact keys here.
        for public_key_with_witness in identity_update_transition.add_public_keys.iter() {
            if !public_key_with_witness.key_type().is_unique_key_type() {
                continue;
            }
            let pop_result = key_signable_bytes.as_slice().verify_signature(
                public_key_with_witness.key_type(),
                public_key_with_witness.data().as_slice(),
                public_key_with_witness.signature().as_slice(),
            );
            if let Some(error) = consensus_errors_as_protocol_error(pop_result) {
                return Err(error);
            }
        }

        let mut state_transition: StateTransition = identity_update_transition.into();
        state_transition
            .sign_external(
                master_public_key,
                signer,
                None::<GetDataContractSecurityLevelRequirementFn>,
            )
            .await?;
        Ok(state_transition)
    }
}

impl IdentityUpdateTransitionAccessorsV0 for IdentityUpdateTransitionV0 {
    fn set_identity_id(&mut self, id: Identifier) {
        self.identity_id = id;
    }

    fn identity_id(&self) -> Identifier {
        self.identity_id
    }

    fn set_revision(&mut self, revision: Revision) {
        self.revision = revision;
    }

    fn revision(&self) -> Revision {
        self.revision
    }

    fn set_nonce(&mut self, nonce: IdentityNonce) {
        self.nonce = nonce;
    }

    fn nonce(&self) -> IdentityNonce {
        self.nonce
    }

    fn set_public_keys_to_add(&mut self, add_public_keys: Vec<IdentityPublicKeyInCreation>) {
        self.add_public_keys = add_public_keys;
    }

    fn public_keys_to_add(&self) -> &[IdentityPublicKeyInCreation] {
        &self.add_public_keys
    }

    fn public_keys_to_add_mut(&mut self) -> &mut [IdentityPublicKeyInCreation] {
        &mut self.add_public_keys
    }

    fn set_public_key_ids_to_disable(&mut self, disable_public_keys: Vec<KeyID>) {
        self.disable_public_keys = disable_public_keys;
    }

    fn public_key_ids_to_disable(&self) -> &[KeyID] {
        &self.disable_public_keys
    }
}
