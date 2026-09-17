use crate::contract_group::ContractGroupMember;
use crate::identifier::Identifier;
use crate::identity::identity_public_key::contract_bounds::ContractBounds::{
    ContractGroup, SingleContract, SingleContractDocumentType,
};
#[cfg(feature = "json-conversion")]
use crate::serialization::JsonConvertible;
#[cfg(feature = "value-conversion")]
use crate::serialization::ValueConvertible;
#[cfg(feature = "state-transitions")]
use crate::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
#[cfg(feature = "state-transitions")]
use crate::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
#[cfg(feature = "state-transitions")]
use crate::state_transition::batch_transition::batched_transition::BatchedTransitionRef;
#[cfg(feature = "state-transitions")]
use crate::state_transition::batch_transition::token_base_transition::v0::v0_methods::TokenBaseTransitionV0Methods;
use crate::ProtocolError;
use bincode::{Decode, DecodeUntrusted, Encode};
use serde::{Deserialize, Serialize};

pub type ContractBoundsType = u8;

/// A contract bounds is the bounds that the key has influence on.
/// For authentication keys the bounds mean that the keys can only be used to sign
/// within the specified contract, or within the members of the specified contract group.
/// For encryption decryption this tells clients to only use these keys for specific
/// contracts.
///
/// New variants go last: the bincode tag is the declaration index, and every stored identity
/// key carries one.
#[cfg_attr(feature = "json-conversion", derive(JsonConvertible))]
#[repr(u8)]
#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Serialize,
    Deserialize,
    Encode,
    Decode,
    Ord,
    PartialOrd,
    Hash,
    DecodeUntrusted,
)]
#[cfg_attr(feature = "value-conversion", derive(ValueConvertible))]
#[serde(tag = "$type", rename_all = "camelCase")]
pub enum ContractBounds {
    /// this key can only be used within a specific contract
    #[serde(rename = "singleContract")]
    SingleContract { id: Identifier } = 0,
    /// this key can only be used within a specific contract and for a specific document type
    #[serde(rename = "documentType", rename_all = "camelCase")]
    SingleContractDocumentType {
        id: Identifier,
        document_type_name: String,
    } = 1,
    /// this key can only be used within the members of a contract group: the contracts,
    /// document types and tokens the group holds when the key signs (protocol version 14)
    #[serde(rename = "contractGroup")]
    ContractGroup { id: Identifier } = 2,
}

/// What authorizing one batch member with a contract-bound AUTHENTICATION key needs.
#[cfg(feature = "state-transitions")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchedTransitionBoundsCheck {
    /// The member lies inside the bounds.
    Allowed,
    /// The member lies outside the bounds.
    Denied,
    /// The bounds name a contract group. The member lies inside them when its contract holds
    /// this membership in the group, or is a whole-contract member of the group. Only state can
    /// answer that; consensus reads the contract's memberships and bills the read.
    RequiresContractGroupMembership {
        contract_group_id: Identifier,
        contract_id: Identifier,
        member: ContractGroupMember,
    },
}

impl ContractBounds {
    /// Creates a new contract bounds for the key
    pub fn new_from_type(
        contract_bounds_type: u8,
        identifier: Vec<u8>,
        document_type: String,
    ) -> Result<Self, ProtocolError> {
        Ok(match contract_bounds_type {
            0 => SingleContract {
                id: Identifier::from_bytes(identifier.as_slice())?,
            },
            1 => SingleContractDocumentType {
                id: Identifier::from_bytes(identifier.as_slice())?,
                document_type_name: document_type,
            },
            2 => ContractGroup {
                id: Identifier::from_bytes(identifier.as_slice())?,
            },
            _ => {
                return Err(ProtocolError::InvalidKeyContractBoundsError(format!(
                    "unrecognized contract bounds type: {}",
                    contract_bounds_type
                )))
            }
        })
    }

    /// Gets the contract bounds type
    pub fn contract_bounds_type(&self) -> ContractBoundsType {
        match self {
            SingleContract { .. } => 0,
            SingleContractDocumentType { .. } => 1,
            ContractGroup { .. } => 2,
        }
    }

