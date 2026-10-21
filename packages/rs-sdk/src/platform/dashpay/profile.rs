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
use dpp::dashcore::key::rand::rngs::StdRng;
use dpp::dashcore::key::rand::{Rng, SeedableRng};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::data_contract::DataContract;
use dpp::document::{Document, DocumentV0, DocumentV0Getters, INITIAL_REVISION};
use dpp::identity::signer::Signer;
use dpp::identity::IdentityPublicKey;
use dpp::platform_value::{platform_value, Value};
use dpp::prelude::Identifier;
use drive::query::{OrderClause, WhereClause, WhereOperator};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

const PROFILE_DOCUMENT_TYPE: &str = "profile";

/// Most owners one profile query can carry: the Platform `In` clause
/// cardinality cap, which is also the document-query limit.
pub(crate) const MAX_PROFILE_OWNERS_PER_QUERY: usize = 100;

/// The profile fields a caller sets. `None` leaves the field alone on
/// [`Sdk::update_profile`] and omits it on [`Sdk::create_profile`].
///
/// `avatar_hash` and `avatar_fingerprint` are computed by the caller from the
/// avatar image bytes. An update that changes `avatar_url` must set both hashes
/// with it, and an update sets the two hashes together. A field cannot be
/// cleared.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProfileFields {
    /// Display name (1 to 25 chars per DIP-15)
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
    /// Fetch the `profile` documents of `owner_ids` (unique), keyed by owner
    /// id, in queries of at most 100 owners each. An owner without a profile
    /// is missing from the result. A duplicated owner is rejected with
    /// [`Error::Generic`] before any query.
    ///
    /// `dashpay_contract` must be the DashPay system data contract.
    pub async fn get_profiles(
        &self,
        dashpay_contract: &Arc<DataContract>,
        owner_ids: &[Identifier],
    ) -> Result<BTreeMap<Identifier, Document>, Error> {
        self.ensure_dashpay_contract(dashpay_contract)?;
        ensure_unique_owners(owner_ids)?;
        let mut profiles = BTreeMap::new();
        for chunk in owner_ids.chunks(MAX_PROFILE_OWNERS_PER_QUERY) {
            profiles.extend(
                self.fetch_profile_documents(profiles_query(dashpay_contract, chunk)?)
                    .await?,
            );
        }
        Ok(profiles)
    }

    /// Create the profile document of `owner_id` from `fields`.
    ///
    /// `signing_key` must be a HIGH or CRITICAL authentication key of the
    /// owner. Returns the document as confirmed by Platform.
    pub async fn create_profile<S: Signer<IdentityPublicKey>>(
        &self,
        dashpay_contract: &Arc<DataContract>,
        owner_id: Identifier,
        fields: ProfileFields,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<Document, Error> {
        self.ensure_dashpay_contract(dashpay_contract)?;
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
    /// The two avatar hashes are set together, and a changed `avatar_url` comes
    /// with both hashes; otherwise the call fails before anything is signed. An
    /// unchanged `avatar_url` may be sent without hashes, which keeps the
    /// existing ones.
    ///
    /// `signing_key` must be a HIGH or CRITICAL authentication key of the
    /// owner. Returns the merged document as confirmed by Platform. Fails if
    /// the owner has no profile.
    pub async fn update_profile<S: Signer<IdentityPublicKey>>(
        &self,
        dashpay_contract: &Arc<DataContract>,
        owner_id: Identifier,
        fields: ProfileFields,
        signing_key: &IdentityPublicKey,
        signer: &S,
    ) -> Result<Document, Error> {
        let existing = self
            .get_profiles(dashpay_contract, &[owner_id])
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

    /// The profile queries and transitions are built against the contract the
    /// caller passes in, so it must be the DashPay system contract.
    fn ensure_dashpay_contract(&self, dashpay_contract: &DataContract) -> Result<(), Error> {
        let expected = self.get_dashpay_contract_id()?;
        if dashpay_contract.id() != expected {
            return Err(Error::Generic(format!(
                "expected the DashPay contract {expected}, got {}",
                dashpay_contract.id()
            )));
        }
        Ok(())
    }
}

/// Refuse a duplicated owner, which Platform rejects in an `In` clause.
fn ensure_unique_owners(owner_ids: &[Identifier]) -> Result<(), Error> {
    let mut seen = BTreeSet::new();
    if let Some(duplicate) = owner_ids.iter().find(|id| !seen.insert(**id)) {
        return Err(Error::Generic(format!(
            "duplicate profile owner {duplicate} in query"
        )));
    }
    Ok(())
}

/// The replacement of `existing`: same id and owner, `fields` overlaid on its
/// properties so a partial update keeps the others, revision incremented.
///
/// Refuses two patches the schema cannot see on the merged document, because
/// the stored avatar fills the gap, and that would leave a hash of a different
/// image under the avatar: one of the two hashes alone, and a changed
/// `avatar_url` without both.
fn replacement_document(
    existing: &Document,
    fields: ProfileFields,
    dashpay_contract: &DataContract,
) -> Result<Document, Error> {
    if fields.avatar_hash.is_some() != fields.avatar_fingerprint.is_some() {
        return Err(Error::Generic(
            "avatarHash and avatarFingerprint must be set together".to_string(),
        ));
    }
    let url_changed = fields.avatar_url.as_deref().is_some_and(|url| {
        existing
            .properties()
            .get("avatarUrl")
            .and_then(Value::as_str)
            != Some(url)
    });
    if url_changed && fields.avatar_hash.is_none() {
        return Err(Error::Generic(
            "a changed avatarUrl needs its avatarHash and avatarFingerprint".to_string(),
        ));
    }
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
fn profiles_query(
    dashpay_contract: &Arc<DataContract>,
    owner_ids: &[Identifier],
) -> Result<DocumentQuery, Error> {
    Ok(
        DocumentQuery::new(Arc::clone(dashpay_contract), PROFILE_DOCUMENT_TYPE)?
            .with_where(WhereClause {
                field: "$ownerId".to_string(),
                operator: WhereOperator::In,
                value: Value::Array(owner_ids.iter().map(|id| platform_value!(id)).collect()),
            })
            .with_order_by(OrderClause {
                field: "$ownerId".to_string(),
                ascending: true,
            })
            .with_limit(MAX_PROFILE_OWNERS_PER_QUERY as u32),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use dpp::system_data_contracts::load_system_data_contract;
    use dpp::version::PlatformVersion;

    fn avatar_fields() -> ProfileFields {
        ProfileFields {
            avatar_url: Some("https://x/a.png".into()),
            avatar_hash: Some([7u8; 32]),
            avatar_fingerprint: Some([1, 2, 3, 4, 5, 6, 7, 8]),
            ..Default::default()
        }
    }

    fn existing_full() -> BTreeMap<String, Value> {
        ProfileFields {
            display_name: Some("Alice".into()),
            public_message: Some("hello world".into()),
            ..avatar_fields()
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

    fn owner(n: usize) -> Identifier {
        let mut bytes = [0u8; 32];
        bytes[..8].copy_from_slice(&(n as u64 + 1).to_be_bytes());
        Identifier::new(bytes)
    }

    /// A partial update (only `displayName`) yields a replacement of the same
    /// document with the sibling fields kept and the revision incremented.
    #[test]
    fn should_replace_the_same_document_keeping_sibling_fields() {
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

    /// A supplied avatar hash and fingerprint replace the existing ones.
    #[test]
    fn should_replace_the_avatar_when_supplied() {
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

    /// An update must not replace only one of the two hashes, though the
    /// stored avatar keeps the merged document complete; replacing the whole
    /// avatar, an unchanged URL on its own, or only unrelated fields stays
    /// allowed.
    #[test]
    fn should_reject_an_update_with_one_hash() {
        let existing = profile_document(
            Identifier::new([0xD1; 32]),
            Identifier::new([0x0E; 32]),
            existing_full(),
            Some(3),
        );
        let replace =
            |fields: ProfileFields| replacement_document(&existing, fields, &dashpay_contract());

        for allowed in [
            avatar_fields(),
            ProfileFields {
                avatar_url: None,
                ..avatar_fields()
            },
            ProfileFields {
                avatar_hash: None,
                avatar_fingerprint: None,
                ..avatar_fields()
            },
            ProfileFields {
                public_message: Some("hi".into()),
                ..Default::default()
            },
        ] {
            assert!(replace(allowed.clone()).is_ok(), "{allowed:?}");
        }
        for one_hash in [
            ProfileFields {
                avatar_fingerprint: None,
                ..avatar_fields()
            },
            ProfileFields {
                avatar_hash: Some([9u8; 32]),
                ..Default::default()
            },
        ] {
            assert!(
                matches!(replace(one_hash.clone()), Err(Error::Generic(_))),
                "{one_hash:?}"
            );
        }
    }

    /// A changed avatar URL must not keep the old hashes under it, but the
    /// unchanged URL the app resends on a text-only edit keeps its hashes.
    #[test]
    fn should_require_hashes_only_for_a_changed_avatar_url() {
        let existing = profile_document(
            Identifier::new([0xD1; 32]),
            Identifier::new([0x0E; 32]),
            existing_full(),
            Some(3),
        );
        let replace =
            |fields: ProfileFields| replacement_document(&existing, fields, &dashpay_contract());
        let new_url = Some("https://x/b.png".to_string());

        assert!(matches!(
            replace(ProfileFields {
                avatar_url: new_url.clone(),
                ..Default::default()
            }),
            Err(Error::Generic(_))
        ));

        let unchanged = replace(ProfileFields {
            display_name: Some("Alice 2".into()),
            avatar_url: Some("https://x/a.png".into()),
            ..Default::default()
        })
        .expect("an unchanged URL needs no hashes");
        assert_eq!(
            unchanged.properties().get("avatarHash"),
            Some(&Value::Bytes32([7u8; 32]))
        );

        let changed = replace(ProfileFields {
            avatar_url: new_url,
            avatar_hash: Some([9u8; 32]),
            avatar_fingerprint: Some([9u8; 8]),
            ..Default::default()
        })
        .expect("a changed URL with its hashes");
        assert_eq!(
            text(changed.properties(), "avatarUrl"),
            Some("https://x/b.png")
        );
        assert_eq!(
            changed.properties().get("avatarHash"),
            Some(&Value::Bytes32([9u8; 32]))
        );
    }

    #[test]
    fn should_reject_duplicated_profile_owners() {
        assert!(ensure_unique_owners(&[]).is_ok());
        assert!(ensure_unique_owners(&[owner(0), owner(1)]).is_ok());
        assert!(matches!(
            ensure_unique_owners(&[owner(0), owner(1), owner(0)]),
            Err(Error::Generic(_))
        ));
    }

    /// DAPI rejects any query that uses a range where-operator without a
    /// matching `orderBy` on the range field ("missing order by for range").
    /// The profile query filters by `$ownerId In [...]`, a range operator, so
    /// every range clause it builds must carry an `orderBy` on its field.
    /// Guarded against vacuous truth: the query must actually contain a range
    /// clause, otherwise the invariant would pass trivially.
    #[test]
    fn should_order_the_profile_query_by_every_range_field() {
        let contract = Arc::new(dashpay_contract());

        let query = profiles_query(&contract, &[owner(0), owner(1)]).unwrap();

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

    #[cfg(feature = "mocks")]
    mod with_mock_sdk {
        use super::*;
        use crate::query_types::Documents;
        use dpp::data_contract::accessors::v0::DataContractV0Setters;

        /// More than 100 owners are fetched in queries of at most 100 each and
        /// merged. The mock answers only the three expected chunk queries, so a
        /// single oversized query would fail.
        #[tokio::test]
        async fn should_fetch_profiles_in_chunks_of_at_most_one_hundred_owners() {
            let contract = Arc::new(dashpay_contract());
            let owners: Vec<Identifier> = (0..250).map(owner).collect();
            let mut sdk = Sdk::new_mock();

            for chunk in owners.chunks(MAX_PROFILE_OWNERS_PER_QUERY) {
                let first = chunk[0];
                let document = profile_document(
                    Identifier::new([first.to_buffer()[7]; 32]),
                    first,
                    existing_full(),
                    Some(1),
                );
                let answer = Documents::from([(document.id(), Some(document))]);
                sdk.mock()
                    .expect_fetch_many::<Identifier, Document, _, Documents>(
                        profiles_query(&contract, chunk).unwrap(),
                        Some(answer),
                    )
                    .await
                    .expect("chunk expectation");
            }

            let profiles = sdk.get_profiles(&contract, &owners).await.unwrap();

            assert_eq!(
                profiles.keys().copied().collect::<Vec<_>>(),
                vec![owner(0), owner(100), owner(200)]
            );
        }

        /// A contract that is not the DashPay system contract is refused before
        /// any query is made.
        #[tokio::test]
        async fn should_reject_a_contract_that_is_not_the_dashpay_contract() {
            let mut other = dashpay_contract();
            other.set_id(Identifier::new([0x42; 32]));

            let result = Sdk::new_mock()
                .get_profiles(&Arc::new(other), &[owner(0)])
                .await;

            assert!(matches!(result, Err(Error::Generic(_))));
        }
    }
}
