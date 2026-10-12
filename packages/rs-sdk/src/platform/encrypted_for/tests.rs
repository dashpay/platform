use super::*;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::config::DataContractConfig;
use dpp::data_contract::document_type::methods::DocumentTypeBasicMethods;
use dpp::data_contract::document_type::DocumentType;
use dpp::document::DocumentV0;
use dpp::identity::contract_bounds::ContractBounds;
use dpp::identity::identity_public_key::v0::IdentityPublicKeyV0;
use dpp::identity::v0::IdentityV0;
use dpp::identity::SecurityLevel;
use dpp::moderation_charter::{
    JOIN_REQUEST_DOCUMENT_TYPE_NAME, MODERATION_CHARTERS_CONTRACT_ID,
    SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME,
};
use dpp::platform_value::{platform_value, BinaryData};
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use platform_encryption::{
    compact_xpub_bytes, decrypt_extended_public_key, encrypt_extended_public_key,
};

const ENCRYPTED_MESSAGE: &str = "encryptedMessage";

/// The IV of the dashpay vector below.
const DASHPAY_VECTOR_IV: [u8; 16] = [0x5a; 16];

/// A dashpay `encryptedPublicKey`: the 69-byte compact xpub of [`dashpay_vector_xpub`]
/// encrypted from the key pair of scalar `0xC0..` to the key pair of scalar `0x0D..` under
/// [`DASHPAY_VECTOR_IV`], by the functions `create_contact_request` calls. Pinned so that a
/// change to either the contact request's encryption or the generic helper shows here; the
/// bytes were cross-checked against an independent secp256k1 ECDH and OpenSSL AES-256-CBC.
const DASHPAY_VECTOR_HEX: &str = "5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a5a864d1b3807cf80fd27df6cac063a5128\
                                  fd6119d0d40491a7788cb4e1975bc47e5070c7919e3d8c21ab26be1a763e7908\
                                  b97cd32a5e36309f3bf9535c519b1b32b2f206696ec6d0e244a2e182fceaa750";

fn key_pair(scalar: u8) -> (SecretKey, PublicKey) {
    let secret_key = SecretKey::from_slice(&[scalar; 32]).expect("a valid scalar");
    let public_key = PublicKey::from_secret_key(&Secp256k1::signing_only(), &secret_key);
    (secret_key, public_key)
}

fn dashpay_vector_xpub() -> Vec<u8> {
    let (_, account_key) = key_pair(0x07);
    compact_xpub_bytes(
        [0x11, 0x22, 0x33, 0x44],
        [0xAA; 32],
        account_key.serialize(),
    )
    .to_vec()
}

fn charters_contract() -> dpp::prelude::DataContract {
    load_system_data_contract(
        SystemDataContract::ModerationCharters,
        PlatformVersion::latest(),
    )
    .expect("the moderation charters contract loads at the latest version")
}

/// The dashpay `contactRequest` type with the declaration dashpay v2 is proposed to carry: key
/// indexes bounded to `u32` and `encryptedPublicKey` declared `encryptedFor` `toUserId`.
fn contact_request_declaring_encrypted_for() -> DocumentType {
    let platform_version = PlatformVersion::latest();
    let dashpay = load_system_data_contract(SystemDataContract::Dashpay, platform_version)
        .expect("the dashpay contract loads");
    let mut schema = dashpay
        .document_type_for_name("contactRequest")
        .expect("dashpay has contactRequest")
        .schema()
        .clone();
    for key_index in ["senderKeyIndex", "recipientKeyIndex"] {
        schema
            .set_value_at_full_path(
                &format!("properties.{key_index}.maximum"),
                Value::U64(u32::MAX as u64),
            )
            .expect("sets the bound");
    }
    schema
        .set_value_at_full_path(
            "properties.encryptedPublicKey.encryptedFor",
            platform_value!({
                "recipient": "toUserId",
                "recipientKey": "recipientKeyIndex",
                "senderKey": "senderKeyIndex",
                "scheme": "ecdh-secp256k1-aes256-cbc"
            }),
        )
        .expect("sets the declaration");
    DocumentType::try_from_schema(
        dashpay.id(),
        dashpay.system_version_type(),
        dashpay.config().version(),
        "contactRequest",
        schema,
        None,
        &BTreeMap::new(),
        dashpay.config(),
        false,
        &mut vec![],
        platform_version,
    )
    .expect("the declaring contactRequest parses")
}

