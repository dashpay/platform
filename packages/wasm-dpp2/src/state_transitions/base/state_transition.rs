use crate::core::private_key::PrivateKeyWasm;
use crate::enums::keys::key_type::KeyTypeWasm;
use crate::enums::keys::purpose::PurposeWasm;
use crate::enums::keys::security_level::SecurityLevelWasm;
use crate::error::{WasmDppError, WasmDppResult};
use crate::identifier::{IdentifierLikeJs, IdentifierWasm};
use crate::identity::public_key::IdentityPublicKeyWasm;
use crate::impl_wasm_type_info;
use crate::mock_bls::MockBLS;
use dpp::dashcore::secp256k1::hashes::hex::Case::Lower;
use dpp::dashcore::secp256k1::hashes::hex::DisplayHex;
use dpp::data_contract::serialized_version::DataContractInSerializationFormat;
use dpp::identity::{KeyID, KeyType};
use dpp::platform_value::BinaryData;
use dpp::platform_value::string_encoding::{Encoding, decode, encode};
use dpp::prelude::Identifier;
use dpp::prelude::{IdentityNonce, UserFeeIncrease};
use dpp::serialization::{PlatformDeserializable, PlatformSerializable, Signable};
use dpp::state_transition::StateTransition::{
    Batch, DataContractCreate, DataContractUpdate, IdentityCreditTransfer,
    IdentityCreditWithdrawal, IdentityUpdate, MasternodeVote,
};
use dpp::state_transition::batch_transition::BatchTransition;
use dpp::state_transition::batch_transition::batched_transition::BatchedTransition;
use dpp::state_transition::batch_transition::batched_transition::document_transition::DocumentTransitionV0Methods;
use dpp::state_transition::batch_transition::batched_transition::token_transition::TokenTransitionV0Methods;
use dpp::state_transition::batch_transition::methods::v0::DocumentsBatchTransitionMethodsV0;
use dpp::state_transition::data_contract_create_transition::DataContractCreateTransition;
use dpp::state_transition::data_contract_create_transition::accessors::DataContractCreateTransitionAccessorsV0;
use dpp::state_transition::data_contract_update_transition::DataContractUpdateTransition;
use dpp::state_transition::data_contract_update_transition::accessors::DataContractUpdateTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_to_addresses_transition::accessors::IdentityCreditTransferToAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_credit_transfer_transition::accessors::IdentityCreditTransferTransitionAccessorsV0;
use dpp::state_transition::identity_credit_withdrawal_transition::accessors::IdentityCreditWithdrawalTransitionAccessorsV0;
use dpp::state_transition::identity_topup_from_addresses_transition::accessors::IdentityTopUpFromAddressesTransitionAccessorsV0;
use dpp::state_transition::identity_topup_transition::accessors::IdentityTopUpTransitionAccessorsV0;
use dpp::state_transition::identity_update_transition::accessors::IdentityUpdateTransitionAccessorsV0;
use dpp::state_transition::masternode_vote_transition::MasternodeVoteTransition;
use dpp::state_transition::masternode_vote_transition::accessors::MasternodeVoteTransitionAccessorsV0;
use dpp::state_transition::{
    StateTransition, StateTransitionIdentitySigned, StateTransitionSigningOptions,
};
use sha2::{Digest, Sha256};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

#[derive(Clone)]
#[wasm_bindgen(js_name = "StateTransition")]
pub struct StateTransitionWasm(StateTransition);

impl From<StateTransition> for StateTransitionWasm {
    fn from(transition: StateTransition) -> Self {
        StateTransitionWasm(transition)
    }
}

impl From<StateTransitionWasm> for StateTransition {
    fn from(transition: StateTransitionWasm) -> Self {
        transition.0
    }
}

impl From<&StateTransitionWasm> for StateTransition {
    fn from(transition: &StateTransitionWasm) -> Self {
        transition.0.clone()
    }
}

