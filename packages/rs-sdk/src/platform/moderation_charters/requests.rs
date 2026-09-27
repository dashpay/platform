//! Building the join and resignation requests, whose message only the leader reads.

use crate::platform::encrypted_for::encrypt_property_for;
use crate::platform::{DataContract, Document, Fetch, Identity};
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::SeedableRng;
use dpp::dashcore::secp256k1::SecretKey;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::document::{DocumentV0, DocumentV0Getters, INITIAL_REVISION};
use dpp::identity::accessors::IdentityGettersV0;
use dpp::moderation_charter::{
    property_names, ELECTED_CHARTER_DOCUMENT_TYPE_NAME, JOIN_REQUEST_DOCUMENT_TYPE_NAME,
    RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME, SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::{Bytes32, Identifier, Value};
use std::collections::BTreeMap;

/// The property of both request types that carries the message, declared `encryptedFor` the
/// leader.
const ENCRYPTED_MESSAGE: &str = "encryptedMessage";

/// What a join request says: which proposal the writer offers to serve on, and why.
pub struct JoinRequestInput {
    /// The proposal, a `submittedCharter` document.
    pub submitted_charter_id: Identifier,
    /// Why the writer wants to join, readable by the leader alone. The property holds at most
    /// 1040 bytes, so the message at most 1023, checked when the document is validated.
    pub message: Vec<u8>,
    /// The identity offering to serve, the join request's owner.
    pub writer: Identity,
    /// The private half of the writer's encryption key bound to `joinRequest`, the key the
    /// schema requires for `senderKeyId`.
    pub writer_encryption_key: SecretKey,
}

/// What a resignation request says: which seated charter the writer asks to leave, and why.
pub struct ResignationRequestInput {
    /// The seated charter, an `electedCharter` document.
    pub elected_charter_id: Identifier,
    /// Why the writer leaves, readable by the leader alone; at most 1023 bytes, as for a join
    /// request.
    pub message: Vec<u8>,
    /// The member asking to leave, the request's owner. Consensus refuses a writer who is not
    /// on the charter's team.
    pub writer: Identity,
    /// The private half of the writer's encryption key bound to `joinRequest`, the key the
    /// schema requires for `senderKeyId` (the same one its join request used).
    pub writer_encryption_key: SecretKey,
}

/// A request document ready to be put with
/// [`PutDocument`](crate::platform::transition::put_document::PutDocument): the document, the
/// name of its type and the entropy its id derives from, which the put must reuse.
#[derive(Debug, Clone)]
pub struct ModerationCharterRequest {
    /// The document. Its id is a placeholder until it is put: from protocol version 14 the id
    /// also commits to the identity contract nonce of the create transition.
    pub document: Document,
    /// `joinRequest` or `resignationRequest`.
    pub document_type_name: String,
    /// The entropy the document id derives from.
    pub entropy: Bytes32,
}

/// Builds a join request of `input.writer` for `proposal`, the `submittedCharter` document
/// `input.submitted_charter_id`, whose owner is `leader`: the message encrypted to the leader's
/// decryption key bound to `submittedCharter` from the writer's encryption key bound to
/// `joinRequest`, and `recipientId`, `recipientKeyId` and `senderKeyId` set to match.
pub fn build_join_request_document(
    contract: &DataContract,
    proposal: &Document,
    leader: &Identity,
    input: &JoinRequestInput,
    entropy: Bytes32,
) -> Result<ModerationCharterRequest, Error> {
    if proposal.id() != input.submitted_charter_id {
        return Err(Error::Generic(format!(
            "the proposal given is {}, not {}",
            proposal.id(),
            input.submitted_charter_id
        )));
    }
    build_request(
        contract,
        JOIN_REQUEST_DOCUMENT_TYPE_NAME,
        (property_names::SUBMITTED_CHARTER_ID, proposal),
        leader,
        &input.message,
        &input.writer,
        &input.writer_encryption_key,
        entropy,
    )
}

/// Builds a resignation request of `input.writer` from `elected_charter`, the `electedCharter`
/// document `input.elected_charter_id`, whose owner is `leader`, encrypted as a join request
/// is.
pub fn build_resignation_request_document(
    contract: &DataContract,
    elected_charter: &Document,
    leader: &Identity,
    input: &ResignationRequestInput,
    entropy: Bytes32,
) -> Result<ModerationCharterRequest, Error> {
    if elected_charter.id() != input.elected_charter_id {
        return Err(Error::Generic(format!(
            "the elected charter given is {}, not {}",
            elected_charter.id(),
            input.elected_charter_id
        )));
    }
    build_request(
        contract,
        RESIGNATION_REQUEST_DOCUMENT_TYPE_NAME,
        (property_names::ELECTED_CHARTER_ID, elected_charter),
        leader,
        &input.message,
        &input.writer,
        &input.writer_encryption_key,
        entropy,
    )
}

/// A request of `document_type_name` referring to `referred`, whose owner, `leader`, the
/// message is encrypted to.
#[allow(clippy::too_many_arguments)]
fn build_request(
    contract: &DataContract,
    document_type_name: &str,
    (reference_property, referred): (&str, &Document),
    leader: &Identity,
    message: &[u8],
    writer: &Identity,
    writer_encryption_key: &SecretKey,
    entropy: Bytes32,
) -> Result<ModerationCharterRequest, Error> {
    if referred.owner_id() != leader.id() {
        return Err(Error::Generic(format!(
            "the leader given, {}, is not the owner of {}",
            leader.id(),
            referred.id()
        )));
    }
    let document_type = contract
        .document_type_for_name(document_type_name)
        .map_err(|e| Error::Protocol(e.into()))?;
    let mut properties = BTreeMap::from([(
        reference_property.to_string(),
        Value::Identifier(referred.id().to_buffer()),
    )]);
    encrypt_property_for(
        document_type,
        ENCRYPTED_MESSAGE,
        message,
        writer,
        writer_encryption_key,
        leader,
        &mut properties,
    )?;
    let document = DocumentV0 {
        id: Document::generate_document_id_v0(
            &contract.id(),
            &writer.id(),
            document_type_name,
            entropy.as_slice(),
        ),
        owner_id: writer.id(),
        properties,
        revision: document_type
            .requires_revision()
            .then_some(INITIAL_REVISION),
        ..Default::default()
    }
    .into();
    Ok(ModerationCharterRequest {
        document,
        document_type_name: document_type_name.to_string(),
        entropy,
    })
}

impl Sdk {
    /// Builds a join request for the proposal `input.submitted_charter_id`: fetches the
    /// proposal and its leader, and encrypts the message with the keys the schema demands (see
    /// [`build_join_request_document`]). The result is put like any document create.
    pub async fn build_join_request(
        &self,
        input: JoinRequestInput,
    ) -> Result<ModerationCharterRequest, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        let proposal = self
            .fetch_submitted_charter_of(contract.clone(), input.submitted_charter_id)
            .await?
            .ok_or_else(|| {
                Error::Generic(format!(
                    "no {SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME} {}",
                    input.submitted_charter_id
                ))
            })?;
        let leader = self.fetch_leader(&proposal).await?;
        build_join_request_document(&contract, &proposal, &leader, &input, fresh_entropy())
    }

    /// Builds a resignation request from the seated charter `input.elected_charter_id`:
    /// fetches the charter and its leader, and encrypts the message with the keys the schema
    /// demands (see [`build_resignation_request_document`]). The result is put like any
    /// document create; deleting the document withdraws the request.
    pub async fn build_resignation_request(
        &self,
        input: ResignationRequestInput,
    ) -> Result<ModerationCharterRequest, Error> {
        let contract = self.fetch_moderation_charters_contract().await?;
        let charter = self
            .fetch_elected_charter_of(contract.clone(), input.elected_charter_id)
            .await?
            .ok_or_else(|| {
                Error::Generic(format!(
                    "no {ELECTED_CHARTER_DOCUMENT_TYPE_NAME} {}",
                    input.elected_charter_id
                ))
            })?;
        let leader = self.fetch_leader(&charter.document).await?;
        build_resignation_request_document(
            &contract,
            &charter.document,
            &leader,
            &input,
            fresh_entropy(),
        )
    }

    /// The owner of `document`, the leader of a proposal or a charter.
    async fn fetch_leader(&self, document: &Document) -> Result<Identity, Error> {
        Identity::fetch(self, document.owner_id())
            .await?
            .ok_or_else(|| Error::Generic(format!("leader {} not found", document.owner_id())))
    }
}

