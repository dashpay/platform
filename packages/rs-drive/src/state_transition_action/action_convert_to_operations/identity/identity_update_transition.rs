use crate::state_transition_action::action_convert_to_operations::DriveHighLevelOperationConverter;
use crate::util::batch::DriveOperation::IdentityOperation;
use crate::util::batch::{DriveOperation, IdentityOperationType};

use crate::error::Error;
use dpp::block::epoch::Epoch;
use dpp::identity::identity_public_key::accessors::v0::IdentityPublicKeyGettersV0;
use dpp::identity::IdentityPublicKey;

use crate::error::drive::DriveError;
use crate::state_transition_action::identity::identity_update::IdentityUpdateTransitionAction;
use dpp::version::PlatformVersion;

impl DriveHighLevelOperationConverter for IdentityUpdateTransitionAction {
    fn into_high_level_drive_operations<'a>(
        self,
        _epoch: &Epoch,
        platform_version: &PlatformVersion,
    ) -> Result<Vec<DriveOperation<'a>>, Error> {
        match platform_version
            .drive
            .methods
            .state_transitions
            .convert_to_high_level_operations
            .identity_update_transition
        {
            0 => {
                let identity_id = self.identity_id();
                let revision = self.revision();
                let (add_public_keys, disable_public_keys) =
                    self.public_keys_to_add_and_disable_owned();

                let (unique_keys, non_unique_keys): (
                    Vec<IdentityPublicKey>,
                    Vec<IdentityPublicKey>,
                ) = add_public_keys
                    .into_iter()
                    .partition(|key| key.key_type().is_unique_key_type());

                let mut drive_operations = vec![];

                drive_operations.push(IdentityOperation(
                    IdentityOperationType::UpdateIdentityRevision {
                        identity_id: identity_id.to_buffer(),
                        revision,
                    },
                ));

                if !unique_keys.is_empty() || !non_unique_keys.is_empty() {
                    drive_operations.push(IdentityOperation(
                        IdentityOperationType::AddNewKeysToIdentity {
                            identity_id: identity_id.to_buffer(),
                            unique_keys_to_add: unique_keys,
                            non_unique_keys_to_add: non_unique_keys,
                        },
                    ));
                }
                if !disable_public_keys.is_empty() {
                    drive_operations.push(IdentityOperation(
                        IdentityOperationType::DisableIdentityKeys {
                            identity_id: identity_id.to_buffer(),
                            keys_ids: disable_public_keys,
                        },
                    ));
                }

                Ok(drive_operations)
            }
            version => Err(Error::Drive(DriveError::UnknownVersionMismatch {
                method: "IdentityUpdateTransitionAction::into_high_level_drive_operations"
                    .to_string(),
                known_versions: vec![0],
                received: version,
            })),
        }
    }
}
