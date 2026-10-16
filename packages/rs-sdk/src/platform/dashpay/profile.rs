//! DashPay profile create, update and fetch
//!
//! Implements the DIP-15 `profile` document. Avatar image hashing is the
//! caller's job: [`ProfileFields`] carries the precomputed SHA-256 `avatarHash`
//! and dHash `avatarFingerprint`, so the SDK needs no image decoder.

use crate::platform::documents::document_query::DocumentQuery;
use crate::platform::documents::transitions::{
    DocumentCreateResult, DocumentCreateTransitionBuilder, DocumentReplaceResult,
    DocumentReplaceTransitionBuilder,
};
use crate::platform::FetchMany;
use crate::{Error, Sdk};
use dpp::dashcore::secp256k1::rand::rngs::StdRng;
use dpp::dashcore::secp256k1::rand::{Rng, SeedableRng};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::Identifier;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use std::collections::BTreeMap;
use std::sync::Arc;

const PROFILE_DOCUMENT_TYPE: &str = "profile";

/// Most owners one [`Sdk::get_profiles`] call can look up: the Platform `In`
/// clause cardinality cap, which is also the document-query limit.
pub const MAX_PROFILE_OWNERS_PER_QUERY: usize = 100;

/// The profile fields a caller sets. `None` leaves the field alone on
/// [`Sdk::update_profile`] and omits it on [`Sdk::create_profile`].
///
/// `avatar_hash` and `avatar_fingerprint` are computed by the caller from the
/// avatar image bytes and set together with a new `avatar_url`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileFields {
    /// Display name (max 25 chars per DIP-15)
    pub display_name: Option<String>,
    /// Public message, the bio (max 140 chars per DIP-15)
    pub public_message: Option<String>,
    /// Avatar URL (HTTPS, max 2048 chars per DIP-15)
    pub avatar_url: Option<String>,
    /// SHA-256 hash of the avatar image bytes
    pub avatar_hash: Option<[u8; 32]>,
    /// dHash perceptual fingerprint of the avatar image
    pub avatar_fingerprint: Option<[u8; 8]>,
}

impl ProfileFields {
    /// Overlay the fields that are set onto `properties`, leaving the rest as
    /// they are.
    fn overlay(self, mut properties: BTreeMap<String, Value>) -> BTreeMap<String, Value> {
        if let Some(name) = self.display_name {
            properties.insert("displayName".to_string(), Value::Text(name));
        }
        if let Some(message) = self.public_message {
            properties.insert("publicMessage".to_string(), Value::Text(message));
        }
        if let Some(url) = self.avatar_url {
            properties.insert("avatarUrl".to_string(), Value::Text(url));
        }
        if let Some(hash) = self.avatar_hash {
            properties.insert("avatarHash".to_string(), Value::Bytes32(hash));
        }
        if let Some(fingerprint) = self.avatar_fingerprint {
            properties.insert(
                "avatarFingerprint".to_string(),
                Value::Bytes(fingerprint.to_vec()),
            );
        }
        properties
    }
}

impl Sdk {
    /// Fetch the `profile` documents of `owner_ids` (unique, at most
    /// [`MAX_PROFILE_OWNERS_PER_QUERY`]), keyed by owner id. An owner without a
    /// profile is missing from the result. More than that many owners, or a
    /// duplicated owner, is rejected with [`Error::Generic`] before any query.
    ///
    /// `dashpay_contract` is the DashPay data contract the caller resolved.
    pub async fn get_profiles(
        &self,
        dashpay_contract: &Arc<DataContract>,
        owner_ids: &[Identifier],
    ) -> Result<BTreeMap<Identifier, Document>, Error> {
        ensure_valid_profile_owners(owner_ids)?;
        if owner_ids.is_empty() {
            return Ok(BTreeMap::new());
        }
        self.fetch_profile_documents(profiles_query(dashpay_contract, owner_ids))
            .await
    }

