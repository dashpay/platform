mod v0;

use crate::error::query::QueryError;
use crate::error::Error;
use crate::platform_types::platform::Platform;
use dapi_grpc::platform::v0::get_documents_request::Start;
use dapi_grpc::platform::v0::{
    get_data_contract_history_response, get_data_contract_response, get_data_contracts_response,
    get_documents_response, get_identities_by_public_key_hashes_response, get_identities_response,
    get_identity_balance_and_revision_response, get_identity_balance_response,
    get_identity_by_public_key_hashes_response, get_identity_keys_response, get_identity_response,
    GetDataContractHistoryRequest, GetDataContractHistoryResponse, GetDataContractRequest,
    GetDataContractResponse, GetDataContractsRequest, GetDataContractsResponse,
    GetDocumentsRequest, GetDocumentsResponse, GetIdentitiesByPublicKeyHashesRequest,
    GetIdentitiesByPublicKeyHashesResponse, GetIdentitiesRequest, GetIdentitiesResponse,
    GetIdentityBalanceAndRevisionResponse, GetIdentityBalanceResponse,
    GetIdentityByPublicKeyHashesRequest, GetIdentityByPublicKeyHashesResponse,
    GetIdentityKeysRequest, GetIdentityKeysResponse, GetIdentityRequest, GetIdentityResponse,
    GetProofsRequest, GetProofsResponse, Proof, ResponseMetadata,
};
use dpp::identifier::Identifier;
use dpp::platform_value::{Bytes20, Bytes32};
use std::collections::BTreeMap;

use dpp::serialization_traits::PlatformSerializable;
use dpp::validation::ValidationResult;
use dpp::{check_validation_result_with_data, ProtocolError};
use drive::drive::identity::IdentityDriveQuery;
use drive::drive::identity::IdentityProveRequestType;

use dapi_grpc::platform::v0::get_data_contracts_response::DataContractEntry;
use dapi_grpc::platform::v0::get_identities_response::IdentityEntry;
use dapi_grpc::platform::v0::get_identity_balance_and_revision_response::BalanceAndRevision;
use dpp::identity::{KeyID, Purpose, SecurityLevel};
use drive::drive::identity::key::fetch::{
    IdentityKeysRequest, KeyKindRequestType, KeyRequestType, PurposeU8, SecurityLevelU8,
    SerializedKeyVec,
};
use drive::error::contract::ContractError;
use drive::error::query::QuerySyntaxError;
use drive::query::{DriveQuery, SingleDocumentDriveQuery};
use prost::Message;

/// A query validation result
pub type QueryValidationResult<TData> = ValidationResult<TData, QueryError>;

impl<C> Platform<C> {
    /// Querying
    pub fn query(
        &self,
        query_path: &str,
        query_data: &[u8],
    ) -> Result<QueryValidationResult<Vec<u8>>, Error> {
        //todo: choose based on protocol version
        self.query_v0(query_path, query_data)
    }
}