/// Encrypts with a pinned IV, which only tests may do.
fn encrypt_property_with_iv(
    document_type: DocumentTypeRef<'_>,
    property_path: &str,
    plaintext: &[u8],
    keys: &EncryptionKeys<'_>,
    iv: &[u8; AES_CBC_IV_LENGTH],
    properties: &mut BTreeMap<String, Value>,
) -> Result<(), EncryptedForError> {
    let declaration = encrypted_for_declaration(document_type, property_path)?;
    encrypt_declared(&declaration, property_path, plaintext, keys, iv, properties)
}

/// The contract id of [`audited_message`].
const AUDITED_CONTRACT_ID: [u8; 32] = [4; 32];

/// A `message` type whose recipient key reference requires a purpose and no binding, with a
/// second, independent `identityPublicKey` reference on the sender key id: `auditIdentityId`
/// makes consensus check that the auditor has a key of that id, nothing more.
fn audited_message() -> DocumentType {
    audited_message_with(&[])
}

/// [`audited_message`] with the extra document type keywords `keywords`.
fn audited_message_with(keywords: &[(&str, Value)]) -> DocumentType {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version).expect("config");
    let identifier = |position: u32, refers_to: Value| {
        platform_value!({
            "type": "array",
            "byteArray": true,
            "minItems": 32,
            "maxItems": 32,
            "contentMediaType": "application/x.dash.dpp.identifier",
            "position": position,
            "refersTo": refers_to
        })
    };
    let mut schema = platform_value!({
        "type": "object",
        "properties": {
            "recipientId": identifier(0, platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "recipientKeyId",
                "keyRequirements": { "purpose": "decryption" }
            })),
            "recipientKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 1 },
            "senderKeyId": { "type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 2 },
            "auditIdentityId": identifier(3, platform_value!({
                "type": "identityPublicKey",
                "keyIdProperty": "senderKeyId"
            })),
            "body": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 4096,
                "position": 4,
                "encryptedFor": {
                    "recipient": "recipientId",
                    "recipientKey": "recipientKeyId",
                    "senderKey": "senderKeyId",
                    "scheme": "ecdh-secp256k1-aes256-cbc"
                }
            }
        },
        "additionalProperties": false
    });
    for (keyword, value) in keywords {
        schema
            .set_value_at_full_path(keyword, value.clone())
            .expect("sets the keyword");
    }
    DocumentType::try_from_schema(
        Identifier::from(AUDITED_CONTRACT_ID),
        1,
        config.version(),
        "message",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        false,
        &mut vec![],
        platform_version,
    )
    .expect("the message type parses")
}

/// A `note` whose body its writer encrypts to itself, with one property for both key ids.
fn note_keeping_both_key_ids_in_one_property() -> DocumentType {
    let platform_version = PlatformVersion::latest();
    let config = DataContractConfig::default_for_version(platform_version).expect("config");
    let schema = platform_value!({
        "type": "object",
        "properties": {
            "keyId": { "type": "integer", "minimum": 0, "maximum": 4294967295u64, "position": 0 },
            "body": {
                "type": "array",
                "byteArray": true,
                "minItems": 32,
                "maxItems": 4096,
                "position": 1,
                "encryptedFor": {
                    "recipient": "$ownerId",
                    "recipientKey": "keyId",
                    "senderKey": "keyId",
                    "scheme": "ecdh-secp256k1-aes256-cbc"
                }
            }
        },
        "additionalProperties": false
    });
    DocumentType::try_from_schema(
        Identifier::from([3; 32]),
        1,
        config.version(),
        "note",
        schema,
        None,
        &BTreeMap::new(),
        &config,
        false,
        &mut vec![],
        platform_version,
    )
    .expect("the note type parses")
}

