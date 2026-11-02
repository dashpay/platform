use super::{
    AddressFunds, AddressIndex, AddressOps, AddressProvider, AddressSyncContext, AddressSyncResult,
    PlatformAddress, TrunkBranchSyncOps,
};
use crate::sdk::{verifier_version, ProofResponseMetadata};
use crate::SdkBuilder;
use async_trait::async_trait;
use dapi_grpc::platform::v0::GetAddressesTrunkStateRequest;
use dpp::version::PlatformVersion;
use rs_dapi_client::{DumpData, RequestSettings};
use std::collections::HashMap;
use std::path::PathBuf;

struct TrunkOnlyProvider;

#[async_trait]
impl AddressProvider for TrunkOnlyProvider {
    type Tag = ();
    type Address = PlatformAddress;

    fn gap_limit(&self) -> AddressIndex {
        0
    }
    fn pending_addresses(&self) -> impl Iterator<Item = (Self::Tag, Self::Address)> + '_ {
        std::iter::empty()
    }
    async fn on_address_found(&mut self, _: Self::Tag, _: &Self::Address, _: AddressFunds) {}
    async fn on_address_absent(&mut self, _: Self::Tag, _: &Self::Address) {}
    fn current_balances(
        &self,
    ) -> impl Iterator<Item = (Self::Tag, Self::Address, AddressFunds)> + '_ {
        std::iter::empty()
    }
}

#[tokio::test]
async fn should_carry_the_verified_trunk_version_into_pinned_address_branch_work() {
    let directory =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/vectors/test_sync_address_balances");
    let (_, response) = DumpData::<GetAddressesTrunkStateRequest>::load(directory.join(
        "msg_GetAddressesTrunkStateRequest_c49aeef007d9b94685bb7a38fa0daa7f11dfff63969ad07d74a36a45514c3d33.json",
    )).expect("recorded trunk proof").deserialize();
    let response = response.expect("successful recorded trunk response").inner;
    let signed_version = response
        .response_metadata()
        .expect("recorded metadata")
        .protocol_version;
    assert_eq!(signed_version, 12);
    let sdk = SdkBuilder::new_mock()
        .with_version(PlatformVersion::get(signed_version - 1).expect("older pin"))
        .with_dump_dir(&directory)
        .with_height_tolerance(None)
        .build()
        .expect("sdk");
    let mut provider = TrunkOnlyProvider;
    let mut key_to_tag = HashMap::new();
    let mut result = AddressSyncResult::new();
    let mut context = AddressSyncContext {
        provider: &mut provider,
        key_to_tag: &mut key_to_tag,
        result: &mut result,
    };
    let trunk = AddressOps::<TrunkOnlyProvider>::execute_trunk_query(
        &sdk,
        RequestSettings::default(),
        &mut context,
    )
    .await
    .expect("real trunk proof verifies");
    assert_eq!(trunk.protocol_version, signed_version);
    assert_eq!(
        verifier_version(sdk.version(), trunk.protocol_version).protocol_version,
        signed_version
    );
    assert_eq!(sdk.protocol_version_number(), signed_version - 1);
}
