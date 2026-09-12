//! Family detection from the wire prefix of a serialized state transition, and the decode
//! entry points that use it.
//!
//! Every ingress path that accepts a serialized state transition (the block decoder, CheckTx,
//! the `getProofs` query, the DAPI broadcast pre-filter, the client factories) has to know how
//! large the transition may be before it decodes the bytes. Ordinary families share one cap,
//! `SystemLimits::max_state_transition_size`; the contract-code capable generations of the
//! contract create and update transitions carry code bundles and get a larger cap of their
//! own. The family is read from the first bytes of the payload, so the choice costs nothing
//! and never allocates.

use crate::consensus::basic::state_transition::StateTransitionNotActiveError;
use crate::consensus::basic::BasicError;
use crate::consensus::ConsensusError;
use crate::serialization::PlatformDeserializable;
use crate::state_transition::StateTransition;
use crate::ProtocolError;
use bincode::config::{BigEndian, Configuration, Limit};
use bincode::error::DecodeError;
use platform_version::version::PlatformVersion;
use std::fmt;

/// The bincode decode budget of a contract-code capable envelope
/// (`SystemLimits::max_contract_code_state_transition_decode_budget` on the protocol versions
/// that set it). bincode's limit is a const generic, so the budgets the tables may hold are a
/// fixed set; this is the only bounded budget
/// [`StateTransition::deserialize_from_bytes_with_budget`] accepts.
pub const CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET: u64 = 67_108_864;

/// The bincode budget a serialized state transition is decoded under.
///
/// The wire cap and the decode budget are different numbers: the wire cap is a plain comparison
/// on the raw length before any decode, while bincode charges the allocation claims the `Value`
/// decoder makes for containers (each map and array claims its capacity up front) against the
/// decode budget as well as the encoded bytes. A compact transition can therefore claim many
/// times its wire size while decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateTransitionDecodeBudget {
    /// The decode every ordinary family shipped with: `StateTransition::deserialize_from_bytes`.
    ///
    /// That entry point applies no bincode budget. The `limit = 100000` declared on the enum sits
    /// in a second `platform_serialize` attribute the derive never reads, so the only bound on
    /// an ordinary family is the raw wire cap. Enforcing a budget there now would change which
    /// historical blocks decode, so the shipped decode is kept exactly as it is for every
    /// ordinary family at every protocol version, including the ones that bound contract-code
    /// envelopes.
    Historical,
    /// An explicit bincode budget read from the version tables.
    Bounded(u64),
}

/// The bincode variant index of `StateTransition::DataContractCreate`.
const DATA_CONTRACT_CREATE_VARIANT_INDEX: u32 = 0;
/// The bincode variant index of `StateTransition::DataContractUpdate`.
const DATA_CONTRACT_UPDATE_VARIANT_INDEX: u32 = 1;
/// The bincode variant index of the contract-code capable generation inside both contract
/// transition enums (`DataContractCreateTransition::V1`, `DataContractUpdateTransition::V1`).
const CONTRACT_CODE_CAPABLE_GENERATION_INDEX: u32 = 1;

/// The longest prefix [`StateTransition::peek_envelope_kind`] reads: two bincode `u32`
/// varints of at most five bytes each.
const ENVELOPE_PREFIX_MAX_LEN: usize = 10;

/// The two contract transition families that can carry a code bundle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContractCodeFamily {
    /// `StateTransition::DataContractCreate` in a contract-code capable generation.
    DataContractCreate,
    /// `StateTransition::DataContractUpdate` in a contract-code capable generation.
    DataContractUpdate,
}

impl fmt::Display for ContractCodeFamily {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContractCodeFamily::DataContractCreate => f.write_str("DataContractCreate"),
            ContractCodeFamily::DataContractUpdate => f.write_str("DataContractUpdate"),
        }
    }
}

/// What the wire prefix of a serialized state transition says about its size class.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateTransitionEnvelopeKind {
    /// Every family bounded by `SystemLimits::max_state_transition_size`, including the
    /// contract transitions in their original generation, and anything the prefix does not
    /// identify (unknown or truncated discriminants fail decode as they always did).
    Ordinary,
    /// A contract create or update transition in a generation that can carry a code bundle,
    /// bounded by `SystemLimits::max_contract_code_state_transition_size` where the protocol
    /// version sets it.
    ContractCodeCapable {
        /// Which of the two contract families the prefix names.
        family: ContractCodeFamily,
    },
}

