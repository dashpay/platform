//! DashPay contact requests created in blocks, shared by the Drive paging test
//! and the drive-abci generator of the SDK's signed contact-request responses.
//!
//! Every request of a block shares the block time as its `$createdAt`. Each
//! set has its own sender and recipient, so a set's requests are exactly the
//! requests sent by `owner` and exactly the requests received by `recipient`.

#![allow(dead_code)]

use dpp::block::block_info::BlockInfo;
use dpp::dashcore::hashes::{sha256, Hash};
use dpp::data_contract::accessors::v0::DataContractV0Getters;
use dpp::data_contract::document_type::methods::DocumentTypeV0Methods;
use dpp::document::{Document, DocumentV0Setters};
use dpp::platform_value::{platform_value, Identifier, Value};
use dpp::prelude::DataContract;
use dpp::system_data_contracts::{load_system_data_contract, SystemDataContract};
use dpp::version::PlatformVersion;
use drive::drive::Drive;
use drive::util::object_size_info::{DocumentAndContractInfo, DocumentInfo, OwnedDocumentInfo};

/// Contact requests from one sender to one recipient, created in blocks:
/// `(block time, requests)`.
pub struct ContactBlocks {
    pub owner: [u8; 32],
    pub recipient: [u8; 32],
    pub blocks: &'static [(u64, usize)],
}

/// A block of 150, one and a half pages of 100, between an earlier block and
/// three later ones: 220 requests, so at a page of 100 the first continuation
/// is full as well.
pub const BLOCK_OF_150: ContactBlocks = ContactBlocks {
    owner: [0x68; 32],
    recipient: [0x78; 32],
    blocks: &[
        (1_700_000_100_000, 5),
        (1_700_000_200_000, 150),
        (1_700_000_300_000, 3),
        (1_700_000_400_000, 2),
        (1_700_000_450_000, 60),
    ],
};

/// A block of exactly one page of 100, then a later block.
pub const BLOCK_OF_100: ContactBlocks = ContactBlocks {
    owner: [0x69; 32],
    recipient: [0x79; 32],
    blocks: &[(1_700_000_100_000, 100), (1_700_000_200_000, 1)],
};

/// A block of 7 between others: the shape read live on mainnet at protocol
/// version 13 with a page size of 2.
pub const BLOCK_OF_7: ContactBlocks = ContactBlocks {
    owner: [0x6a; 32],
    recipient: [0x7a; 32],
    blocks: &[
        (1_700_000_100_000, 1),
        (1_700_000_200_000, 7),
        (1_700_000_300_000, 2),
    ],
};

/// One request between two other identities, created in the block of 150's
/// block: a cursor that is not a row of any other set's query.
pub const FOREIGN: ContactBlocks = ContactBlocks {
    owner: [0x6b; 32],
    recipient: [0x7b; 32],
    blocks: &[(1_700_000_200_000, 1)],
};

pub const ALL: [&ContactBlocks; 4] = [&BLOCK_OF_150, &BLOCK_OF_100, &BLOCK_OF_7, &FOREIGN];

/// The two queries that list a set's requests: received by its recipient
/// (`toUserId`) and sent by its owner (`$ownerId`).
pub const DIRECTIONS: [&str; 2] = ["toUserId", "$ownerId"];

impl ContactBlocks {
    /// The identity a query over `field` filters on.
    pub fn identity(&self, field: &str) -> Identifier {
        match field {
            "toUserId" => Identifier::from(self.recipient),
            "$ownerId" => Identifier::from(self.owner),
            field => panic!("no contact-request direction filters on {field}"),
        }
    }

    /// Every request as `(created_at, id)`, in the `(field, $createdAt)`
    /// index order: by creation time, then by document id within a block.
    pub fn rows(&self) -> Vec<(u64, Identifier)> {
        let mut rows: Vec<_> = self
            .requests()
            .map(|(created_at, account_reference)| (created_at, self.request_id(account_reference)))
            .collect();
        rows.sort();
        rows
    }

    /// The ids of the requests created after `after`, in index order.
    pub fn ids_after(&self, after: u64) -> Vec<Identifier> {
        self.rows()
            .into_iter()
            .filter(|(created_at, _)| *created_at > after)
            .map(|(_, id)| id)
            .collect()
    }

    /// `(created_at, account_reference)` of every request, in insertion
    /// order. The account reference is unique per sender and recipient, as
    /// the contract's unique index requires.
    fn requests(&self) -> impl Iterator<Item = (u64, u32)> + '_ {
        self.blocks
            .iter()
            .flat_map(|&(created_at, count)| std::iter::repeat_n(created_at, count))
            .zip(1u32..)
    }

    /// A document id unrelated to insertion order, so the order inside a
    /// block is the id order Drive keeps, not the order requests were added.
    fn request_id(&self, account_reference: u32) -> Identifier {
        let mut preimage = self.owner.to_vec();
        preimage.extend_from_slice(&account_reference.to_be_bytes());
        Identifier::from(sha256::Hash::hash(&preimage).to_byte_array())
    }

    fn documents(&self, contract: &DataContract, version: &PlatformVersion) -> Vec<Document> {
        let document_type = contract
            .document_type_for_name("contactRequest")
            .expect("contactRequest type");
        self.requests()
            .map(|(created_at, account_reference)| {
                let mut document = document_type
                    .create_document_from_data(
                        platform_value!({
                            "toUserId": Identifier::from(self.recipient),
                            "encryptedPublicKey": Value::Bytes(vec![0x42; 96]),
                            "senderKeyIndex": 2u32,
                            "recipientKeyIndex": 3u32,
                            "accountReference": account_reference,
                            "$createdAt": created_at,
                            "$createdAtCoreBlockHeight": 1_000u32,
                        }),
                        Identifier::from(self.owner),
                        1,
                        1_000,
                        [0; 32],
                        version,
                    )
                    .expect("contact request");
                document.set_id(self.request_id(account_reference));
                document
            })
            .collect()
    }
}

/// Apply the DashPay contract of `version` to `drive` and insert every set
/// in [`ALL`].
pub fn populate(drive: &Drive, version: &PlatformVersion) -> DataContract {
    let contract =
        load_system_data_contract(SystemDataContract::Dashpay, version).expect("DashPay contract");
    drive
        .apply_contract(&contract, BlockInfo::default(), true, None, None, version)
        .expect("apply DashPay contract");
    let document_type = contract
        .document_type_for_name("contactRequest")
        .expect("contactRequest type");
    for set in ALL {
        for document in set.documents(&contract, version) {
            drive
                .add_document_for_contract(
                    DocumentAndContractInfo {
                        owned_document_info: OwnedDocumentInfo {
                            document_info: DocumentInfo::DocumentRefInfo((&document, None)),
                            owner_id: None,
                        },
                        contract: &contract,
                        document_type,
                    },
                    false,
                    BlockInfo::default(),
                    true,
                    None,
                    version,
                    None,
                )
                .expect("insert contact request");
        }
    }
    contract
}
