use serde_json::json;
use crate::identifier::Identifier;
use crate::identity::Identity;

//3bufpwQjL5qsvuP4fmCKgXJrKG852DDMYfi9J6XKqPAT
//[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237]

pub fn identity_fixture_json() -> serde_json::Value {
    json!({"protocolVersion":1,"id":[198, 23, 40, 120, 58, 93, 0, 165, 27, 49, 4, 117, 107, 204,  67, 46, 164, 216, 230, 135, 201, 92, 31, 155, 62, 131, 211, 177, 139, 175, 163, 237],"publicKeys":[{"id":0,"type":0,"purpose":0,"securityLevel":0,"data":"AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di","readOnly":false},{"id":1,"type":0,"purpose":1,"securityLevel":3,"data":"A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L","readOnly":false}],"balance":10,"revision":0}
)
  //   Identity::
  //
  //   json!({
  //   protocolVersion: protocolVersion.latestVersion,
  //   id: id.toBuffer(),
  //   balance: 10,
  //   revision: 0,
  //   publicKeys: [
  //     {
  //       id: 0,
  //       type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  //       data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
  //       purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
  //       securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
  //       readOnly: false,
  //     },
  //     {
  //       id: 1,
  //       type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
  //       data: Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'),
  //       purpose: IdentityPublicKey.PURPOSES.ENCRYPTION,
  //       securityLevel: IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
  //       readOnly: false,
  //     },
  //   ],
  // })
}