fn key(
    id: KeyID,
    purpose: Purpose,
    bound_to: Option<&str>,
    public_key: &PublicKey,
) -> IdentityPublicKey {
    key_with_bounds(
        id,
        purpose,
        bound_to.map(
            |document_type_name| ContractBounds::SingleContractDocumentType {
                id: MODERATION_CHARTERS_CONTRACT_ID,
                document_type_name: document_type_name.to_string(),
            },
        ),
        public_key,
    )
}

fn key_with_bounds(
    id: KeyID,
    purpose: Purpose,
    contract_bounds: Option<ContractBounds>,
    public_key: &PublicKey,
) -> IdentityPublicKey {
    IdentityPublicKeyV0 {
        id,
        purpose,
        security_level: SecurityLevel::MEDIUM,
        contract_bounds,
        key_type: KeyType::ECDSA_SECP256K1,
        read_only: false,
        data: BinaryData::new(public_key.serialize().to_vec()),
        disabled_at: None,
    }
    .into()
}

fn identity(id: u8, keys: Vec<IdentityPublicKey>) -> Identity {
    IdentityV0 {
        id: Identifier::from([id; 32]),
        public_keys: keys.into_iter().map(|key| (key.id(), key)).collect(),
        balance: 0,
        revision: 0,
    }
    .into()
}

fn keys_between(sender_private_key: &SecretKey, recipient_scalar: u8) -> EncryptionKeys<'_> {
    let (_, recipient_public_key) = key_pair(recipient_scalar);
    EncryptionKeys {
        sender_key_id: 3,
        sender_private_key,
        recipient_key_id: 7,
        recipient_public_key,
    }
}

#[test]
fn should_decrypt_what_it_encrypts_and_fill_the_key_id_properties() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let (writer_private_key, _) = key_pair(0x21);
    let keys = keys_between(&writer_private_key, 0x42);
    let plaintext = b"I moderated a forum for five years and would like to help.";

    let mut properties = BTreeMap::new();
    encrypt_property(
        join_request,
        ENCRYPTED_MESSAGE,
        plaintext,
        &keys,
        &mut properties,
    )
    .expect("encrypts");

    assert_eq!(properties.get("recipientKeyId"), Some(&Value::U32(7)));
    assert_eq!(properties.get("senderKeyId"), Some(&Value::U32(3)));
    let (recipient_private_key, _) = key_pair(0x42);
    let (_, sender_public_key) = key_pair(0x21);
    let decrypted = decrypt_property(
        join_request,
        ENCRYPTED_MESSAGE,
        &properties,
        &recipient_private_key,
        &sender_public_key,
    )
    .expect("the recipient decrypts");
    assert_eq!(decrypted, plaintext);

    // ECDH is symmetric: the sender reads its own message back
    let (_, recipient_public_key) = key_pair(0x42);
    let (sender_private_key, _) = key_pair(0x21);
    let by_the_sender = decrypt_property(
        join_request,
        ENCRYPTED_MESSAGE,
        &properties,
        &sender_private_key,
        &recipient_public_key,
    )
    .expect("the sender decrypts");
    assert_eq!(by_the_sender, plaintext);
}