    /// Create the profile document of `owner_id` from `fields`.
    ///
    /// `signing_key` must be a HIGH or CRITICAL authentication key of the owner.
    /// Returns the document as confirmed by Platform.
    pub async fn create_profile<S: Signer<IdentityPublicKey>>(
        &self,
        dashpay_contract: &Arc<DataContract>,
        owner_id: Identifier,
        fields: ProfileFields,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<Document, Error> {
        let properties = sanitized_properties(dashpay_contract, fields.overlay(BTreeMap::new()))?;

        let entropy: [u8; 32] = StdRng::from_os_rng().random();
        let id = Document::generate_document_id_v0(
            &dashpay_contract.id(),
            &owner_id,
            PROFILE_DOCUMENT_TYPE,
            entropy.as_slice(),
        );

        let builder = DocumentCreateTransitionBuilder::new(
            Arc::clone(dashpay_contract),
            PROFILE_DOCUMENT_TYPE.to_string(),
            profile_document(id, owner_id, properties, None),
            entropy,
        );
        let DocumentCreateResult::Document(document) =
            self.document_create(builder, signing_key, signer).await?;
        Ok(document)
    }

    /// Update the existing profile document of `owner_id`: read it, overlay the
    /// `fields` that are set so a partial update keeps the other fields,
    /// increment the revision and replace it.
    ///
    /// `signing_key` must be a HIGH or CRITICAL authentication key of the owner.
    /// Returns the merged document as confirmed by Platform. Fails if the owner
    /// has no profile.
    pub async fn update_profile<S: Signer<IdentityPublicKey>>(
        &self,
        dashpay_contract: &Arc<DataContract>,
        owner_id: Identifier,
        fields: ProfileFields,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<Document, Error> {
        let existing = self
            .fetch_profile_documents(owner_profile_query(dashpay_contract, owner_id))
            .await?
            .remove(&owner_id)
            .ok_or_else(|| {
                Error::Generic("No existing profile document found to update".to_string())
            })?;

        let builder = DocumentReplaceTransitionBuilder::new(
            Arc::clone(dashpay_contract),
            PROFILE_DOCUMENT_TYPE.to_string(),
            replacement_document(&existing, fields, dashpay_contract)?,
        );
        let DocumentReplaceResult::Document(document) =
            self.document_replace(builder, signing_key, signer).await?;
        Ok(document)
    }

    async fn fetch_profile_documents(
        &self,
        query: DocumentQuery,
    ) -> Result<BTreeMap<Identifier, Document>, Error> {
        Ok(Document::fetch_many(self, query)
            .await?
            .into_values()
            .flatten()
            .map(|document| (document.owner_id(), document))
            .collect())
    }
}

/// Refuse owner lists Platform would reject: more than
/// [`MAX_PROFILE_OWNERS_PER_QUERY`] owners, or a duplicated owner.
fn ensure_valid_profile_owners(owner_ids: &[Identifier]) -> Result<(), Error> {
    if owner_ids.len() > MAX_PROFILE_OWNERS_PER_QUERY {
        return Err(Error::Generic(format!(
            "at most {MAX_PROFILE_OWNERS_PER_QUERY} profile owners per query, got {}",
            owner_ids.len()
        )));
    }
    let mut seen = std::collections::BTreeSet::new();
    if let Some(duplicate) = owner_ids.iter().find(|id| !seen.insert(**id)) {
        return Err(Error::Generic(format!(
            "duplicate profile owner {duplicate} in query"
        )));
    }
    Ok(())
}

/// The replacement of `existing`: same id and owner, `fields` overlaid on its
/// properties so a partial update keeps the others, revision incremented.
fn replacement_document(
    existing: &Document,
    fields: ProfileFields,
    dashpay_contract: &DataContract,
) -> Result<Document, Error> {
    let properties = sanitized_properties(
        dashpay_contract,
        fields.overlay(existing.properties().clone()),
    )?;
    let revision = existing.revision().unwrap_or(INITIAL_REVISION) + 1;
    Ok(profile_document(
        existing.id(),
        existing.owner_id(),
        properties,
        Some(revision),
    ))
}

/// Normalize `properties` to the representations the profile schema expects.
fn sanitized_properties(
    dashpay_contract: &DataContract,
    mut properties: BTreeMap<String, Value>,
) -> Result<BTreeMap<String, Value>, Error> {
    dashpay_contract
        .document_type_for_name(PROFILE_DOCUMENT_TYPE)
        .map_err(|e| Error::Protocol(e.into()))?
        .sanitize_document_properties(&mut properties);
    Ok(properties)
}

fn profile_document(
    id: Identifier,
    owner_id: Identifier,
    properties: BTreeMap<String, Value>,
    revision: Option<u64>,
) -> Document {
    Document::V0(DocumentV0 {
        id,
        owner_id,
        properties,
        revision,
        ..Default::default()
    })
}

/// Build the `$ownerId In [owner_ids]` profile query.
///
/// `In` is a range operator, so DAPI requires a matching `orderBy` on the
/// range field or it rejects the query with "missing order by for range". The
/// `$ownerId` index is unique, so ordering does not change the result set
/// (at most one profile per owner).
fn profiles_query(dashpay_contract: &Arc<DataContract>, owner_ids: &[Identifier]) -> DocumentQuery {
    DocumentQuery {
        select: drive::query::SelectProjection::documents(),
        data_contract: Arc::clone(dashpay_contract),
        document_type_name: PROFILE_DOCUMENT_TYPE.to_string(),
        where_clauses: vec![WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::In,
            value: Value::Array(owner_ids.iter().map(|id| platform_value!(id)).collect()),
        }],
        time_range_clauses: vec![],
        integer_range_clauses: vec![],
        sub_queries: vec![],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![OrderClause {
            field: "$ownerId".to_string(),
            ascending: true,
        }],
        limit: MAX_PROFILE_OWNERS_PER_QUERY as u32,
        offset: None,
        start: None,
    }
}

/// Build the single-owner `profile` query `$ownerId == owner_id` (limit 1: a
/// profile is unique per owner).
fn owner_profile_query(
    dashpay_contract: &Arc<DataContract>,
    owner_id: Identifier,
) -> DocumentQuery {
    DocumentQuery {
        select: drive::query::SelectProjection::documents(),
        data_contract: Arc::clone(dashpay_contract),
        document_type_name: PROFILE_DOCUMENT_TYPE.to_string(),
        where_clauses: vec![WhereClause {
            field: "$ownerId".to_string(),
            operator: WhereOperator::Equal,
            value: platform_value!(owner_id),
        }],
        time_range_clauses: vec![],
        integer_range_clauses: vec![],
        sub_queries: vec![],
        group_by: vec![],
        having: vec![],
        order_by_clauses: vec![],
        limit: 1,
        offset: None,
        start: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    fn existing_full() -> BTreeMap<String, Value> {
        ProfileFields {
            display_name: Some("Alice".into()),
            public_message: Some("hello world".into()),
            avatar_url: Some("https://x/a.png".into()),
            avatar_hash: Some([7u8; 32]),
            avatar_fingerprint: Some([1, 2, 3, 4, 5, 6, 7, 8]),
        }
        .overlay(BTreeMap::new())
    }

    fn text<'a>(properties: &'a BTreeMap<String, Value>, key: &str) -> Option<&'a str> {
        properties.get(key).and_then(|v| v.as_str())
    }

