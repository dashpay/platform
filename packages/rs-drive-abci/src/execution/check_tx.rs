use crate::error::Error;
use crate::platform::{Platform, PlatformRef};
use crate::rpc::core::CoreRPCLike;
use crate::validation::state_transition::process_state_transition;
use dpp::block::block_info::BlockInfo;
use dpp::consensus::ConsensusError;
use dpp::serialization_traits::PlatformDeserializable;
use dpp::state_transition::StateTransition;
use dpp::validation::ValidationResult;
use drive::fee::result::FeeResult;

impl<C> Platform<C>
where
    C: CoreRPCLike,
{
    /// Checks a state transition to determine if it should be added to the mempool.
    ///
    /// This function performs a few checks, including validating the state transition and ensuring that the
    /// user can pay for it. It may be inaccurate in rare cases, so the proposer needs to re-check transactions
    /// before proposing a block.
    ///
    /// # Arguments
    ///
    /// * `raw_tx` - A raw transaction represented as a vector of bytes.
    ///
    /// # Returns
    ///
    /// * `Result<ValidationResult<FeeResult, ConsensusError>, Error>` - If the state transition passes all
    ///   checks, it returns a `ValidationResult` with fee information. If any check fails, it returns an `Error`.
    pub fn check_tx(
        &self,
        raw_tx: Vec<u8>,
    ) -> Result<ValidationResult<FeeResult, ConsensusError>, Error> {
        let state_transition =
            StateTransition::deserialize(raw_tx.as_slice()).map_err(Error::Protocol)?;
        let state_read_guard = self.state.read().unwrap();
        let platform_ref = PlatformRef {
            drive: &self.drive,
            state: &state_read_guard,
            config: &self.config,
            core_rpc: &self.core_rpc,
        };
        let execution_event = process_state_transition(&platform_ref, state_transition, None)?;

        // We should run the execution event in dry run to see if we would have enough fees for the transaction

        // We need the approximate block info
        if let Some(block_info) = state_read_guard.last_committed_block_info.as_ref() {
            // We do not put the transaction, because this event happens outside of a block
            execution_event.and_then_borrowed_validation(|execution_event| {
                self.validate_fees_of_event(execution_event, &block_info.basic_info, None)
            })
        } else {
            execution_event.and_then_borrowed_validation(|execution_event| {
                self.validate_fees_of_event(execution_event, &BlockInfo::default(), None)
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{PlatformConfig, PlatformTestConfig};
    use crate::error::Error;
    use crate::platform::Platform;
    use crate::test::helpers::setup::TestPlatformBuilder;
    use dpp::block::block_info::BlockInfo;
    use dpp::identity::Identity;
    use dpp::prelude::{Identifier, IdentityPublicKey};
    use dpp::serialization_traits::{PlatformDeserializable, Signable};
    use dpp::state_transition::StateTransition;
    use dpp::state_transition::StateTransition::DataContractCreate;
    use std::collections::BTreeMap;
    use tenderdash_abci::proto::abci::RequestInitChain;
    use tenderdash_abci::proto::google::protobuf::Timestamp;

    #[test]
    fn data_contract_create_check_tx() {
        let serialized = hex::decode("00010001e2716cafada3bbe565025e4597276424c7a1d2b19bb67d3f44fa111f4e7696e300000000013468747470733a2f2f736368656d612e646173682e6f72672f6470702d302d342d302f6d6574612f646174612d636f6e7472616374019e71b47e5b533e2c53366158f0d7548ba79ca6cbde04401fd7c79597bef6fb2c030f696e6465786564446f63756d656e74160512047479706512066f626a6563741207696e64696365731506160312046e616d651206696e64657831120a70726f70657274696573150216011208246f776e6572496412036173631601120966697273744e616d6512036173631206756e697175651301160312046e616d651206696e64657832120a70726f70657274696573150216011208246f776e657249641203617363160112086c6173744e616d6512036173631206756e697175651301160212046e616d651206696e64657833120a70726f706572746965731501160112086c6173744e616d651203617363160212046e616d651206696e64657834120a70726f7065727469657315021601120a2463726561746564417412036173631601120a247570646174656441741203617363160212046e616d651206696e64657835120a70726f7065727469657315011601120a247570646174656441741203617363160212046e616d651206696e64657836120a70726f7065727469657315011601120a246372656174656441741203617363120a70726f706572746965731602120966697273744e616d6516021204747970651206737472696e6712096d61784c656e677468023f12086c6173744e616d6516021204747970651206737472696e6712096d61784c656e677468023f120872657175697265641504120966697273744e616d65120a24637265617465644174120a2475706461746564417412086c6173744e616d6512146164646974696f6e616c50726f7065727469657313000c6e696365446f63756d656e74160412047479706512066f626a656374120a70726f70657274696573160112046e616d6516011204747970651206737472696e67120872657175697265641501120a2463726561746564417412146164646974696f6e616c50726f7065727469657313000e7769746842797465417272617973160512047479706512066f626a6563741207696e64696365731501160212046e616d651206696e64657831120a70726f7065727469657315011601120e6279746541727261794669656c641203617363120a70726f706572746965731602120e6279746541727261794669656c641603120474797065120561727261791209627974654172726179130112086d61784974656d730210120f6964656e7469666965724669656c64160512047479706512056172726179120962797465417272617913011210636f6e74656e744d656469615479706512216170706c69636174696f6e2f782e646173682e6470702e6964656e74696669657212086d696e4974656d73022012086d61784974656d730220120872657175697265641501120e6279746541727261794669656c6412146164646974696f6e616c50726f706572746965731300005e236018181816974c2d73a407a1d4ce4e936cb3431e0ea94e2777fb3e87f9e80141204039fe689533d8bc9137ef298c3e6a5f7c40e8001bb670a370149b1c7f3f4b326e311e3df862f4ea6f146ad858d1ef2b3a02949284cdb3b09f63974d5912b04e").expect("expected to decode");
        let platform = TestPlatformBuilder::new()
            .with_config(PlatformConfig::default())
            .build_with_mock_rpc();

        let key = IdentityPublicKey::random_authentication_key(1, Some(1));

        platform
            .drive
            .create_initial_state_structure(None)
            .expect("expected to create state structure");
        let identity = Identity {
            protocol_version: 1,
            id: Identifier::new([
                158, 113, 180, 126, 91, 83, 62, 44, 83, 54, 97, 88, 240, 215, 84, 139, 167, 156,
                166, 203, 222, 4, 64, 31, 215, 199, 149, 151, 190, 246, 251, 44,
            ]),
            public_keys: BTreeMap::from([(1, key)]),
            balance: 100000,
            revision: 0,
            asset_lock_proof: None,
            metadata: None,
        };
        platform
            .drive
            .add_new_identity(identity, &BlockInfo::default(), true, None)
            .expect("expected to insert identity");

        let validation_result = platform.check_tx(serialized).expect("expected to check tx");

        //todo fix
        // assert!(validation_result.errors.is_empty());
    }
}