#[test]
fn should_decrypt_the_dashpay_contact_request_vector_with_the_generic_helper() {
    let (sender_private_key, sender_public_key) = key_pair(0xC0);
    let (recipient_private_key, recipient_public_key) = key_pair(0x0D);
    let xpub = dashpay_vector_xpub();

    // What `create_contact_request` writes into `encryptedPublicKey`
    let shared_key = derive_shared_key_ecdh(&sender_private_key, &recipient_public_key);
    let vector = encrypt_extended_public_key(&shared_key, &DASHPAY_VECTOR_IV, &xpub);
    assert_eq!(hex::encode(&vector), DASHPAY_VECTOR_HEX);

    let document_type = contact_request_declaring_encrypted_for();
    let properties = BTreeMap::from([
        (
            "toUserId".to_string(),
            Value::Identifier(Identifier::from([9; 32]).to_buffer()),
        ),
        (
            "encryptedPublicKey".to_string(),
            Value::Bytes(vector.clone()),
        ),
        ("senderKeyIndex".to_string(), Value::U32(2)),
        ("recipientKeyIndex".to_string(), Value::U32(1)),
        ("accountReference".to_string(), Value::U32(0)),
    ]);
    let decrypted = decrypt_property(
        document_type.as_ref(),
        "encryptedPublicKey",
        &properties,
        &recipient_private_key,
        &sender_public_key,
    )
    .expect("the generic helper decrypts the contact request");
    assert_eq!(decrypted, xpub);

    // And encrypting the same xpub under the same IV writes the same bytes and key ids
    let mut written = BTreeMap::new();
    encrypt_property_with_iv(
        document_type.as_ref(),
        "encryptedPublicKey",
        &xpub,
        &EncryptionKeys {
            sender_key_id: 2,
            sender_private_key: &sender_private_key,
            recipient_key_id: 1,
            recipient_public_key,
        },
        &DASHPAY_VECTOR_IV,
        &mut written,
    )
    .expect("encrypts");
    assert_eq!(
        written.get("encryptedPublicKey"),
        Some(&Value::Bytes(vector))
    );
    assert_eq!(written.get("senderKeyIndex"), Some(&Value::U32(2)));
    assert_eq!(written.get("recipientKeyIndex"), Some(&Value::U32(1)));
    let written_bytes = written["encryptedPublicKey"]
        .to_binary_bytes()
        .expect("bytes");
    assert_eq!(
        decrypt_extended_public_key(&shared_key, &written_bytes).expect("dashpay decrypts it"),
        xpub
    );
}

#[test]
fn should_write_an_iv_plus_whole_blocks_that_pass_the_consensus_shape_check() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let scheme = EncryptionScheme::EcdhSecp256k1Aes256Cbc;
    for plaintext_length in [0usize, 1, 15, 16, 17, 31, 32, 500, 1023] {
        let mut properties = BTreeMap::new();
        encrypt_property(
            join_request,
            ENCRYPTED_MESSAGE,
            &vec![0x61; plaintext_length],
            &keys_between(&key_pair(0x21).0, 0x42),
            &mut properties,
        )
        .expect("encrypts");
        let length = properties[ENCRYPTED_MESSAGE]
            .to_binary_bytes()
            .expect("bytes")
            .len();
        // PKCS7 always pads, so a whole block past the plaintext at most
        assert_eq!(
            length,
            16 + (plaintext_length / 16 + 1) * 16,
            "{plaintext_length} bytes"
        );
        assert!(scheme.is_valid_ciphertext_length(length));
        assert!(join_request
            .validate_encrypted_property_shapes(&properties, PlatformVersion::latest())
            .expect("the check runs")
            .is_valid());
    }
}

#[test]
fn should_fail_to_decrypt_with_a_wrong_key() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let mut properties = BTreeMap::new();
    encrypt_property_with_iv(
        join_request,
        ENCRYPTED_MESSAGE,
        b"only the leader reads this",
        &keys_between(&key_pair(0x21).0, 0x42),
        &[0x01; 16],
        &mut properties,
    )
    .expect("encrypts");

    let (_, sender_public_key) = key_pair(0x21);
    let (someone_else, _) = key_pair(0x43);
    assert_eq!(
        decrypt_property(
            join_request,
            ENCRYPTED_MESSAGE,
            &properties,
            &someone_else,
            &sender_public_key,
        ),
        Err(EncryptedForError::DecryptionFailed)
    );
    let (recipient_private_key, _) = key_pair(0x42);
    let (_, another_sender) = key_pair(0x22);
    assert_eq!(
        decrypt_property(
            join_request,
            ENCRYPTED_MESSAGE,
            &properties,
            &recipient_private_key,
            &another_sender,
        ),
        Err(EncryptedForError::DecryptionFailed)
    );
}

#[test]
fn should_refuse_bytes_of_the_wrong_shape_before_decrypting() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let (recipient_private_key, _) = key_pair(0x42);
    let (_, sender_public_key) = key_pair(0x21);
    for length in [0usize, 16, 31, 33, 47] {
        let properties =
            BTreeMap::from([(ENCRYPTED_MESSAGE.to_string(), Value::Bytes(vec![0; length]))]);
        assert_eq!(
            decrypt_property(
                join_request,
                ENCRYPTED_MESSAGE,
                &properties,
                &recipient_private_key,
                &sender_public_key,
            ),
            Err(EncryptedForError::InvalidCiphertextLength {
                path: ENCRYPTED_MESSAGE.to_string(),
                scheme: EncryptionScheme::EcdhSecp256k1Aes256Cbc,
                length,
            })
        );
    }
}