    pub fn contract_bounds_type_from_str(str: &str) -> Result<ContractBoundsType, ProtocolError> {
        match str {
            "singleContract" => Ok(0),
            "documentType" => Ok(1),
            "contractGroup" => Ok(2),
            _ => Err(ProtocolError::DecodingError(String::from(
                "Expected type to be one of singleContract, documentType or contractGroup",
            ))),
        }
    }
    /// Gets the contract bounds type
    pub fn contract_bounds_type_string(&self) -> &str {
        match self {
            SingleContract { .. } => "singleContract",
            SingleContractDocumentType { .. } => "documentType",
            ContractGroup { .. } => "contractGroup",
        }
    }

    /// Gets the identifier: the contract id, or the contract group id for a group bound. Use
    /// [`Self::contract_id`] or [`Self::contract_group_id`] when the kind matters.
    pub fn identifier(&self) -> &Identifier {
        match self {
            SingleContract { id } => id,
            SingleContractDocumentType { id, .. } => id,
            ContractGroup { id } => id,
        }
    }

    /// The bound contract id, when the bounds name one contract.
    pub fn contract_id(&self) -> Option<&Identifier> {
        match self {
            SingleContract { id } | SingleContractDocumentType { id, .. } => Some(id),
            ContractGroup { .. } => None,
        }
    }

    /// The bound contract group id, when the bounds name a contract group.
    pub fn contract_group_id(&self) -> Option<&Identifier> {
        match self {
            SingleContract { .. } | SingleContractDocumentType { .. } => None,
            ContractGroup { id } => Some(id),
        }
    }

    /// Gets the document type
    pub fn document_type(&self) -> Option<&String> {
        match self {
            SingleContract { .. } => None,
            SingleContractDocumentType {
                document_type_name: document_type,
                ..
            } => Some(document_type),
            ContractGroup { .. } => None,
        }
    }

    /// What authorizing `transition` with a key carrying these bounds needs. Consensus uses this
    /// to authorize a batch signed by a contract-bound AUTHENTICATION key. Token operations are
    /// contract-wide, so a document-type bound never covers them. A contract group bound is
    /// answered by state: the member's contract must hold the returned membership, or be a
    /// whole-contract member of the group.
    #[cfg(feature = "state-transitions")]
    pub fn check_batched_transition(
        &self,
        transition: BatchedTransitionRef<'_>,
    ) -> BatchedTransitionBoundsCheck {
        use BatchedTransitionBoundsCheck::{Allowed, Denied, RequiresContractGroupMembership};
        let allowed_if = |inside: bool| if inside { Allowed } else { Denied };
        match (self, transition) {
            (SingleContract { id }, BatchedTransitionRef::Document(document)) => {
                allowed_if(document.data_contract_id() == *id)
            }
            (SingleContract { id }, BatchedTransitionRef::Token(token)) => {
                allowed_if(token.data_contract_id() == *id)
            }
            (
                SingleContractDocumentType {
                    id,
                    document_type_name,
                },
                BatchedTransitionRef::Document(document),
            ) => allowed_if(
                document.data_contract_id() == *id
                    && document.document_type_name() == document_type_name.as_str(),
            ),
            (SingleContractDocumentType { .. }, BatchedTransitionRef::Token(_)) => Denied,
            (ContractGroup { id }, BatchedTransitionRef::Document(document)) => {
                RequiresContractGroupMembership {
                    contract_group_id: *id,
                    contract_id: document.data_contract_id(),
                    member: ContractGroupMember::DocumentType(
                        document.document_type_name().to_string(),
                    ),
                }
            }
            (ContractGroup { id }, BatchedTransitionRef::Token(token)) => {
                RequiresContractGroupMembership {
                    contract_group_id: *id,
                    contract_id: token.data_contract_id(),
                    member: ContractGroupMember::Token(token.base().token_contract_position()),
                }
            }
        }
    }
}

#[cfg(test)]
mod core_tests {
    use super::*;

    // -- new_from_type: valid types --
    #[test]
    fn test_new_from_type_single_contract() {
        let id_bytes = vec![0xAAu8; 32];
        let bounds =
            ContractBounds::new_from_type(0, id_bytes.clone(), "ignored".to_string()).unwrap();
        assert!(matches!(bounds, ContractBounds::SingleContract { .. }));
        assert_eq!(bounds.contract_bounds_type(), 0);
        assert_eq!(bounds.contract_bounds_type_string(), "singleContract");
        assert_eq!(bounds.identifier().as_bytes(), id_bytes.as_slice());
        // document_type is None for SingleContract regardless of what we passed in.
        assert!(bounds.document_type().is_none());
    }

