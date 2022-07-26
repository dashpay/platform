use crate::error::Error;
use crate::platform::Platform;
use rs_drive::common::value_to_cbor;
use rs_drive::contract::document::Document;
use rs_drive::contract::{Contract, DataContract};
use rs_drive::dpp::data_contract::extra::DriveContractExt;
use rs_drive::drive::flags::StorageFlags;
use rs_drive::grovedb::TransactionArg;
use serde_json::json;

pub const MN_REWARD_SHARES_CONTRACT_ID: [u8; 32] = [
    0x0c, 0xac, 0xe2, 0x05, 0x24, 0x66, 0x93, 0xa7, 0xc8, 0x15, 0x65, 0x23, 0x62, 0x0d, 0xaa, 0x93,
    0x7d, 0x2f, 0x22, 0x47, 0x93, 0x44, 0x63, 0xee, 0xb0, 0x1f, 0xf7, 0x21, 0x95, 0x90, 0x95, 0x8c,
];

pub const MN_REWARD_SHARES_DOCUMENT_TYPE: &str = "rewardShare";

impl Platform {
    pub(crate) fn get_reward_shares_list_for_masternode(
        &self,
        masternode_owner_id: &Vec<u8>,
        transaction: TransactionArg,
    ) -> Result<Vec<Document>, Error> {
        let query_json = json!({
            "where": [
                ["$ownerId", "==", bs58::encode(masternode_owner_id).into_string()]
            ],
        });

        let query_cbor = value_to_cbor(query_json, None);

        let (document_cbors, _, _) = self.drive.query_documents(
            &query_cbor,
            MN_REWARD_SHARES_CONTRACT_ID,
            MN_REWARD_SHARES_DOCUMENT_TYPE,
            transaction,
        )?;

        document_cbors
            .iter()
            .map(|cbor| Document::from_cbor(cbor, None, None).map_err(Error::Drive))
            .collect::<Result<Vec<Document>, Error>>()
    }

    pub fn create_mn_shares_contract(&self, transaction: TransactionArg) -> Contract {
        let contract_hex = "01000000a56324696458200cace205246693a7c8156523620daa937d2f2247934463eeb01ff7219590958c6724736368656d61783468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374676f776e65724964582024da2bb09da5b1429f717ac1ce6537126cc65215f1d017e67b65eb252ef964b76776657273696f6e0169646f63756d656e7473a16b7265776172645368617265a66474797065666f626a65637467696e646963657382a3646e616d65716f776e65724964416e64506179546f496466756e69717565f56a70726f7065727469657382a168246f776e6572496463617363a167706179546f496463617363a2646e616d65676f776e657249646a70726f7065727469657381a168246f776e65724964636173636872657175697265648267706179546f49646a70657263656e746167656a70726f70657274696573a267706179546f4964a66474797065656172726179686d61784974656d731820686d696e4974656d73182069627974654172726179f56b6465736372697074696f6e781f4964656e74696669657220746f20736861726520726577617264207769746870636f6e74656e744d656469615479706578216170706c69636174696f6e2f782e646173682e6470702e6964656e7469666965726a70657263656e74616765a4647479706567696e7465676572676d6178696d756d192710676d696e696d756d016b6465736372697074696f6e781a5265776172642070657263656e7461676520746f2073686172656b6465736372697074696f6e78405368617265207370656369666965642070657263656e74616765206f66206d61737465726e6f646520726577617264732077697468206964656e746974696573746164646974696f6e616c50726f70657274696573f4";

        let contract_cbor = hex::decode(contract_hex).expect("Decoding failed");

        let contract = <Contract as DriveContractExt>::from_cbor(&contract_cbor, None)
            .expect("expected to deserialize the contract");

        self.drive
            .apply_contract(
                &contract,
                contract_cbor.clone(),
                0f64,
                true,
                StorageFlags { epoch: 0 },
                transaction,
            )
            .expect("expected to apply contract successfully");

        contract
    }
}