#[test]
fn should_refuse_a_property_that_declares_no_encrypted_for() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    assert_eq!(
        encrypt_property(
            join_request,
            "submittedCharterId",
            b"x",
            &keys_between(&key_pair(0x21).0, 0x42),
            &mut BTreeMap::new(),
        ),
        Err(EncryptedForError::NotDeclared {
            document_type: JOIN_REQUEST_DOCUMENT_TYPE_NAME.to_string(),
            property: "submittedCharterId".to_string(),
        })
    );
}

#[test]
fn should_pick_the_keys_the_key_requirements_demand() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let (writer_private_key, writer_public_key) = key_pair(0x21);
    let (_, unbound_writer_key) = key_pair(0x22);
    let writer = identity(
        1,
        vec![
            key(0, Purpose::AUTHENTICATION, None, &unbound_writer_key),
            key(
                4,
                Purpose::ENCRYPTION,
                Some(JOIN_REQUEST_DOCUMENT_TYPE_NAME),
                &writer_public_key,
            ),
        ],
    );
    let (_, leader_bound) = key_pair(0x42);
    let (_, leader_bound_disabled) = key_pair(0x43);
    let (_, leader_unbound) = key_pair(0x44);
    let (_, leader_bound_elsewhere) = key_pair(0x45);
    let mut disabled = key(
        9,
        Purpose::DECRYPTION,
        Some(SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME),
        &leader_bound_disabled,
    );
    if let IdentityPublicKey::V0(v0) = &mut disabled {
        v0.disabled_at = Some(1);
    }
    let leader = identity(
        2,
        vec![
            key(
                2,
                Purpose::DECRYPTION,
                Some(SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME),
                &leader_bound,
            ),
            key(3, Purpose::DECRYPTION, None, &leader_unbound),
            key(
                5,
                Purpose::DECRYPTION,
                Some(JOIN_REQUEST_DOCUMENT_TYPE_NAME),
                &leader_bound_elsewhere,
            ),
            disabled,
        ],
    );

    let keys = select_encryption_keys(
        join_request,
        ENCRYPTED_MESSAGE,
        &writer,
        &writer_private_key,
        &leader,
    )
    .expect("keys are found");
    assert_eq!(keys.sender_key_id, 4);
    assert_eq!(keys.recipient_key_id, 2);
    assert_eq!(keys.recipient_public_key, leader_bound);

    // A writer key that is not bound to joinRequest is refused, as consensus would
    let (other_private_key, other_public_key) = key_pair(0x23);
    let unbound_writer = identity(
        1,
        vec![key(4, Purpose::ENCRYPTION, None, &other_public_key)],
    );
    assert!(matches!(
        select_encryption_keys(
            join_request,
            ENCRYPTED_MESSAGE,
            &unbound_writer,
            &other_private_key,
            &leader,
        ),
        Err(EncryptedForError::NoSuitableKey {
            role: EncryptionKeyRole::Sender,
            ..
        })
    ));
    // A private key that is none of the writer's keys is refused
    let (stranger, _) = key_pair(0x77);
    assert!(matches!(
        select_encryption_keys(join_request, ENCRYPTED_MESSAGE, &writer, &stranger, &leader),
        Err(EncryptedForError::NoSuitableKey {
            role: EncryptionKeyRole::Sender,
            ..
        })
    ));
    // A leader without a decryption key bound to submittedCharter cannot be written to
    let leader_without = identity(2, vec![key(3, Purpose::DECRYPTION, None, &leader_unbound)]);
    assert!(matches!(
        select_encryption_keys(
            join_request,
            ENCRYPTED_MESSAGE,
            &writer,
            &writer_private_key,
            &leader_without,
        ),
        Err(EncryptedForError::NoSuitableKey {
            role: EncryptionKeyRole::Recipient,
            ..
        })
    ));
}