    #[test]
    fn test_new_from_type_single_contract_document_type() {
        let id_bytes = vec![0xBBu8; 32];
        let bounds = ContractBounds::new_from_type(1, id_bytes.clone(), "myDoc".to_string())
            .expect("expected to construct SingleContractDocumentType");
        assert!(matches!(
            bounds,
            ContractBounds::SingleContractDocumentType { .. }
        ));
        assert_eq!(bounds.contract_bounds_type(), 1);
        assert_eq!(bounds.contract_bounds_type_string(), "documentType");
        assert_eq!(bounds.identifier().as_bytes(), id_bytes.as_slice());
        assert_eq!(bounds.document_type().map(String::as_str), Some("myDoc"));
    }

    #[test]
    fn should_build_contract_group_bounds_from_type_two() {
        let id_bytes = vec![0xEEu8; 32];
        let bounds = ContractBounds::new_from_type(2, id_bytes.clone(), "ignored".to_string())
            .expect("expected to construct ContractGroup");
        assert!(matches!(bounds, ContractBounds::ContractGroup { .. }));
        assert_eq!(bounds.contract_bounds_type(), 2);
        assert_eq!(bounds.contract_bounds_type_string(), "contractGroup");
        assert_eq!(bounds.identifier().as_bytes(), id_bytes.as_slice());
        assert_eq!(
            bounds.contract_group_id().map(|id| id.as_slice()),
            Some(id_bytes.as_slice())
        );
        assert!(bounds.contract_id().is_none());
        assert!(bounds.document_type().is_none());
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str("contractGroup").unwrap(),
            2
        );
    }

    #[test]
    fn should_expose_the_contract_id_only_for_contract_bounds() {
        let id = Identifier::from([0x12u8; 32]);
        let single = ContractBounds::SingleContract { id };
        let typed = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "note".to_string(),
        };
        assert_eq!(single.contract_id(), Some(&id));
        assert_eq!(typed.contract_id(), Some(&id));
        assert!(single.contract_group_id().is_none());
        assert!(typed.contract_group_id().is_none());
    }

    // -- new_from_type: invalid type --
    #[test]
    fn test_new_from_type_unrecognized_type_returns_error() {
        let id_bytes = vec![0xCCu8; 32];
        let err = ContractBounds::new_from_type(99, id_bytes, "".to_string()).unwrap_err();
        match err {
            ProtocolError::InvalidKeyContractBoundsError(msg) => {
                assert!(msg.contains("99"), "expected error message to mention 99");
            }
            other => panic!("expected InvalidKeyContractBoundsError, got {:?}", other),
        }
    }

    // -- new_from_type: identifier wrong length --
    #[test]
    fn test_new_from_type_invalid_identifier_length_returns_error() {
        // Identifier::from_bytes requires exactly 32 bytes.
        let short = vec![0x01u8; 10];
        assert!(ContractBounds::new_from_type(0, short, "".to_string()).is_err());
    }

    // -- contract_bounds_type_from_str --
    #[test]
    fn test_contract_bounds_type_from_str_single_contract() {
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str("singleContract").unwrap(),
            0
        );
    }

    #[test]
    fn test_contract_bounds_type_from_str_document_type() {
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str("documentType").unwrap(),
            1
        );
    }

    #[test]
    fn test_contract_bounds_type_from_str_unknown_returns_error() {
        let err = ContractBounds::contract_bounds_type_from_str("garbage").unwrap_err();
        match err {
            ProtocolError::DecodingError(_) => {}
            other => panic!("expected ProtocolError::DecodingError, got {:?}", other),
        }
    }

    // -- equality / clone / hash (derives) --
    #[test]
    fn test_contract_bounds_equality_and_clone() {
        let id = Identifier::from([0x11u8; 32]);
        let a = ContractBounds::SingleContract { id };
        let b = a.clone();
        assert_eq!(a, b);

        let different = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "foo".to_string(),
        };
        assert_ne!(a, different);
    }

    #[test]
    fn test_contract_bounds_type_string_roundtrip_with_from_str() {
        // The string form produced by the variant should round-trip through from_str
        // back to its numeric discriminant.
        let id_bytes = vec![0xD0u8; 32];
        let sc = ContractBounds::new_from_type(0, id_bytes.clone(), "".to_string()).unwrap();
        let sctd = ContractBounds::new_from_type(1, id_bytes, "docType".to_string()).unwrap();

        assert_eq!(
            ContractBounds::contract_bounds_type_from_str(sc.contract_bounds_type_string())
                .unwrap(),
            sc.contract_bounds_type()
        );
        assert_eq!(
            ContractBounds::contract_bounds_type_from_str(sctd.contract_bounds_type_string())
                .unwrap(),
            sctd.contract_bounds_type()
        );
    }
}