impl StateTransition {
    /// Reads the family of a serialized state transition from its wire prefix without decoding
    /// the payload.
    ///
    /// The outer `StateTransition` enum and the inner transition enums are bincode enums, so
    /// the payload starts with the outer variant index followed by the inner variant index,
    /// both big-endian `u32` varints. A prefix naming `DataContractCreate` or
    /// `DataContractUpdate` in the contract-code capable generation is
    /// [`StateTransitionEnvelopeKind::ContractCodeCapable`]; every other prefix, including an
    /// unknown index or fewer bytes than a discriminant needs, is
    /// [`StateTransitionEnvelopeKind::Ordinary`], so it is bounded by the ordinary cap and
    /// fails decode exactly as it does today. At most ten bytes are read and nothing is
    /// allocated. The discriminants are pinned by the classification test over real
    /// serialized fixtures, so a reordering of either enum fails a test instead of silently
    /// moving the cap.
    pub fn peek_envelope_kind(bytes: &[u8]) -> StateTransitionEnvelopeKind {
        let prefix = &bytes[..bytes.len().min(ENVELOPE_PREFIX_MAX_LEN)];
        let config = bincode::config::standard().with_big_endian();
        let Ok((outer_index, consumed)) = bincode::decode_from_slice::<u32, _>(prefix, config)
        else {
            return StateTransitionEnvelopeKind::Ordinary;
        };
        let family = match outer_index {
            DATA_CONTRACT_CREATE_VARIANT_INDEX => ContractCodeFamily::DataContractCreate,
            DATA_CONTRACT_UPDATE_VARIANT_INDEX => ContractCodeFamily::DataContractUpdate,
            _ => return StateTransitionEnvelopeKind::Ordinary,
        };
        let Ok((inner_index, _)) =
            bincode::decode_from_slice::<u32, _>(&prefix[consumed..], config)
        else {
            return StateTransitionEnvelopeKind::Ordinary;
        };
        if inner_index == CONTRACT_CODE_CAPABLE_GENERATION_INDEX {
            StateTransitionEnvelopeKind::ContractCodeCapable { family }
        } else {
            StateTransitionEnvelopeKind::Ordinary
        }
    }

    /// The raw size cap that applies to a serialized state transition of the given kind under
    /// the given protocol version: `max_state_transition_size` unless the kind is contract-code
    /// capable and the version sets `max_contract_code_state_transition_size`.
    pub fn family_max_size(
        kind: StateTransitionEnvelopeKind,
        platform_version: &PlatformVersion,
    ) -> u64 {
        let limits = &platform_version.system_limits;
        match kind {
            StateTransitionEnvelopeKind::Ordinary => limits.max_state_transition_size,
            StateTransitionEnvelopeKind::ContractCodeCapable { .. } => limits
                .max_contract_code_state_transition_size
                .unwrap_or(limits.max_state_transition_size),
        }
    }

    /// The bincode decode budget that applies to a serialized state transition of the given
    /// kind under the given protocol version: [`StateTransitionDecodeBudget::Historical`]
    /// unless the kind is contract-code capable and the version sets
    /// `max_contract_code_state_transition_decode_budget`.
    pub fn family_decode_budget(
        kind: StateTransitionEnvelopeKind,
        platform_version: &PlatformVersion,
    ) -> StateTransitionDecodeBudget {
        match kind {
            StateTransitionEnvelopeKind::Ordinary => StateTransitionDecodeBudget::Historical,
            StateTransitionEnvelopeKind::ContractCodeCapable { .. } => platform_version
                .system_limits
                .max_contract_code_state_transition_decode_budget
                .map_or(
                    StateTransitionDecodeBudget::Historical,
                    StateTransitionDecodeBudget::Bounded,
                ),
        }
    }