#[test]
fn should_write_the_recipient_and_read_the_envelope_back() {
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let (writer_private_key, writer_public_key) = key_pair(0x21);
    let writer = identity(
        1,
        vec![key(
            4,
            Purpose::ENCRYPTION,
            Some(JOIN_REQUEST_DOCUMENT_TYPE_NAME),
            &writer_public_key,
        )],
    );
    let (leader_private_key, leader_public_key) = key_pair(0x42);
    let leader = identity(
        2,
        vec![key(
            2,
            Purpose::DECRYPTION,
            Some(SUBMITTED_CHARTER_DOCUMENT_TYPE_NAME),
            &leader_public_key,
        )],
    );

    let mut properties = BTreeMap::new();
    encrypt_property_for(
        join_request,
        ENCRYPTED_MESSAGE,
        b"hello",
        &writer,
        &writer_private_key,
        &leader,
        &mut properties,
    )
    .expect("encrypts");
    assert_eq!(
        properties.get("recipientId"),
        Some(&Value::Identifier(leader.id().to_buffer()))
    );

    let document: Document = DocumentV0 {
        id: Identifier::from([5; 32]),
        owner_id: writer.id(),
        properties,
        ..Default::default()
    }
    .into();
    let envelope =
        EncryptedPropertyEnvelope::read(join_request, ENCRYPTED_MESSAGE, &document).expect("reads");
    assert_eq!(
        envelope,
        EncryptedPropertyEnvelope {
            recipient_id: leader.id(),
            recipient_key_id: 2,
            sender_id: writer.id(),
            sender_key_id: 4,
        }
    );
    assert_eq!(
        decrypt_property(
            join_request,
            ENCRYPTED_MESSAGE,
            document.properties(),
            &leader_private_key,
            &writer_public_key,
        )
        .expect("the leader decrypts"),
        b"hello"
    );
}

#[test]
fn should_refuse_a_ciphertext_that_is_not_bytes() {
    // A document built without its contract holds a byte array as a list of numbers; the wasm
    // bindings sanitize it against the document type before calling here
    let contract = charters_contract();
    let join_request = contract
        .document_type_for_name(JOIN_REQUEST_DOCUMENT_TYPE_NAME)
        .expect("joinRequest exists");
    let properties = BTreeMap::from([(
        ENCRYPTED_MESSAGE.to_string(),
        Value::Array(vec![Value::U64(1); 32]),
    )]);
    let (recipient_private_key, _) = key_pair(0x42);
    let (_, sender_public_key) = key_pair(0x21);
    assert!(matches!(
        decrypt_property(
            join_request,
            ENCRYPTED_MESSAGE,
            &properties,
            &recipient_private_key,
            &sender_public_key,
        ),
        Err(EncryptedForError::InvalidProperty { .. })
    ));
}

#[test]
fn should_encrypt_under_one_key_when_one_property_keeps_both_key_ids() {
    let note = note_keeping_both_key_ids_in_one_property();
    let (writer_private_key, writer_public_key) = key_pair(0x21);
    let (_, decryption_public_key) = key_pair(0x22);
    let writer = identity(
        1,
        vec![
            key(1, Purpose::ENCRYPTION, None, &writer_public_key),
            key(2, Purpose::DECRYPTION, None, &decryption_public_key),
        ],
    );

    // The writer's decryption key would be picked for a separate recipient key property; with
    // one property for both, the one key is the writer's own
    let mut properties = BTreeMap::new();
    let keys = encrypt_property_for(
        note.as_ref(),
        "body",
        b"note to self",
        &writer,
        &writer_private_key,
        &writer,
        &mut properties,
    )
    .expect("encrypts");
    assert_eq!((keys.sender_key_id, keys.recipient_key_id), (1, 1));
    assert_eq!(properties.get("keyId"), Some(&Value::U32(1)));
    assert_eq!(
        decrypt_property(
            note.as_ref(),
            "body",
            &properties,
            &writer_private_key,
            &writer_public_key,
        )
        .expect("the key the document names decrypts"),
        b"note to self"
    );

    // Two different key ids for the one property are refused rather than one overwritten
    let mismatched = EncryptionKeys {
        sender_key_id: 1,
        sender_private_key: &writer_private_key,
        recipient_key_id: 2,
        recipient_public_key: decryption_public_key,
    };
    assert!(matches!(
        encrypt_property(
            note.as_ref(),
            "body",
            b"x",
            &mismatched,
            &mut BTreeMap::new()
        ),
        Err(EncryptedForError::SharedKeyIdProperty { .. })
    ));
}