#[cfg(all(test, feature = "json-conversion"))]
mod tests {
    use super::*;
    use crate::serialization::JsonConvertible;

    #[test]
    fn contract_bounds_single_contract_json_round_trip() {
        let id = Identifier::from([0xABu8; 32]);
        let bounds = ContractBounds::SingleContract { id };

        let json = bounds.to_json().expect("to_json should succeed");
        assert!(
            json["id"].is_string(),
            "Identifier should be a base58 string, got: {:?}",
            json["id"]
        );

        let expected_base58 = id.to_string(platform_value::string_encoding::Encoding::Base58);
        assert_eq!(json["id"].as_str().unwrap(), expected_base58);

        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);
    }

    #[test]
    fn contract_bounds_document_type_json_round_trip() {
        let id = Identifier::from([0xCDu8; 32]);
        let bounds = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "myDocument".to_string(),
        };

        let json = bounds.to_json().expect("to_json should succeed");
        assert!(json["id"].is_string());
        assert_eq!(json["documentTypeName"].as_str().unwrap(), "myDocument");

        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);
    }

    #[test]
    fn contract_bounds_contract_group_json_round_trip() {
        let id = Identifier::from([0xEFu8; 32]);
        let bounds = ContractBounds::ContractGroup { id };

        let json = bounds.to_json().expect("to_json should succeed");
        assert_eq!(json["$type"].as_str().unwrap(), "contractGroup");
        assert_eq!(
            json["id"].as_str().unwrap(),
            id.to_string(platform_value::string_encoding::Encoding::Base58)
        );
        assert!(json.get("documentTypeName").is_none());

        let restored = ContractBounds::from_json(json).expect("from_json should succeed");
        assert_eq!(bounds, restored);

        let obj = bounds.to_object().expect("to_object should succeed");
        assert_eq!(
            ContractBounds::from_object(obj).expect("from_object"),
            bounds
        );
    }

    #[test]
    fn contract_bounds_value_round_trip() {
        let id = Identifier::from([0x55u8; 32]);
        let bounds = ContractBounds::SingleContractDocumentType {
            id,
            document_type_name: "note".to_string(),
        };

        let obj = bounds.to_object().expect("to_object should succeed");
        let restored = ContractBounds::from_object(obj).expect("from_object should succeed");
        assert_eq!(bounds, restored);
    }
}

#[cfg(all(test, feature = "state-transitions"))]
mod batched_transition_tests {
    use super::{BatchedTransitionBoundsCheck, ContractBounds};
    use crate::contract_group::ContractGroupMember;
    use crate::identifier::Identifier;
    use crate::state_transition::batch_transition::batched_transition::{
        document_create_transition::DocumentCreateTransitionV0,
        document_delete_transition::DocumentDeleteTransitionV0,
        document_index_only_delete_transition::DocumentIndexOnlyDeleteTransitionV0,
        document_purchase_transition::DocumentPurchaseTransitionV0,
        document_replace_transition::DocumentReplaceTransitionV0,
        document_transfer_transition::DocumentTransferTransitionV0,
        document_update_price_transition::DocumentUpdatePriceTransitionV0,
        token_transfer_transition::TokenTransferTransitionV0, BatchedTransitionRef,
        DocumentTransition, TokenTransition,
    };