    fn dashpay_contract() -> DataContract {
        load_system_data_contract(
            dpp::data_contracts::SystemDataContract::Dashpay,
            PlatformVersion::latest(),
        )
        .expect("DashPay system contract loads")
    }

    /// A partial update (only `displayName`) yields a replacement of the same
    /// document with the sibling fields kept and the revision incremented.
    #[test]
    fn partial_update_replaces_the_same_document_keeping_sibling_fields() {
        let (id, owner) = (Identifier::new([0xD1; 32]), Identifier::new([0x0E; 32]));
        let existing = profile_document(id, owner, existing_full(), Some(3));
        let fields = ProfileFields {
            display_name: Some("Alice 2".to_string()),
            ..Default::default()
        };

        let replacement = replacement_document(&existing, fields, &dashpay_contract()).unwrap();

        assert_eq!(replacement.id(), id);
        assert_eq!(replacement.owner_id(), owner);
        assert_eq!(replacement.revision(), Some(4));
        let merged = replacement.properties();
        assert_eq!(text(merged, "displayName"), Some("Alice 2"));
        assert_eq!(text(merged, "publicMessage"), Some("hello world"));
        assert_eq!(text(merged, "avatarUrl"), Some("https://x/a.png"));
        assert_eq!(merged.get("avatarHash"), Some(&Value::Bytes32([7u8; 32])));
        assert_eq!(
            merged.get("avatarFingerprint"),
            Some(&Value::Bytes(vec![1, 2, 3, 4, 5, 6, 7, 8]))
        );
    }