fn fresh_entropy() -> Bytes32 {
    Bytes32::random_with_rng(&mut StdRng::from_entropy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::encrypted_for::{decrypt_property, EncryptedPropertyEnvelope};
    use dpp::dashcore::secp256k1::{PublicKey, Secp256k1};
    use dpp::data_contract::validate_document::DataContractDocumentValidationMethodsV0;
    use dpp::identity::contract_bounds::ContractBounds;
    use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
    use dpp::identity::v0::IdentityV0;
    use dpp::identity::{IdentityPublicKey, KeyType, Purpose, SecurityLevel};
    use dpp::moderation_charter::MODERATION_CHARTERS_CONTRACT_ID;
    use dpp::platform_value::BinaryData;
    use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
    use dpp::version::PlatformVersion;

    fn key_pair(scalar: u8) -> (SecretKey, PublicKey) {
        let secret_key = SecretKey::from_slice(&[scalar; 32]).expect("a valid scalar");
        let public_key = PublicKey::from_secret_key(&Secp256k1::signing_only(), &secret_key);
        (secret_key, public_key)
    }

    fn identity_with_key(
        id: u8,
        key_id: u32,
        purpose: Purpose,
        bound_to: &str,
        public_key: &PublicKey,
    ) -> Identity {
        let key: IdentityPublicKey = IdentityPublicKeyV0 {
            id: key_id,
            purpose,
            security_level: SecurityLevel::MEDIUM,
            contract_bounds: Some(ContractBounds::SingleContractDocumentType {
                id: MODERATION_CHARTERS_CONTRACT_ID,
                document_type_name: bound_to.to_string(),
            }),
            key_type: KeyType::ECDSA_SECP256K1,
            read_only: false,
            data: BinaryData::new(public_key.serialize().to_vec()),
            disabled_at: None,
        }
        .into();
        IdentityV0 {
            id: Identifier::from([id; 32]),
            public_keys: BTreeMap::from([(key_id, key)]),
            balance: 0,
            revision: 0,
        }
        .into()
    }

    fn owned_by(id: u8, owner: &Identity) -> Document {
        DocumentV0 {
            id: Identifier::from([id; 32]),
            owner_id: owner.id(),
            ..Default::default()
        }
        .into()
    }

    #[test]
    fn should_build_requests_the_leader_decrypts_that_the_schema_accepts() {
        let platform_version = PlatformVersion::latest();
        let contract =
            load_system_data_contract(SystemDataContract::ModerationCharters, platform_version)
                .expect("loads");
        let (leader_private_key, leader_public_key) = key_pair(0x42);
        let leader = identity_with_key(
            1,
            6,
            Purpose::DECRYPTION,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            &leader_public_key,
        );
        let (writer_private_key, writer_public_key) = key_pair(0x21);
        let writer = identity_with_key(
            2,
            3,
            Purpose::ENCRYPTION,
            JOIN_REQUEST_DOCUMENT_TYPE_NAME,
            &writer_public_key,
        );
        let proposal = owned_by(0x50, &leader);
        let charter = owned_by(0x60, &leader);

        let join = build_join_request_document(
            &contract,
            &proposal,
            &leader,
            &JoinRequestInput {
                submitted_charter_id: proposal.id(),
                message: b"let me help".to_vec(),
                writer: writer.clone(),
                writer_encryption_key: writer_private_key,
            },
            Bytes32::new([9; 32]),
        )
        .expect("builds");
        let resignation = build_resignation_request_document(
            &contract,
            &charter,
            &leader,
            &ResignationRequestInput {
                elected_charter_id: charter.id(),
                message: b"moving on".to_vec(),
                writer: writer.clone(),
                writer_encryption_key: writer_private_key,
            },
            Bytes32::new([8; 32]),
        )
        .expect("builds");

        for (request, reference, referred, message) in [
            (
                &join,
                property_names::SUBMITTED_CHARTER_ID,
                &proposal,
                &b"let me help"[..],
            ),
            (
                &resignation,
                property_names::ELECTED_CHARTER_ID,
                &charter,
                &b"moving on"[..],
            ),
        ] {
            let document_type = contract
                .document_type_for_name(&request.document_type_name)
                .expect("exists");
            assert_eq!(request.document.owner_id(), writer.id());
            assert_eq!(
                request.document.properties().get(reference),
                Some(&Value::Identifier(referred.id().to_buffer()))
            );
            assert_eq!(
                EncryptedPropertyEnvelope::read(
                    document_type,
                    ENCRYPTED_MESSAGE,
                    &request.document
                )
                .expect("reads"),
                EncryptedPropertyEnvelope {
                    recipient_id: leader.id(),
                    recipient_key_id: 6,
                    sender_id: writer.id(),
                    sender_key_id: 3,
                }
            );
            assert_eq!(
                decrypt_property(
                    document_type,
                    ENCRYPTED_MESSAGE,
                    request.document.properties(),
                    &leader_private_key,
                    &writer_public_key,
                )
                .expect("the leader decrypts"),
                message
            );
            // The properties pass the schema and the consensus shape check
            let properties = request.document.properties();
            assert!(document_type
                .validate_encrypted_property_shapes(properties, platform_version)
                .expect("runs")
                .is_valid());
            let schema_result = contract
                .validate_document_properties(
                    &request.document_type_name,
                    Value::from(properties.clone()),
                    platform_version,
                )
                .expect("runs");
            assert!(
                schema_result.is_valid(),
                "{} fails its schema: {:?}",
                request.document_type_name,
                schema_result.errors
            );
        }

        // A leader who does not own the proposal is refused
        let someone = identity_with_key(
            3,
            6,
            Purpose::DECRYPTION,
            SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
            &leader_public_key,
        );
        assert!(build_join_request_document(
            &contract,
            &proposal,
            &someone,
            &JoinRequestInput {
                submitted_charter_id: proposal.id(),
                message: b"x".to_vec(),
                writer,
                writer_encryption_key: writer_private_key,
            },
            Bytes32::new([9; 32]),
        )
        .is_err());
    }
}
