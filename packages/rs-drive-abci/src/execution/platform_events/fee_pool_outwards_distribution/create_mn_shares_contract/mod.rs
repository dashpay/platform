/// For testing only
use crate::platform_types::platform::Platform;
use dpp::block::block_info::BlockInfo;

use dpp::data_contract::DataContract;

use drive::drive::flags::StorageFlags;

use drive::grovedb::TransactionArg;

use dpp::data_contract::conversion::cbor::DataContractCborConversionMethodsV0;
use dpp::version::PlatformVersion;
use std::borrow::Cow;

impl<C> Platform<C> {
    /// A function to create and apply the masternode reward shares contract.
    pub fn create_mn_shares_contract(
        &self,
        transaction: TransactionArg,
        platform_version: &PlatformVersion,
    ) -> DataContract {
        let platform_version = PlatformVersion::latest();
        let contract_hex = "01a56324696458200cace205246693a7c8156523620daa937d2f2247934463eeb01ff7219590958c6724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e65724964582024da2bb09da5b1429f717ac1ce6537126cc65215f1d017e67b65eb252ef964b76776657273696f6e0169646f63756d656e7473a16b7265776172645368617265a66474797065666f626a65637467696e646963657382a3646e616d65716f776e65724964416e64506179546f496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a167706179546f496463617363a2646e616d65676f776e657249646a70726f7065727469657381a168246f776e65724964636173636872657175697265648267706179546f49646a70657263656e746167656a70726f70657274696573a267706179546f4964a66474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e781f4964656e74696669657220746f20736861726520726577617264207769746870636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726a70657263656e74616765a4647479706567696e7465676572676d6178696d756d192710676d696e696d756d016b6465736372697074696f6e781a5265776172642070657263656e7461676520746f2073686172656b6465736372697074696f6e78405368617265207370656369666965642070657263656e74616765206f66206d61737465726e6f646520726577617264732077697468206964656e746974696573746164646974696f6e616c50726f70657274696573f4";

        let contract_cbor = hex::decode(contract_hex).expect("Decoding failed");

        let contract = DataContract::from_cbor(&contract_cbor, platform_version)
            .expect("expected to deserialize the contract");

        let storage_flags = Some(Cow::Owned(StorageFlags::SingleEpoch(0)));

        self.drive
            .apply_contract_with_serialization(
                &contract,
                contract_cbor,
                BlockInfo::genesis(),
                true,
                storage_flags,
                transaction,
                platform_version,
            )
            .expect("expected to apply contract successfully");

        contract
    }
}