#[wasm_bindgen(js_class = StateTransition)]
impl StateTransitionWasm {
    #[wasm_bindgen(js_name = "sign")]
    pub fn sign(
        &mut self,
        #[wasm_bindgen(js_name = "privateKey")] private_key: &PrivateKeyWasm,
        #[wasm_bindgen(js_name = "publicKey")] public_key: &IdentityPublicKeyWasm,
    ) -> WasmDppResult<Vec<u8>> {
        self.0.sign(
            &public_key.clone().into(),
            private_key.to_bytes().as_slice(),
            &MockBLS {},
        )?;

        let Some(signature) = self.0.signature() else {
            return Err(WasmDppError::generic(
                "Signature was not set after signing the state transition; this is a bug",
            ));
        };

        let Some(signature_public_key) = self.0.signature_public_key_id() else {
            return Err(WasmDppError::generic(
                "Signature public key ID was not set after signing the state transition; this is a bug",
            ));
        };

        self.0.set_signature(signature.clone());
        self.0.set_signature_public_key_id(signature_public_key);

        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "signByPrivateKey")]
    pub fn sign_by_private_key(
        &mut self,
        #[wasm_bindgen(js_name = "privateKey")] private_key: &PrivateKeyWasm,
        #[wasm_bindgen(js_name = "keyId")] key_id: Option<KeyID>,
        #[wasm_bindgen(js_name = "keyType")] key_type: JsValue,
    ) -> WasmDppResult<Vec<u8>> {
        let key_type = if key_type.is_undefined() {
            KeyTypeWasm::ECDSA_SECP256K1
        } else {
            KeyTypeWasm::try_from(key_type)?
        };

        self.0.sign_by_private_key(
            private_key.to_bytes().as_slice(),
            KeyType::from(key_type),
            &MockBLS {},
        )?;

        if let Some(key_id) = key_id {
            self.0.set_signature_public_key_id(key_id);
        }

        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "verifyPublicKey")]
    pub fn verify_public_key(
        &self,
        #[wasm_bindgen(js_name = "publicKey")] public_key: &IdentityPublicKeyWasm,
        #[wasm_bindgen(js_name = "allowSigningWithAnySecurityLevel")]
        allow_signing_with_any_security_level: Option<bool>,
        #[wasm_bindgen(js_name = "allowSigningWithAnyPurpose")]
        allow_signing_with_any_purpose: Option<bool>,
    ) -> WasmDppResult<()> {
        let allow_signing_with_any_security_level =
            allow_signing_with_any_security_level.unwrap_or(false);
        let allow_signing_with_any_purpose = allow_signing_with_any_purpose.unwrap_or(false);

        match &self.0 {
            DataContractCreate(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            DataContractUpdate(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            Batch(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            IdentityCreditWithdrawal(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            IdentityUpdate(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            IdentityCreditTransfer(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            MasternodeVote(st) => {
                st.verify_public_key_level_and_purpose(
                    &public_key.clone().into(),
                    StateTransitionSigningOptions {
                        allow_signing_with_any_security_level,
                        allow_signing_with_any_purpose,
                    },
                )?;

                st.verify_public_key_is_enabled(&public_key.clone().into())?;
            }
            _ => {}
        }

        Ok(())
    }

    #[wasm_bindgen(js_name = "toBytes")]
    pub fn to_bytes(&self) -> WasmDppResult<Vec<u8>> {
        Ok(self.0.serialize_to_bytes()?)
    }

    #[wasm_bindgen(js_name = "toHex")]
    pub fn to_hex(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Encoding::Hex))
    }

    #[wasm_bindgen(js_name = "toBase64")]
    pub fn to_base64(&self) -> WasmDppResult<String> {
        let bytes = self.0.serialize_to_bytes()?;
        Ok(encode(bytes.as_slice(), Encoding::Base64))
    }

    #[wasm_bindgen(js_name = "fromBytes")]
    pub fn from_bytes(bytes: Vec<u8>) -> WasmDppResult<StateTransitionWasm> {
        let st = StateTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(st.into())
    }

    #[wasm_bindgen(js_name = "fromHex")]
    pub fn from_hex(hex: String) -> WasmDppResult<StateTransitionWasm> {
        let bytes =
            decode(&hex, Encoding::Hex).map_err(|e| WasmDppError::serialization(e.to_string()))?;

        let st = StateTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(st.into())
    }

    #[wasm_bindgen(js_name = "fromBase64")]
    pub fn from_base64(base64: String) -> WasmDppResult<StateTransitionWasm> {
        let bytes = decode(&base64, Encoding::Base64)
            .map_err(|e| WasmDppError::serialization(e.to_string()))?;

        let st = StateTransition::deserialize_from_bytes(bytes.as_slice())?;

        Ok(st.into())
    }

    #[wasm_bindgen(js_name = "hash")]
    pub fn get_hash(
        &self,
        #[wasm_bindgen(js_name = "skipSignature")] skip_signature: bool,
    ) -> WasmDppResult<String> {
        let payload = if skip_signature {
            self.0.signable_bytes()?
        } else {
            dpp::serialization::PlatformSerializable::serialize_to_bytes(&self.0)?
        };

        Ok(Sha256::digest(payload).to_hex_string(Lower))
    }

    #[wasm_bindgen(getter = "actionType")]
    pub fn action_type(&self) -> String {
        self.0.name()
    }

    #[wasm_bindgen(getter = "actionTypeNumber")]
    pub fn action_type_number(&self) -> u8 {
        use StateTransition::*;
        match self.0 {
            DataContractCreate(_) => 0,
            Batch(_) => 1,
            IdentityCreate(_) => 2,
            IdentityTopUp(_) => 3,
            DataContractUpdate(_) => 4,
            IdentityUpdate(_) => 5,
            IdentityCreditWithdrawal(_) => 6,
            IdentityCreditTransfer(_) => 7,
            MasternodeVote(_) => 8,
            IdentityCreditTransferToAddresses(_) => 9,
            IdentityCreateFromAddresses(_) => 10,
            IdentityTopUpFromAddresses(_) => 11,
            AddressFundsTransfer(_) => 12,
            AddressFundingFromAssetLock(_) => 13,
            AddressCreditWithdrawal(_) => 14,
        }
    }

    #[wasm_bindgen(getter = "ownerId")]
    pub fn owner_id(&self) -> Option<IdentifierWasm> {
        self.0.owner_id().map(Into::into)
    }

    #[wasm_bindgen(getter = "signature")]
    pub fn signature(&self) -> Option<Vec<u8>> {
        self.0.signature().map(BinaryData::to_vec)
    }

    #[wasm_bindgen(getter = "signaturePublicKeyId")]
    pub fn signature_public_key_id(&self) -> Option<KeyID> {
        self.0.signature_public_key_id()
    }

    #[wasm_bindgen(getter = "userFeeIncrease")]
    pub fn user_fee_increase(&self) -> UserFeeIncrease {
        self.0.user_fee_increase()
    }

    #[wasm_bindgen(getter = "purposeRequirement")]
    pub fn purpose_requirement(&self) -> Option<Vec<String>> {
        let requirements = self.0.purpose_requirement();

        requirements.map(|req| {
            req.iter()
                .map(|purpose| PurposeWasm::from(*purpose))
                .map(String::from)
                .collect()
        })
    }

    #[wasm_bindgen(js_name = "getKeyLevelRequirement")]
    pub fn get_key_level_requirement(
        &self,
        purpose: &JsValue,
    ) -> WasmDppResult<Option<Vec<String>>> {
        let purpose = PurposeWasm::try_from(purpose.clone())?;

        let requirements = self.0.security_level_requirement(purpose.into());

        match requirements {
            None => Ok(None),
            Some(req) => Ok(Some(
                req.iter()
                    .map(|security_level| SecurityLevelWasm::from(*security_level))
                    .map(String::from)
                    .collect(),
            )),
        }
    }

    #[wasm_bindgen(getter = "identityContractNonce")]
    pub fn identity_contract_nonce(&self) -> Option<IdentityNonce> {
        use StateTransition::*;
        match &self.0 {
            DataContractCreate(_) => None,
            DataContractUpdate(contract_update) => Some(contract_update.identity_contract_nonce()),
            Batch(batch) => match batch {
                BatchTransition::V0(v0) => Some(v0.transitions.first()?.identity_contract_nonce()),
                BatchTransition::V1(v1) => match v1.transitions.first()? {
                    BatchedTransition::Document(doc_batch) => {
                        Some(doc_batch.identity_contract_nonce())
                    }
                    BatchedTransition::Token(token_batch) => {
                        Some(token_batch.identity_contract_nonce())
                    }
                },
            },
            StateTransition::IdentityCreate(_) => None,
            IdentityTopUp(_) => None,
            IdentityCreditWithdrawal(_) => None,
            IdentityUpdate(_) => None,
            IdentityCreditTransfer(_) => None,
            MasternodeVote(_) => None,
            IdentityCreditTransferToAddresses(_)
            | IdentityCreateFromAddresses(_)
            | IdentityTopUpFromAddresses(_)
            | AddressFundsTransfer(_)
            | AddressFundingFromAssetLock(_)
            | AddressCreditWithdrawal(_) => None,
        }
    }

    #[wasm_bindgen(getter = "identityNonce")]
    pub fn identity_nonce(&self) -> Option<IdentityNonce> {
        use StateTransition::*;
        match &self.0 {
            DataContractCreate(contract_create) => Some(contract_create.identity_nonce()),
            DataContractUpdate(_) => None,
            Batch(_) => None,
            StateTransition::IdentityCreate(_) => None,
            IdentityTopUp(_) => None,
            IdentityCreditWithdrawal(withdrawal) => Some(withdrawal.nonce()),
            IdentityUpdate(identity_update) => Some(identity_update.nonce()),
            IdentityCreditTransfer(credit_transfer) => Some(credit_transfer.nonce()),
            MasternodeVote(mn_vote) => Some(mn_vote.nonce()),
            IdentityCreditTransferToAddresses(ct) => Some(ct.nonce()),
            IdentityCreateFromAddresses(_) => None,
            IdentityTopUpFromAddresses(_) => None,
            AddressFundsTransfer(_)
            | AddressFundingFromAssetLock(_)
            | AddressCreditWithdrawal(_) => None,
        }
    }

    #[wasm_bindgen(setter = "signature")]
    pub fn set_signature(&mut self, signature: Vec<u8>) -> bool {
        self.0.set_signature(BinaryData::from(signature))
    }

    #[wasm_bindgen(setter = "signaturePublicKeyId")]
    pub fn set_signature_public_key_id(
        &mut self,
        #[wasm_bindgen(js_name = "keyId")] key_id: KeyID,
    ) {
        self.0.set_signature_public_key_id(key_id)
    }

    #[wasm_bindgen(setter = "userFeeIncrease")]
    pub fn set_user_fee_increase(
        &mut self,
        #[wasm_bindgen(js_name = "userFeeIncrease")] user_fee_increase: UserFeeIncrease,
    ) {
        self.0.set_user_fee_increase(user_fee_increase)
    }

    #[wasm_bindgen(js_name = "setOwnerId")]
    pub fn set_owner_id(
        &mut self,
        #[wasm_bindgen(js_name = "ownerId")] owner_id: IdentifierLikeJs,
    ) -> WasmDppResult<()> {
        use dpp::state_transition::StateTransition::*;
        let owner_id: Identifier = owner_id.try_into()?;

        match self.0.clone() {
            DataContractCreate(mut contract_create) => {
                let new_contract = match contract_create.data_contract().clone() {
                    DataContractInSerializationFormat::V0(mut v0) => {
                        v0.owner_id = owner_id;

                        DataContractInSerializationFormat::V0(v0)
                    }
                    DataContractInSerializationFormat::V1(mut v1) => {
                        v1.owner_id = owner_id;

                        DataContractInSerializationFormat::V1(v1)
                    }
                };

                contract_create.set_data_contract(new_contract);

                self.0 = DataContractCreate(contract_create);
            }
            DataContractUpdate(mut contract_update) => {
                if let Some(data_contract) = contract_update.data_contract() {
                    let new_contract = match data_contract.clone() {
                        DataContractInSerializationFormat::V0(mut v0) => {
                            v0.owner_id = owner_id;

                            DataContractInSerializationFormat::V0(v0)
                        }
                        DataContractInSerializationFormat::V1(mut v1) => {
                            v1.owner_id = owner_id;

                            DataContractInSerializationFormat::V1(v1)
                        }
                    };

                    contract_update.set_data_contract(new_contract);
                }

                self.0 = DataContractUpdate(contract_update);
            }
            Batch(mut batch) => {
                batch = match batch {
                    BatchTransition::V0(mut v0) => {
                        v0.owner_id = owner_id;

                        BatchTransition::V0(v0)
                    }
                    BatchTransition::V1(mut v1) => {
                        v1.owner_id = owner_id;

                        BatchTransition::V1(v1)
                    }
                };

                self.0 = Batch(batch);
            }
            StateTransition::IdentityCreate(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set owner for identity create transition",
                ));
            }
            IdentityTopUp(mut top_up) => {
                top_up.set_identity_id(owner_id);

                self.0 = IdentityTopUp(top_up);
            }
            IdentityCreditWithdrawal(mut withdrawal) => {
                withdrawal.set_identity_id(owner_id);

                self.0 = IdentityCreditWithdrawal(withdrawal);
            }
            IdentityUpdate(mut identity_update) => {
                identity_update.set_identity_id(owner_id);

                self.0 = IdentityUpdate(identity_update);
            }
            IdentityCreditTransfer(mut credit_transfer) => {
                credit_transfer.set_identity_id(owner_id);

                self.0 = IdentityCreditTransfer(credit_transfer);
            }
            MasternodeVote(mut mn_vote) => {
                mn_vote.set_voter_identity_id(owner_id);

                self.0 = MasternodeVote(mn_vote);
            }

            IdentityCreditTransferToAddresses(mut ct) => {
                ct.set_identity_id(owner_id);
                self.0 = IdentityCreditTransferToAddresses(ct);
            }
            IdentityCreateFromAddresses(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set owner for identity create transition",
                ));
            }
            IdentityTopUpFromAddresses(mut top_up) => {
                top_up.set_identity_id(owner_id);
                self.0 = IdentityTopUpFromAddresses(top_up);
            }
            AddressFundsTransfer(_)
            | AddressFundingFromAssetLock(_)
            | AddressCreditWithdrawal(_) => {
                // NOOP - address funds transfer has no owner id
                return Err(WasmDppError::invalid_argument(
                    "Cannot set owner for address funds transfer transition",
                ));
            }
        };

        Ok(())
    }

    #[wasm_bindgen(js_name = "setIdentityContractNonce")]
    pub fn set_identity_contract_nonce(&mut self, nonce: JsValue) -> WasmDppResult<()> {
        use crate::utils::try_to_u64;
        let nonce: IdentityNonce = try_to_u64(&nonce, "identityContractNonce")?;
        use StateTransition::*;
        self.0 = match self.0.clone() {
            DataContractCreate(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Data Contract Create",
                ));
            }
            DataContractUpdate(contract_update) => match contract_update {
                DataContractUpdateTransition::V0(mut v0) => {
                    v0.identity_contract_nonce = nonce;

                    DataContractUpdateTransition::V0(v0).into()
                }
                DataContractUpdateTransition::V1(mut v1) => {
                    v1.identity_contract_nonce = nonce;

                    DataContractUpdateTransition::V1(v1).into()
                }
            },
            Batch(mut batch) => {
                batch.set_identity_contract_nonce(nonce);

                batch.into()
            }
            StateTransition::IdentityCreate(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Identity Create",
                ));
            }
            IdentityTopUp(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Identity Top Up",
                ));
            }
            IdentityCreditWithdrawal(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Identity Credit Withdrawal",
                ));
            }
            IdentityUpdate(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Identity Update",
                ));
            }
            IdentityCreditTransfer(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Identity Credit Transfer",
                ));
            }
            MasternodeVote(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for Masternode Vote",
                ));
            }
            IdentityCreditTransferToAddresses(_)
            | IdentityCreateFromAddresses(_)
            | IdentityTopUpFromAddresses(_)
            | AddressFundsTransfer(_)
            | AddressFundingFromAssetLock(_)
            | AddressCreditWithdrawal(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity contract nonce for address-related transition types",
                ));
            }
        };

        Ok(())
    }

    #[wasm_bindgen(js_name = "setIdentityNonce")]
    pub fn set_identity_nonce(&mut self, nonce: JsValue) -> WasmDppResult<()> {
        use crate::utils::try_to_u64;
        let nonce: IdentityNonce = try_to_u64(&nonce, "identityNonce")?;
        use StateTransition::*;
        self.0 = match self.0.clone() {
            DataContractCreate(mut contract_create) => {
                contract_create = match contract_create {
                    DataContractCreateTransition::V0(mut v0) => {
                        v0.identity_nonce = nonce;
                        v0.into()
                    }
                    DataContractCreateTransition::V1(mut v1) => {
                        v1.identity_nonce = nonce;
                        v1.into()
                    }
                };

                contract_create.into()
            }
            DataContractUpdate(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for Data Contract Update",
                ));
            }
            Batch(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for Batch",
                ));
            }
            StateTransition::IdentityCreate(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for Identity Create",
                ));
            }
            IdentityTopUp(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for Identity Top Up",
                ));
            }
            IdentityCreditWithdrawal(mut withdrawal) => {
                withdrawal.set_nonce(nonce);

                withdrawal.into()
            }
            IdentityUpdate(mut identity_update) => {
                identity_update.set_nonce(nonce);

                identity_update.into()
            }
            IdentityCreditTransfer(mut credit_transfer) => {
                credit_transfer.set_nonce(nonce);

                credit_transfer.into()
            }
            MasternodeVote(mut mn_vote) => {
                mn_vote = match mn_vote {
                    MasternodeVoteTransition::V0(mut v0) => {
                        v0.nonce = nonce;

                        v0.into()
                    }
                };

                mn_vote.into()
            }
            IdentityCreditTransferToAddresses(mut transfer) => {
                transfer.set_nonce(nonce);
                transfer.into()
            }
            IdentityCreateFromAddresses(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for Identity Create From Addresses",
                ));
            }
            IdentityTopUpFromAddresses(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for Identity Top Up From Addresses",
                ));
            }
            AddressFundsTransfer(_)
            | AddressFundingFromAssetLock(_)
            | AddressCreditWithdrawal(_) => {
                return Err(WasmDppError::invalid_argument(
                    "Cannot set identity nonce for address-related transition types",
                ));
            }
        };

        Ok(())
    }
}

impl_wasm_type_info!(StateTransitionWasm, StateTransition);