    /// Decodes a state transition under the given budget.
    ///
    /// [`StateTransitionDecodeBudget::Historical`] is the shipped `deserialize_from_bytes`.
    /// bincode's limit is a const generic, so only the bounded budgets the version tables can
    /// hold are supported: [`CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET`] today. Any other
    /// bounded value is `ProtocolError::CorruptedCodeExecution`: unreachable while the tables
    /// hold that number, and pinned by a test over every platform version.
    ///
    /// The error mapping of the bounded decode is the one the derive uses where a limit is
    /// applied: a limit hit is `MaxEncodedBytesReachedError`, anything else
    /// `PlatformDeserializationError`.
    pub fn deserialize_from_bytes_with_budget(
        bytes: &[u8],
        budget: StateTransitionDecodeBudget,
    ) -> Result<Self, ProtocolError> {
        match budget {
            StateTransitionDecodeBudget::Historical => Self::deserialize_from_bytes(bytes),
            StateTransitionDecodeBudget::Bounded(CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET) => {
                const BUDGET: usize = CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET as usize;
                let config: Configuration<BigEndian, _, Limit<BUDGET>> =
                    bincode::config::standard()
                        .with_big_endian()
                        .with_limit::<BUDGET>();
                bincode::decode_from_slice(bytes, config)
                    .map(|(state_transition, _)| state_transition)
                    .map_err(|error| match error {
                        DecodeError::Io { .. } | DecodeError::LimitExceeded => {
                            ProtocolError::MaxEncodedBytesReachedError {
                                max_size_kbytes: BUDGET,
                                size_hit: bytes.len(),
                            }
                        }
                        other => ProtocolError::PlatformDeserializationError(format!(
                            "unable to deserialize StateTransition: {other}"
                        )),
                    })
            }
            StateTransitionDecodeBudget::Bounded(other) => {
                Err(ProtocolError::CorruptedCodeExecution(format!(
                    "state transition decode budget {other} is not one of the supported budgets"
                )))
            }
        }
    }