    // Default transitions target contract [0; 32] and the empty document type name.
    fn documents() -> Vec<DocumentTransition> {
        vec![
            DocumentTransition::Create(DocumentCreateTransitionV0::default().into()),
            DocumentTransition::Replace(DocumentReplaceTransitionV0::default().into()),
            DocumentTransition::Delete(DocumentDeleteTransitionV0::default().into()),
            DocumentTransition::IndexOnlyDelete(
                DocumentIndexOnlyDeleteTransitionV0::default().into(),
            ),
            DocumentTransition::Transfer(DocumentTransferTransitionV0::default().into()),
            DocumentTransition::UpdatePrice(DocumentUpdatePriceTransitionV0::default().into()),
            DocumentTransition::Purchase(DocumentPurchaseTransitionV0::default().into()),
        ]
    }

    fn tokens() -> Vec<TokenTransition> {
        vec![
            TokenTransition::Burn(Default::default()),
            TokenTransition::Mint(Default::default()),
            TokenTransition::Transfer(TokenTransferTransitionV0::default().into()),
            TokenTransition::Freeze(Default::default()),
            TokenTransition::Unfreeze(Default::default()),
            TokenTransition::DestroyFrozenFunds(Default::default()),
            TokenTransition::Claim(Default::default()),
            TokenTransition::EmergencyAction(Default::default()),
            TokenTransition::ConfigUpdate(Default::default()),
            TokenTransition::DirectPurchase(Default::default()),
            TokenTransition::SetPriceForDirectPurchase(Default::default()),
        ]
    }

    #[test]
    fn single_contract_bounds_cover_every_operation_on_that_contract_only() {
        let bounds = ContractBounds::SingleContract {
            id: Identifier::from([0; 32]),
        };
        let foreign = ContractBounds::SingleContract {
            id: Identifier::from([1; 32]),
        };
        for document in documents() {
            let member = BatchedTransitionRef::Document(&document);
            assert_eq!(
                bounds.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Allowed,
                "{document:?}"
            );
            assert_eq!(
                foreign.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Denied,
                "{document:?}"
            );
        }
        for token in tokens() {
            let member = BatchedTransitionRef::Token(&token);
            assert_eq!(
                bounds.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Allowed,
                "{token:?}"
            );
            assert_eq!(
                foreign.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Denied,
                "{token:?}"
            );
        }
    }

    #[test]
    fn document_type_bounds_cover_that_type_only_and_never_tokens() {
        let bounds = ContractBounds::SingleContractDocumentType {
            id: Identifier::from([0; 32]),
            document_type_name: String::new(),
        };
        let other_type = ContractBounds::SingleContractDocumentType {
            id: Identifier::from([0; 32]),
            document_type_name: "other".to_string(),
        };
        for document in documents() {
            let member = BatchedTransitionRef::Document(&document);
            assert_eq!(
                bounds.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Allowed,
                "{document:?}"
            );
            assert_eq!(
                other_type.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Denied,
                "{document:?}"
            );
        }
        for token in tokens() {
            let member = BatchedTransitionRef::Token(&token);
            assert_eq!(
                bounds.check_batched_transition(member),
                BatchedTransitionBoundsCheck::Denied,
                "token operations are contract-wide: {token:?}"
            );
        }
    }

    #[test]
    fn should_defer_contract_group_bounds_to_state_with_the_member_to_look_up() {
        let group_id = Identifier::from([7; 32]);
        let bounds = ContractBounds::ContractGroup { id: group_id };
        for document in documents() {
            let member = BatchedTransitionRef::Document(&document);
            assert_eq!(
                bounds.check_batched_transition(member),
                BatchedTransitionBoundsCheck::RequiresContractGroupMembership {
                    contract_group_id: group_id,
                    contract_id: Identifier::from([0; 32]),
                    member: ContractGroupMember::DocumentType(String::new()),
                },
                "{document:?}"
            );
        }
        for token in tokens() {
            let member = BatchedTransitionRef::Token(&token);
            assert_eq!(
                bounds.check_batched_transition(member),
                BatchedTransitionBoundsCheck::RequiresContractGroupMembership {
                    contract_group_id: group_id,
                    contract_id: Identifier::from([0; 32]),
                    member: ContractGroupMember::Token(0),
                },
                "{token:?}"
            );
        }
    }
}
