mod v0;

use crate::drive::Drive;
use crate::error::drive::DriveError;
use crate::error::Error;
use crate::fees::op::LowLevelDriveOperation;
use dpp::data_contract::document_type::property_constraints::AggregateRead;
use dpp::data_contract::document_type::DocumentTypeRef;
use dpp::platform_value::Value;
use dpp::version::PlatformVersion;
use grovedb::TransactionArg;

impl Drive {
    /// The total a `countOf` or `sumOf` of a `propertyConstraints` rule reads, as the
    /// count or sum tree of `counted` keeps it before the write being judged: how many
    /// documents of `counted`, a type of the contract `contract_id`, match `filter_values`
    /// (from [`AggregateRead::filter_values`]), or the total of the summed property over
    /// them; over every document of the type when the read has no filter. The reads are
    /// added to `drive_operations`, so that consensus can bill them.
    ///
    /// Registration makes a tree keep every total a rule reads
    /// ([`AggregateRead::whole_type_kept`], [`AggregateRead::answering_index`]); a read
    /// no tree keeps is a corrupted contract.
    ///
    /// # Parameters
    ///
    /// * `contract_id`: The contract declaring both types.
    /// * `counted`: The document type the read totals.
    /// * `read`: The `countOf` or `sumOf`.
    /// * `filter_values`: The values its filter's keys must take.
    /// * `transaction`: The GroveDB transaction.
    /// * `drive_operations`: The operations the reads add to.
    /// * `platform_version`: The platform version.
    #[allow(clippy::too_many_arguments)]
    pub fn fetch_property_constraint_aggregate(
        &self,
        contract_id: [u8; 32],
        counted: DocumentTypeRef,
        read: &AggregateRead,
        filter_values: &[(String, Value)],
        transaction: TransactionArg,
        drive_operations: &mut Vec<LowLevelDriveOperation>,
        platform_version: &PlatformVersion,
    ) -> Result<i128, Error> {
        match platform_version
            .drive
            .methods
            .document
            .fetch_property_constraint_aggregate
        {
            0 => self.fetch_property_constraint_aggregate_v0(
                contract_id,
                counted,
                read,
                filter_values,
                transaction,
                drive_operations,
                platform_version,
            ),
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "fetch_property_constraint_aggregate".to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