#[test]
fn should_skip_keys_bound_to_another_scope_when_no_bound_to_is_required() {
    // No keyRequirements at all: the dashpay-shaped contactRequest declaration
    let contact_request = contact_request_declaring_encrypted_for();
    // Purpose-only keyRequirements: the audited message
    let message = audited_message();

    let (writer_private_key, writer_public_key) = key_pair(0x21);
    let writer = identity(
        1,
        vec![key(4, Purpose::ENCRYPTION, None, &writer_public_key)],
    );
    let public = |scalar| key_pair(scalar).1;

    for (document_type, property) in [
        (contact_request.as_ref(), "encryptedPublicKey"),
        (message.as_ref(), "body"),
    ] {
        let contract_id = document_type.data_contract_id();
        let elsewhere = ContractBounds::SingleContract {
            id: MODERATION_CHARTERS_CONTRACT_ID,
        };
        let other_type_here = ContractBounds::SingleContractDocumentType {
            id: contract_id,
            document_type_name: "somethingElse".to_string(),
        };
        let recipient = identity(
            2,
            vec![
                key_with_bounds(
                    9,
                    Purpose::DECRYPTION,
                    Some(elsewhere.clone()),
                    &public(0x49),
                ),
                key_with_bounds(8, Purpose::DECRYPTION, Some(other_type_here), &public(0x48)),
                key_with_bounds(
                    7,
                    Purpose::DECRYPTION,
                    Some(ContractBounds::ContractGroup { id: contract_id }),
                    &public(0x47),
                ),
                key_with_bounds(2, Purpose::DECRYPTION, None, &public(0x42)),
            ],
        );
        let keys = select_encryption_keys(
            document_type,
            property,
            &writer,
            &writer_private_key,
            &recipient,
        )
        .expect("an unbound key is in scope");
        assert_eq!(
            keys.recipient_key_id,
            2,
            "{} passes over keys bound elsewhere",
            document_type.name()
        );

        // A key bound to this contract, or to this very document type, is in scope
        let mut in_scope = recipient.clone();
        for (id, bounds) in [
            (5, ContractBounds::SingleContract { id: contract_id }),
            (
                6,
                ContractBounds::SingleContractDocumentType {
                    id: contract_id,
                    document_type_name: document_type.name().clone(),
                },
            ),
        ] {
            let Identity::V0(IdentityV0 { public_keys, .. }) = &mut in_scope;
            public_keys.insert(
                id,
                key_with_bounds(
                    id,
                    Purpose::DECRYPTION,
                    Some(bounds),
                    &public(0x50 + id as u8),
                ),
            );
        }
        let keys = select_encryption_keys(
            document_type,
            property,
            &writer,
            &writer_private_key,
            &in_scope,
        )
        .expect("keys are found");
        assert_eq!(keys.recipient_key_id, 6);

        // A writer key bound to another contract is refused as the sender key
        let (bound_private_key, bound_public_key) = key_pair(0x23);
        let bound_writer = identity(
            1,
            vec![key_with_bounds(
                4,
                Purpose::ENCRYPTION,
                Some(elsewhere),
                &bound_public_key,
            )],
        );
        assert!(matches!(
            select_encryption_keys(
                document_type,
                property,
                &bound_writer,
                &bound_private_key,
                &recipient,
            ),
            Err(EncryptedForError::NoSuitableKey {
                role: EncryptionKeyRole::Sender,
                ..
            })
        ));
    }
}