    /// `get_profiles` refuses too many owners and duplicated owners locally.
    #[test]
    fn profile_owners_over_the_cap_or_duplicated_are_rejected() {
        let ids = |n: usize| -> Vec<Identifier> {
            (0..n).map(|i| Identifier::new([i as u8 + 1; 32])).collect()
        };

        assert!(ensure_valid_profile_owners(&[]).is_ok());
        assert!(ensure_valid_profile_owners(&ids(MAX_PROFILE_OWNERS_PER_QUERY)).is_ok());
        assert!(matches!(
            ensure_valid_profile_owners(&ids(MAX_PROFILE_OWNERS_PER_QUERY + 1)),
            Err(Error::Generic(_))
        ));

        let mut duplicated = ids(2);
        duplicated.push(duplicated[0]);
        assert!(matches!(
            ensure_valid_profile_owners(&duplicated),
            Err(Error::Generic(_))
        ));
    }

    /// A supplied avatar hash and fingerprint replace the existing ones.
    #[test]
    fn avatar_replaced_when_supplied() {
        let replaced = ProfileFields {
            avatar_hash: Some([9u8; 32]),
            avatar_fingerprint: Some([9u8; 8]),
            ..Default::default()
        }
        .overlay(existing_full());
        assert_eq!(replaced.get("avatarHash"), Some(&Value::Bytes32([9u8; 32])));
        assert_eq!(
            replaced.get("avatarFingerprint"),
            Some(&Value::Bytes(vec![9u8; 8]))
        );
    }

    /// DAPI rejects any query that uses a range where-operator without a
    /// matching `orderBy` on the range field ("missing order by for range").
    /// The profile query filters by `$ownerId In [...]`, a range operator, so
    /// every range clause it builds must carry an `orderBy` on its field.
    /// Guarded against vacuous truth: the query must actually contain a range
    /// clause, otherwise the invariant would pass trivially.
    #[test]
    fn profiles_query_orders_by_every_range_field() {
        let contract = Arc::new(dashpay_contract());
        let owners = vec![Identifier::new([1u8; 32]), Identifier::new([2u8; 32])];

        let query = profiles_query(&contract, &owners);

        assert!(
            query.where_clauses.iter().any(|wc| wc.operator.is_range()),
            "expected a range where-clause (e.g. $ownerId In [...])"
        );
        for wc in &query.where_clauses {
            if wc.operator.is_range() {
                assert!(
                    query.order_by_clauses.iter().any(|oc| oc.field == wc.field),
                    "range clause on `{}` has no matching orderBy; DAPI rejects \
                     this with \"missing order by for range\"",
                    wc.field
                );
            }
        }
    }
}
