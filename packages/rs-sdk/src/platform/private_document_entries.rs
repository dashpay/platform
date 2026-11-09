//! The entries of a private document type's store (`getPrivateDocumentEntries`): the type's
//! fixed-size encrypted entries in append order, each with the height and time of the block
//! that appended it, a page at a time. Nodes serve them from protocol version 15, and the SDK
//! sends no such request before the network has proven 15.
//!
//! A page starts at any position of the store and reads to the end of the store's chunk
//! holding it (the type's entry size sets the chunk), or to the store's last entry, as its
//! `total_count` tells. A reader walking the store asks for the page from its cursor, then
//! for the page from the position after the last entry it got.
//!
//! [`PrivateDocumentEntries::fetch`] verifies the page against the platform's root hash, with
//! the contract the context provider supplies, so a node can neither leave entries out of it
//! nor change their header times. [`FetchUnproved`] reads it without a proof. A provider that
//! holds the contract from before its private type was declared fails that verification;
//! [`Sdk::fetch_private_document_contract`] gives it the contract with the type first.

use std::sync::Arc;

use crate::platform::{
    Fetch, FetchUnproved, Identifier, ProtocolRequirement, Query, QuerySettings,
};
use crate::{Error, Sdk};
use dapi_grpc::platform::v0::get_private_document_entries_request::GetPrivateDocumentEntriesRequestV0;
use dapi_grpc::platform::v0::{
    get_private_document_entries_request, GetPrivateDocumentEntriesRequest,
};
use dash_context_provider::ContextProvider;
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::accessors::DocumentTypeV2Getters;
use dpp::data_contract::DataContract;
use dpp::version::feature_initial_protocol_versions::PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION;
pub use drive_proof_verifier::types::private_document_entries::{
    PrivateDocumentEntries, PrivateDocumentEntry,
};

/// Query for a page of the entries of a private document type's store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrivateDocumentEntriesQuery {
    /// The contract declaring the private type.
    pub data_contract_id: Identifier,
    /// The private type.
    pub document_type_name: String,
    /// The page's first position.
    pub start_position: u64,
}

impl PrivateDocumentEntriesQuery {
    /// The query for the page of the store of `document_type_name` from `start_position` to
    /// the end of the chunk holding it.
    pub fn new(
        data_contract_id: Identifier,
        document_type_name: impl Into<String>,
        start_position: u64,
    ) -> Self {
        PrivateDocumentEntriesQuery {
            data_contract_id,
            document_type_name: document_type_name.into(),
            start_position,
        }
    }
}

impl Query<GetPrivateDocumentEntriesRequest> for PrivateDocumentEntriesQuery {
    fn query(
        &self,
        settings: &QuerySettings<'_>,
    ) -> Result<GetPrivateDocumentEntriesRequest, Error> {
        Ok(GetPrivateDocumentEntriesRequest {
            version: Some(get_private_document_entries_request::Version::V0(
                GetPrivateDocumentEntriesRequestV0 {
                    data_contract_id: self.data_contract_id.to_vec(),
                    document_type_name: self.document_type_name.clone(),
                    start_position: self.start_position,
                    prove: settings.prove,
                },
            )),
        })
    }
}

impl Fetch for PrivateDocumentEntries {
    type Query = GetPrivateDocumentEntriesRequest;
    type Request = GetPrivateDocumentEntriesRequest;
}

impl FetchUnproved for PrivateDocumentEntries {
    type Request = GetPrivateDocumentEntriesRequest;
}

/// The gRPC method a page of a private document type's store is read with.
const PRIVATE_DOCUMENT_ENTRIES_METHOD: &str = "get_private_document_entries";

/// Whether `contract` declares `document_type_name` as a private type.
fn declares_private_type(contract: &DataContract, document_type_name: &str) -> bool {
    contract
        .document_type_for_name(document_type_name)
        .is_ok_and(|document_type| document_type.is_private())
}