#[test]
fn should_name_the_owner_as_the_sender_whatever_else_refers_to_the_sender_key() {
    let message = audited_message();
    let (writer_private_key, writer_public_key) = key_pair(0x21);
    let writer = identity(
        1,
        vec![key(4, Purpose::ENCRYPTION, None, &writer_public_key)],
    );
    let (leader_private_key, leader_public_key) = key_pair(0x42);
    let leader = identity(
        2,
        vec![key(2, Purpose::DECRYPTION, None, &leader_public_key)],
    );
    let auditor = Identifier::from([0xAD; 32]);

    let mut properties = BTreeMap::from([(
        "auditIdentityId".to_string(),
        Value::Identifier(auditor.to_buffer()),
    )]);
    encrypt_property_for(
        message.as_ref(),
        "body",
        b"for the leader",
        &writer,
        &writer_private_key,
        &leader,
        &mut properties,
    )
    .expect("encrypts");
    // A creator other than the owner does not make the creator the sender either
    let document: Document = DocumentV0 {
        id: Identifier::from([5; 32]),
        owner_id: writer.id(),
        creator_id: Some(Identifier::from([0xC0; 32])),
        properties,
        ..Default::default()
    }
    .into();

    let envelope =
        EncryptedPropertyEnvelope::read(message.as_ref(), "body", &document).expect("reads");
    assert_eq!(envelope.sender_id, writer.id());
    assert_eq!(envelope.recipient_id, leader.id());
    assert_eq!(
        decrypt_property(
            message.as_ref(),
            "body",
            document.properties(),
            &leader_private_key,
            &writer_public_key,
        )
        .expect("the owner's key is the sender key"),
        b"for the leader"
    );
}

#[test]
fn should_refuse_to_name_a_sender_once_the_owner_may_have_changed() {
    let (writer_private_key, writer_public_key) = key_pair(0x21);
    let writer = identity(
        1,
        vec![key(4, Purpose::ENCRYPTION, None, &writer_public_key)],
    );
    let (_, leader_public_key) = key_pair(0x42);
    let leader = identity(
        2,
        vec![key(2, Purpose::DECRYPTION, None, &leader_public_key)],
    );
    let written = |document_type: &DocumentType| -> BTreeMap<String, Value> {
        let mut properties =
            BTreeMap::from([("auditIdentityId".to_string(), Value::Identifier([0xAD; 32]))]);
        encrypt_property_for(
            document_type.as_ref(),
            "body",
            b"x",
            &writer,
            &writer_private_key,
            &leader,
            &mut properties,
        )
        .expect("encrypts");
        properties
    };
    let refused = |document_type: &DocumentType, document: &Document| {
        matches!(
            EncryptedPropertyEnvelope::read(document_type.as_ref(), "body", document),
            Err(EncryptedForError::SenderUnknownAfterTransfer { .. })
        )
    };

    // A transfer time on the document: its owner changed after the bytes were written
    let message = audited_message();
    let transferred: Document = DocumentV0 {
        id: Identifier::from([5; 32]),
        owner_id: Identifier::from([0x0E; 32]),
        properties: written(&message),
        transferred_at: Some(1),
        ..Default::default()
    }
    .into();
    assert!(refused(&message, &transferred));

    // A transferable type that records no transfer time: a transfer cannot be ruled out
    let transferable = audited_message_with(&[("transferable", Value::U8(1))]);
    let document: Document = DocumentV0 {
        id: Identifier::from([6; 32]),
        owner_id: writer.id(),
        properties: written(&transferable),
        ..Default::default()
    }
    .into();
    assert!(refused(&transferable, &document));

    // One that records transfer times and carries none still has its writer as its owner
    let recorded = audited_message_with(&[
        ("transferable", Value::U8(1)),
        (
            "required",
            Value::Array(vec![Value::Text("$transferredAt".to_string())]),
        ),
    ]);
    let untransferred: Document = DocumentV0 {
        id: Identifier::from([7; 32]),
        owner_id: writer.id(),
        properties: written(&recorded),
        ..Default::default()
    }
    .into();
    assert_eq!(
        EncryptedPropertyEnvelope::read(recorded.as_ref(), "body", &untransferred)
            .expect("reads")
            .sender_id,
        writer.id()
    );
}