    /// Decodes a state transition under the budget of the family its wire prefix names, with
    /// the value depth limit of the protocol version, and checks that the decoded variant is
    /// active in that version.
    ///
    /// This is the entry point of the code paths that admit the larger contract-code envelopes
    /// (`decode_raw_state_transitions` v1, `getProofs` v1, the client factories). Two things
    /// differ from `deserialize_from_bytes_in_version`, which every older path keeps calling
    /// unchanged:
    ///
    /// * the budget is [`StateTransition::family_decode_budget`] of the peeked kind, so a
    ///   contract-code envelope decodes under the bounded budget of the tables and every other
    ///   family decodes exactly as before;
    /// * a variant outside its `active_version_range()` is reported as the consensus error it
    ///   is, `BasicError::StateTransitionNotActiveError` wrapped in
    ///   `ProtocolError::ConsensusError`, rather than as `ProtocolError::StateTransitionError`,
    ///   so a caller classifies it as an unpaid rejection instead of a node fault.
    ///
    /// The raw size cap is not checked here: callers compare the length with
    /// [`StateTransition::family_max_size`] before decoding, where the rejection is cheapest.
    pub fn deserialize_from_bytes_in_version_bounded(
        bytes: &[u8],
        platform_version: &PlatformVersion,
    ) -> Result<Self, ProtocolError> {
        let kind = Self::peek_envelope_kind(bytes);
        let budget = Self::family_decode_budget(kind, platform_version);
        let max_value_depth = platform_version
            .system_limits
            .max_document_value_depth
            .map(usize::from);
        let state_transition =
            platform_value::with_value_decode_depth_limit(max_value_depth, || {
                Self::deserialize_from_bytes_with_budget(bytes, budget)
            })?;
        let active_version_range = state_transition.active_version_range();
        if active_version_range.contains(&platform_version.protocol_version) {
            Ok(state_transition)
        } else {
            Err(ProtocolError::ConsensusError(Box::new(
                ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(
                    StateTransitionNotActiveError::new(
                        state_transition.name(),
                        platform_version.protocol_version,
                        *active_version_range.start(),
                    ),
                )),
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data_contract::config::v0::DataContractConfigV0;
    use crate::data_contract::config::DataContractConfig;
    use crate::data_contract::serialized_version::DataContractInSerializationFormat;
    use crate::data_contract::v1::DataContractV1;
    use crate::data_contract::DataContract;
    use crate::identity::core_script::CoreScript;
    use crate::serialization::PlatformSerializable;
    use crate::state_transition::batch_transition::batched_transition::document_base_transition::v0::DocumentBaseTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_base_transition::DocumentBaseTransition;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::v0::DocumentDeleteTransitionV0;
    use crate::state_transition::batch_transition::batched_transition::document_delete_transition::DocumentDeleteTransition;
    use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransition;
    use crate::state_transition::batch_transition::batched_transition::BatchedTransition;
    use crate::state_transition::batch_transition::BatchTransitionV1;
    use crate::state_transition::batch_transition::BatchTransition;
    use crate::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
    use crate::state_transition::data_contract_create_transition::DataContractCreateTransitionV0;
    use crate::state_transition::data_contract_create_transition::DataContractCreateTransition;
    use crate::state_transition::data_contract_update_transition::DataContractUpdateTransitionV0;
    use crate::state_transition::data_contract_update_transition::DataContractUpdateTransition;
    use crate::state_transition::identity_credit_transfer_transition::v0::IdentityCreditTransferTransitionV0;
    use crate::state_transition::identity_credit_transfer_transition::IdentityCreditTransferTransition;
    use crate::state_transition::identity_credit_withdrawal_transition::v0::IdentityCreditWithdrawalTransitionV0;
    use crate::state_transition::identity_credit_withdrawal_transition::IdentityCreditWithdrawalTransition;
    use crate::state_transition::masternode_vote_transition::v0::MasternodeVoteTransitionV0;
    use crate::state_transition::masternode_vote_transition::MasternodeVoteTransition;
    use crate::withdrawal::Pooling;
    use platform_value::{BinaryData, Identifier, Value};
    use platform_version::version::PLATFORM_VERSIONS;
    use std::collections::BTreeMap;

    fn contract_with_document_schemas(
        document_schemas: BTreeMap<String, Value>,
    ) -> DataContractInSerializationFormat {
        use platform_version::TryIntoPlatformVersioned;

        let contract = DataContract::V1(DataContractV1 {
            id: Identifier::from([9u8; 32]),
            version: 1,
            owner_id: Identifier::from([7u8; 32]),
            document_types: BTreeMap::new(),
            config: DataContractConfig::V0(DataContractConfigV0 {
                can_be_deleted: false,
                readonly: false,
                keeps_history: false,
                documents_keep_history_contract_default: false,
                documents_mutable_contract_default: false,
                documents_can_be_deleted_contract_default: false,
                requires_identity_encryption_bounded_key: None,
                requires_identity_decryption_bounded_key: None,
            }),
            schema_defs: None,
            created_at: None,
            updated_at: None,
            created_at_block_height: None,
            updated_at_block_height: None,
            created_at_epoch: None,
            updated_at_epoch: None,
            groups: BTreeMap::new(),
            tokens: BTreeMap::new(),
            keywords: Vec::new(),
            description: None,
        });
        let mut format: DataContractInSerializationFormat = contract
            .try_into_platform_versioned(PlatformVersion::latest())
            .expect("expected to serialize a trivial contract");
        if let DataContractInSerializationFormat::V1(v1) = &mut format {
            v1.document_schemas = document_schemas;
        }
        format
    }

    fn create_v0(data_contract: DataContractInSerializationFormat) -> StateTransition {
        StateTransition::DataContractCreate(DataContractCreateTransition::V0(
            DataContractCreateTransitionV0 {
                data_contract,
                identity_nonce: 1,
                user_fee_increase: 5,
                signature_public_key_id: 2,
                signature: BinaryData::new(vec![0xAB; 65]),
            },
        ))
    }

    fn update_v0(data_contract: DataContractInSerializationFormat) -> StateTransition {
        StateTransition::DataContractUpdate(DataContractUpdateTransition::V0(
            DataContractUpdateTransitionV0 {
                identity_contract_nonce: 4,
                data_contract,
                user_fee_increase: 9,
                signature_public_key_id: 6,
                signature: BinaryData::new(vec![0xCD; 65]),
            },
        ))
    }

    fn batch_with_delete() -> StateTransition {
        let base = DocumentBaseTransition::V0(DocumentBaseTransitionV0 {
            id: Identifier::from([1u8; 32]),
            identity_contract_nonce: 3,
            document_type_name: "preorder".to_string(),
            data_contract_id: Identifier::from([2u8; 32]),
        });
        let delete = DocumentDeleteTransition::V0(DocumentDeleteTransitionV0 { base });
        StateTransition::Batch(BatchTransition::V1(BatchTransitionV1 {
            owner_id: Identifier::from([3u8; 32]),
            transitions: vec![BatchedTransition::Document(DocumentTransition::Delete(
                delete,
            ))],
            user_fee_increase: 0,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![0u8; 65]),
        }))
    }

    fn credit_transfer() -> StateTransition {
        StateTransition::IdentityCreditTransfer(IdentityCreditTransferTransition::V0(
            IdentityCreditTransferTransitionV0 {
                identity_id: Identifier::from([1u8; 32]),
                recipient_id: Identifier::from([2u8; 32]),
                amount: 1_000,
                nonce: 7,
                user_fee_increase: 3,
                signature_public_key_id: 11,
                signature: BinaryData::new(vec![0u8; 65]),
            },
        ))
    }

    fn credit_withdrawal() -> StateTransition {
        StateTransition::IdentityCreditWithdrawal(IdentityCreditWithdrawalTransition::V0(
            IdentityCreditWithdrawalTransitionV0 {
                identity_id: Identifier::from([4u8; 32]),
                amount: 500,
                core_fee_per_byte: 1,
                pooling: Pooling::Never,
                output_script: CoreScript::from_bytes(vec![0x76, 0xa9]),
                nonce: 2,
                user_fee_increase: 0,
                signature_public_key_id: 1,
                signature: BinaryData::new(vec![0u8; 65]),
            },
        ))
    }

    fn masternode_vote() -> StateTransition {
        StateTransition::MasternodeVote(MasternodeVoteTransition::V0(MasternodeVoteTransitionV0 {
            pro_tx_hash: Identifier::from([5u8; 32]),
            voter_identity_id: Identifier::from([6u8; 32]),
            vote: Default::default(),
            nonce: 1,
            signature_public_key_id: 0,
            signature: BinaryData::new(vec![0u8; 96]),
        }))
    }

    /// The prefix decode reads discriminants, not types, so the only way to keep it honest is
    /// to serialize real transitions and check where each lands. Every fixture here is a
    /// generation without a code bundle, so every one is `Ordinary`; the contract-code capable
    /// generations, once they exist, are added here asserting `ContractCodeCapable`.
    #[test]
    fn should_classify_every_serialized_fixture_as_ordinary() {
        let fixtures: Vec<(&str, StateTransition)> = vec![
            (
                "contract create v0",
                create_v0(contract_with_document_schemas(BTreeMap::new())),
            ),
            (
                "contract update v0",
                update_v0(contract_with_document_schemas(BTreeMap::new())),
            ),
            ("batch", batch_with_delete()),
            ("credit transfer", credit_transfer()),
            ("credit withdrawal", credit_withdrawal()),
            ("masternode vote", masternode_vote()),
        ];
        for (name, fixture) in fixtures {
            let bytes = fixture.serialize_to_bytes().expect("serialize fixture");
            assert_eq!(
                StateTransition::peek_envelope_kind(&bytes),
                StateTransitionEnvelopeKind::Ordinary,
                "{name} carries no code bundle and must be bounded like every other family"
            );
        }
    }

    /// The contract families are the outer indices 0 and 1 and the contract-code capable
    /// generation is inner index 1. Those indices are pinned against the serialized V0 forms:
    /// the outer index is what the real transitions start with, and flipping the inner byte
    /// from the V0 index to the next one is exactly what a V1 variant of the same enum encodes
    /// as. A reordering of either enum changes what these bytes mean and fails here.
    #[test]
    fn should_recognise_the_contract_code_capable_prefix_of_both_contract_families() {
        let create_bytes = create_v0(contract_with_document_schemas(BTreeMap::new()))
            .serialize_to_bytes()
            .expect("serialize create");
        let update_bytes = update_v0(contract_with_document_schemas(BTreeMap::new()))
            .serialize_to_bytes()
            .expect("serialize update");
        assert_eq!(create_bytes[0], DATA_CONTRACT_CREATE_VARIANT_INDEX as u8);
        assert_eq!(update_bytes[0], DATA_CONTRACT_UPDATE_VARIANT_INDEX as u8);
        assert_eq!(create_bytes[1], 0, "the V0 generation is inner index 0");
        assert_eq!(update_bytes[1], 0, "the V0 generation is inner index 0");

        let mut create_v1_prefix = create_bytes.clone();
        create_v1_prefix[1] = CONTRACT_CODE_CAPABLE_GENERATION_INDEX as u8;
        assert_eq!(
            StateTransition::peek_envelope_kind(&create_v1_prefix),
            StateTransitionEnvelopeKind::ContractCodeCapable {
                family: ContractCodeFamily::DataContractCreate
            }
        );
        let mut update_v1_prefix = update_bytes.clone();
        update_v1_prefix[1] = CONTRACT_CODE_CAPABLE_GENERATION_INDEX as u8;
        assert_eq!(
            StateTransition::peek_envelope_kind(&update_v1_prefix),
            StateTransitionEnvelopeKind::ContractCodeCapable {
                family: ContractCodeFamily::DataContractUpdate
            }
        );

        // Only the two bytes matter: the same prefix on its own classifies identically.
        assert_eq!(
            StateTransition::peek_envelope_kind(&[0, 1]),
            StateTransitionEnvelopeKind::ContractCodeCapable {
                family: ContractCodeFamily::DataContractCreate
            }
        );
        assert_eq!(
            StateTransition::peek_envelope_kind(&[1, 1]),
            StateTransitionEnvelopeKind::ContractCodeCapable {
                family: ContractCodeFamily::DataContractUpdate
            }
        );
        // bincode accepts a non-canonical varint for a discriminant, so the full decoder would
        // read a two-byte encoding of index 0 as `DataContractCreate` too; the peek agrees with
        // the decoder rather than second-guessing it, which keeps the cap consistent with what
        // the bytes decode into.
        assert_eq!(
            StateTransition::peek_envelope_kind(&[251, 0, 0, 1]),
            StateTransitionEnvelopeKind::ContractCodeCapable {
                family: ContractCodeFamily::DataContractCreate
            }
        );
    }

    #[test]
    fn should_treat_truncated_and_unknown_prefixes_as_ordinary() {
        let cases: Vec<(&str, Vec<u8>)> = vec![
            ("empty", vec![]),
            ("outer index only", vec![0]),
            ("outer update index only", vec![1]),
            ("unknown outer index", vec![200, 1]),
            ("batch outer index with inner 1", vec![2, 1]),
            ("contract create with a later inner generation", vec![0, 2]),
            ("multi-byte unknown outer index", vec![251, 0, 200, 1]),
            ("reserved varint discriminant", vec![255, 1]),
            ("truncated multi-byte inner index", vec![0, 251]),
        ];
        for (name, bytes) in cases {
            assert_eq!(
                StateTransition::peek_envelope_kind(&bytes),
                StateTransitionEnvelopeKind::Ordinary,
                "{name} must fall back to the ordinary cap"
            );
        }
    }

    #[test]
    fn should_pick_the_cap_and_budget_from_the_kind_and_the_version() {
        let latest = PlatformVersion::latest();
        let pre_contract_code = PlatformVersion::get(14).expect("protocol version 14");
        let contract_code_kind = StateTransitionEnvelopeKind::ContractCodeCapable {
            family: ContractCodeFamily::DataContractCreate,
        };

        assert_eq!(
            StateTransition::family_max_size(StateTransitionEnvelopeKind::Ordinary, latest),
            latest.system_limits.max_state_transition_size
        );
        assert_eq!(
            StateTransition::family_max_size(contract_code_kind, latest),
            latest
                .system_limits
                .max_contract_code_state_transition_size
                .expect("the latest version bounds contract code envelopes")
        );
        assert!(
            StateTransition::family_max_size(contract_code_kind, latest)
                > StateTransition::family_max_size(StateTransitionEnvelopeKind::Ordinary, latest)
        );
        // A version without the contract-code limits bounds the kind like any other family.
        assert_eq!(
            StateTransition::family_max_size(contract_code_kind, pre_contract_code),
            pre_contract_code.system_limits.max_state_transition_size
        );

        assert_eq!(
            StateTransition::family_decode_budget(StateTransitionEnvelopeKind::Ordinary, latest),
            StateTransitionDecodeBudget::Historical
        );
        assert_eq!(
            StateTransition::family_decode_budget(contract_code_kind, latest),
            StateTransitionDecodeBudget::Bounded(CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET)
        );
        assert_eq!(
            StateTransition::family_decode_budget(contract_code_kind, pre_contract_code),
            StateTransitionDecodeBudget::Historical
        );
    }

    /// The budgets are const generics, so the tables can only hold the values the method
    /// matches on. If a table ever holds another number the v1 decoder would report a node
    /// fault on every block, so every registered version is checked here.
    #[test]
    fn should_decode_with_each_supported_budget_and_every_table_budget_is_supported() {
        let fixture = credit_transfer();
        let bytes = fixture.serialize_to_bytes().expect("serialize fixture");
        for budget in [
            StateTransitionDecodeBudget::Historical,
            StateTransitionDecodeBudget::Bounded(CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET),
        ] {
            let decoded = StateTransition::deserialize_from_bytes_with_budget(&bytes, budget)
                .expect("decode under a supported budget");
            assert_eq!(decoded, fixture);
        }
        for platform_version in PLATFORM_VERSIONS {
            if let Some(budget) = platform_version
                .system_limits
                .max_contract_code_state_transition_decode_budget
            {
                assert!(
                    StateTransition::deserialize_from_bytes_with_budget(
                        &bytes,
                        StateTransitionDecodeBudget::Bounded(budget)
                    )
                    .is_ok(),
                    "protocol version {} holds a decode budget the decoder does not support",
                    platform_version.protocol_version
                );
            }
        }
    }

    #[test]
    fn should_reject_a_budget_outside_the_supported_set() {
        let bytes = credit_transfer()
            .serialize_to_bytes()
            .expect("serialize fixture");
        let error = StateTransition::deserialize_from_bytes_with_budget(
            &bytes,
            StateTransitionDecodeBudget::Bounded(12_345),
        )
        .expect_err("an unsupported budget is a node fault");
        assert!(matches!(error, ProtocolError::CorruptedCodeExecution(_)));
    }

    /// A credit transfer whose trailing signature `Vec<u8>` really carries `signature_len`
    /// bytes: a structurally valid payload of any size.
    fn credit_transfer_with_signature_len(signature_len: usize) -> Vec<u8> {
        let mut payload = credit_transfer()
            .serialize_to_bytes()
            .expect("serialize fixture");
        payload.truncate(payload.len() - 66);
        let config = bincode::config::standard().with_big_endian();
        payload.extend(bincode::encode_to_vec(signature_len as u64, config).expect("encode"));
        payload.extend(std::iter::repeat_n(0xEEu8, signature_len));
        payload
    }

    /// Under the bounded budget a crafted length claim is rejected by the limit before anything
    /// is allocated: the larger budget widens what a contract envelope may claim, it does not
    /// remove the bound. The historical decode has no such bound (the claim is only caught when
    /// the reader runs out of bytes), which is why the contract-code envelopes, the first family
    /// large enough to make that matter, are decoded under an explicit budget.
    #[test]
    fn should_reject_a_crafted_length_claim_by_the_limit_under_the_bounded_budget() {
        let mut payload = credit_transfer()
            .serialize_to_bytes()
            .expect("serialize fixture");
        // The signature is the trailing `Vec<u8>`; replace its length prefix with a claim just
        // above the budget and drop the bytes.
        payload.truncate(payload.len() - 66);
        let config = bincode::config::standard().with_big_endian();
        payload.extend(
            bincode::encode_to_vec(CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET + 1, config)
                .expect("encode"),
        );
        let error = StateTransition::deserialize_from_bytes_with_budget(
            &payload,
            StateTransitionDecodeBudget::Bounded(CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET),
        )
        .expect_err("a claim above the budget must be rejected");
        assert!(
            matches!(error, ProtocolError::MaxEncodedBytesReachedError { .. }),
            "expected the limit to fire, got {error:?}"
        );
        let historical_error = StateTransition::deserialize_from_bytes_with_budget(
            &payload,
            StateTransitionDecodeBudget::Historical,
        )
        .expect_err("the bytes are missing either way");
        assert!(
            matches!(
                historical_error,
                ProtocolError::PlatformDeserializationError(_)
            ),
            "the historical decode has no limit to fire, got {historical_error:?}"
        );
    }

    /// The bounded budget admits a real payload well above anything an ordinary family
    /// carries, so a contract-code envelope of legitimate size decodes; the historical decode
    /// admits it as well, which documents that the ordinary families are bounded by the wire
    /// cap alone.
    #[test]
    fn should_decode_a_large_real_payload_under_both_budgets() {
        let payload = credit_transfer_with_signature_len(150_000);
        for budget in [
            StateTransitionDecodeBudget::Historical,
            StateTransitionDecodeBudget::Bounded(CONTRACT_CODE_STATE_TRANSITION_DECODE_BUDGET),
        ] {
            let decoded = StateTransition::deserialize_from_bytes_with_budget(&payload, budget)
                .expect("150 KB is inside both");
            assert!(matches!(
                decoded,
                StateTransition::IdentityCreditTransfer(_)
            ));
        }
    }

    /// The wire cap and the decode budget are different numbers. A compact transition well
    /// under 20 KiB on the wire whose schema decodes into many small containers claims more
    /// than 20,480 against a bincode budget (each map and array claims its capacity) and must
    /// keep decoding through both the historical entry point and the bounded one, on both
    /// sides of the contract-code gate: the bounded path must never bound an ordinary family
    /// tighter than the shipped decode does.
    #[test]
    fn should_keep_decoding_a_compact_wide_container_on_the_bounded_path() {
        // 40 document types with one empty map property each: every map claims its capacity.
        let document_schemas: BTreeMap<String, Value> = (0..40)
            .map(|i| {
                let properties: Vec<(Value, Value)> = (0..24)
                    .map(|j| {
                        (
                            Value::Text(format!("p{j}")),
                            Value::Map(vec![(Value::Text("t".to_string()), Value::Map(vec![]))]),
                        )
                    })
                    .collect();
                (
                    format!("type{i}"),
                    Value::Map(vec![
                        (
                            Value::Text("type".to_string()),
                            Value::Text("object".to_string()),
                        ),
                        (
                            Value::Text("properties".to_string()),
                            Value::Map(properties),
                        ),
                    ]),
                )
            })
            .collect();
        let fixture = create_v0(contract_with_document_schemas(document_schemas));
        let bytes = fixture.serialize_to_bytes().expect("serialize fixture");
        let latest = PlatformVersion::latest();
        let pre_contract_code = PlatformVersion::get(14).expect("protocol version 14");
        assert!(
            (bytes.len() as u64) < latest.system_limits.max_state_transition_size,
            "the fixture must stay under the ordinary wire cap, got {} bytes",
            bytes.len()
        );
        // Prove the claims exceed the wire cap: a budget equal to the wire cap fails.
        const WIRE_CAP: usize = 20_480;
        let tight = bincode::config::standard()
            .with_big_endian()
            .with_limit::<WIRE_CAP>();
        assert!(
            bincode::decode_from_slice::<StateTransition, _>(&bytes, tight).is_err(),
            "the fixture must claim more than the wire cap for this test to mean anything"
        );

        for platform_version in [latest, pre_contract_code] {
            let historical =
                StateTransition::deserialize_from_bytes_in_version(&bytes, platform_version)
                    .expect("the historical entry point decodes it");
            let bounded = StateTransition::deserialize_from_bytes_in_version_bounded(
                &bytes,
                platform_version,
            )
            .expect("the bounded entry point decodes it");
            assert_eq!(historical, fixture);
            assert_eq!(bounded, fixture);
        }
    }

    /// The bounded entry point reports a not-active variant as the consensus error it is; the
    /// untouched entry point keeps the legacy protocol error the frozen v0 decoder maps to a
    /// node fault.
    #[test]
    fn should_report_not_active_as_a_consensus_error_only_on_the_bounded_path() {
        // A format-V1 contract is active from protocol version 9.
        let fixture = create_v0(contract_with_document_schemas(BTreeMap::new()));
        assert!(matches!(
            &fixture,
            StateTransition::DataContractCreate(create)
                if matches!(create.data_contract(), DataContractInSerializationFormat::V1(_))
        ));
        let bytes = fixture.serialize_to_bytes().expect("serialize fixture");
        let too_early = PlatformVersion::get(8).expect("protocol version 8");

        let bounded_error =
            StateTransition::deserialize_from_bytes_in_version_bounded(&bytes, too_early)
                .expect_err("a format-V1 contract is not active at 8");
        match bounded_error {
            ProtocolError::ConsensusError(consensus_error) => match *consensus_error {
                ConsensusError::BasicError(BasicError::StateTransitionNotActiveError(error)) => {
                    assert_eq!(error.state_transition_type(), "DataContractCreate");
                    assert_eq!(error.current_protocol_version(), 8);
                    assert_eq!(error.required_protocol_version(), 9);
                }
                other => panic!("expected StateTransitionNotActiveError, got {other:?}"),
            },
            other => panic!("expected a consensus error, got {other:?}"),
        }

        let legacy_error = StateTransition::deserialize_from_bytes_in_version(&bytes, too_early)
            .expect_err("a format-V1 contract is not active at 8");
        assert!(
            matches!(legacy_error, ProtocolError::StateTransitionError(_)),
            "the untouched entry point keeps the legacy error, got {legacy_error:?}"
        );

        // At a version that admits it, both paths agree.
        let active = PlatformVersion::get(9).expect("protocol version 9");
        assert_eq!(
            StateTransition::deserialize_from_bytes_in_version_bounded(&bytes, active)
                .expect("active at 9"),
            fixture
        );
    }

    /// Reproduces the decode-side half of the size benchmark recorded in the pull request: a
    /// synthetic contract-code envelope at the family cap is rejected by the prefix peek and
    /// the budget check in microseconds, never allocated. Run with `--ignored --nocapture` to
    /// print the timings.
    #[test]
    #[ignore]
    fn measure_decode_gate_at_the_family_cap() {
        use std::time::Instant;

        let latest = PlatformVersion::latest();
        let cap = latest
            .system_limits
            .max_contract_code_state_transition_size
            .expect("the latest version bounds contract code envelopes") as usize;
        for size in [1 << 20, 8 << 20, 16 << 20, cap] {
            let mut envelope = vec![0u8; size];
            envelope[1] = CONTRACT_CODE_CAPABLE_GENERATION_INDEX as u8;
            let started = Instant::now();
            let kind = StateTransition::peek_envelope_kind(&envelope);
            let peek = started.elapsed();
            let started = Instant::now();
            let result =
                StateTransition::deserialize_from_bytes_in_version_bounded(&envelope, latest);
            let decode = started.elapsed();
            println!(
                "{size} bytes: kind {kind:?} in {peek:?}, decode attempt {decode:?}, outcome {}",
                if result.is_ok() {
                    "decoded"
                } else {
                    "rejected"
                }
            );
        }
    }
}