impl Sdk {
    /// The contract `data_contract_id`, with its private type `document_type_name`, as the
    /// context provider holds it: a page of the type's store is verified against the contract
    /// the provider gives, and one kept from before the type was declared, as a client
    /// hydrated across the protocol version that declared it may hold, fails every page.
    ///
    /// A held contract that declares the type is used as it is and never replaced, so a newer
    /// one the provider proved stays. Otherwise the contract is fetched from the network,
    /// refused when it does not declare the type, given to the provider and read back; the
    /// call fails when the provider then still holds none that declares it. A newer contract
    /// the provider was given while the fetch ran, by another operation, is kept and used
    /// rather than replaced by the fetched one. Without a context provider the fetched
    /// contract is returned.
    ///
    /// Private types exist from protocol version 15. Below it, as the page fetch is, the call
    /// is refused with [`Error::ProtocolVersionNotActive`], before anything is fetched on a
    /// network that has not reached it, and when the contract declares no such type while
    /// this SDK runs below it: a reader takes that as no store to read yet.
    pub async fn fetch_private_document_contract(
        &self,
        data_contract_id: Identifier,
        document_type_name: &str,
    ) -> Result<Arc<DataContract>, Error> {
        self.ensure_network_serves(
            Some(ProtocolRequirement {
                protocol_version:
                    PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                method: PRIVATE_DOCUMENT_ENTRIES_METHOD,
                feature: None,
            }),
            None,
        )
        .await?;
        let provider = self.context_provider();
        if let Some(provider) = &provider {
            if let Some(held) = provider.get_data_contract(&data_contract_id, self.version())? {
                if declares_private_type(&held, document_type_name) {
                    return Ok(held);
                }
            }
        }
        let contract = DataContract::fetch(self, data_contract_id)
            .await?
            .map(Arc::new)
            .ok_or_else(|| Error::Generic(format!("the contract {data_contract_id} is gone")))?;
        if !declares_private_type(&contract, document_type_name) {
            // Another operation may have given the provider a newer contract declaring the
            // type while this one was fetched: it is read against that one.
            if let Some(provider) = &provider {
                if let Some(held) = provider.get_data_contract(&data_contract_id, self.version())? {
                    if held.version() > contract.version()
                        && declares_private_type(&held, document_type_name)
                    {
                        return Ok(held);
                    }
                }
            }
            if self.protocol_version_number()
                < PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION
            {
                return Err(self.protocol_version_not_active(
                    PRIVATE_DOCUMENT_CREATE_FROM_SHIELDED_POOL_INITIAL_PROTOCOL_VERSION,
                    PRIVATE_DOCUMENT_ENTRIES_METHOD,
                    None,
                    None,
                ));
            }
            return Err(Error::Generic(format!(
                "the contract {data_contract_id} declares no private type \"{document_type_name}\""
            )));
        }
        let Some(provider) = provider else {
            return Ok(contract);
        };
        // Another operation may have given the provider a newer contract while this one was
        // fetched: it stays, as a provider whose registration replaces would put the older
        // fetched one in its place.
        if let Some(held) = provider.get_data_contract(&data_contract_id, self.version())? {
            if held.version() > contract.version() {
                return if declares_private_type(&held, document_type_name) {
                    Ok(held)
                } else {
                    Err(Error::Generic(format!(
                        "the context provider holds a newer contract {data_contract_id} that \
                         declares no private type \"{document_type_name}\""
                    )))
                };
            }
        }
        provider.register_data_contract(contract);
        match provider.get_data_contract(&data_contract_id, self.version())? {
            Some(held) if declares_private_type(&held, document_type_name) => Ok(held),
            _ => Err(Error::Generic(format!(
                "the context provider does not hold the contract {data_contract_id} with its \
                 private type \"{document_type_name}\" once given it, so the type's pages cannot \
                 be verified"
            ))),
        }
    }
}

/// A context provider for tests reading private document pages.
#[cfg(all(test, feature = "mocks"))]
pub(crate) mod held_contract_provider {
    use std::sync::{Arc, Mutex};

    use dash_context_provider::{ContextProvider, ContextProviderError};
    use dpp::data_contract::{DataContract, TokenConfiguration};
    use dpp::prelude::{CoreBlockHeight, Identifier};
    use dpp::version::PlatformVersion;

    /// A context provider holding `held`, or nothing, whatever version it is asked for, as a
    /// provider's known contracts do. A `writable` one puts a registered contract in its
    /// place, whatever its version, unlike a provider keeping the newer one as
    /// `ContextProvider::register_data_contract` asks: so a test shows a reader does not rely
    /// on that. A read-only one keeps what it holds. Its clones share what it holds.
    #[derive(Clone)]
    pub(crate) struct HeldContractProvider {
        held: Arc<Mutex<Option<Arc<DataContract>>>>,
        writable: bool,
        /// Held from the first read on, as if another operation gave it to the provider
        /// while the reader's fetch ran.
        promoted: Arc<Mutex<Option<Arc<DataContract>>>>,
    }

    impl HeldContractProvider {
        pub(crate) fn new(held: Option<DataContract>, writable: bool) -> Self {
            Self {
                held: Arc::new(Mutex::new(held.map(Arc::new))),
                writable,
                promoted: Arc::new(Mutex::new(None)),
            }
        }

        /// A writable provider holding `held` until it is first read, then `promoted`.
        pub(crate) fn promoting(held: DataContract, promoted: DataContract) -> Self {
            let provider = Self::new(Some(held), true);
            *provider.promoted.lock().expect("the lock") = Some(Arc::new(promoted));
            provider
        }

        pub(crate) fn held(&self) -> Option<Arc<DataContract>> {
            self.held.lock().expect("the lock").clone()
        }
    }

    impl ContextProvider for HeldContractProvider {
        fn get_data_contract(
            &self,
            _id: &Identifier,
            _platform_version: &PlatformVersion,
        ) -> Result<Option<Arc<DataContract>>, ContextProviderError> {
            let held = self.held();
            if let Some(promoted) = self.promoted.lock().expect("the lock").take() {
                *self.held.lock().expect("the lock") = Some(promoted);
            }
            Ok(held)
        }

        fn get_token_configuration(
            &self,
            _token_id: &Identifier,
        ) -> Result<Option<TokenConfiguration>, ContextProviderError> {
            Ok(None)
        }

        fn get_quorum_public_key(
            &self,
            _quorum_type: u32,
            _quorum_hash: [u8; 32],
            _core_chain_locked_height: u32,
        ) -> Result<[u8; 48], ContextProviderError> {
            Err(ContextProviderError::InvalidQuorum(
                "no quorum in this test".to_string(),
            ))
        }

        fn get_platform_activation_height(&self) -> Result<CoreBlockHeight, ContextProviderError> {
            Ok(1)
        }

        fn register_data_contract(&self, contract: Arc<DataContract>) {
            if self.writable {
                *self.held.lock().expect("the lock") = Some(contract);
            }
        }
    }
}

#[cfg(all(test, feature = "mocks"))]
mod tests {
    use super::*;
    use crate::{Sdk, SdkBuilder};
    use dpp::version::PlatformVersion;

    #[tokio::test]
    async fn should_fetch_a_page_of_private_entries_from_mock_expectations() {
        let mut sdk: Sdk = SdkBuilder::new_mock()
            .with_version(PlatformVersion::latest())
            .build()
            .expect("expected a mock sdk");
        let query = PrivateDocumentEntriesQuery::new(Identifier::new([4; 32]), "secret", 64);
        let page = PrivateDocumentEntries {
            chunk_power: None,
            entries: (64..67u64)
                .map(|position| PrivateDocumentEntry {
                    position,
                    block_height: 10 + position,
                    block_time_ms: 500 * position,
                    data: vec![position as u8; 585],
                })
                .collect(),
            total_count: 67,
        };
        sdk.mock()
            .expect_fetch(query.clone(), Some(page.clone()))
            .await
            .expect("expected to register the page");

        assert_eq!(
            PrivateDocumentEntries::fetch(&sdk, query)
                .await
                .expect("expected the page"),
            Some(page)
        );
    }
}
