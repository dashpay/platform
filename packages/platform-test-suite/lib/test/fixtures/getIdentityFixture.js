const Dash = require('dash');

const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const {
  Platform,
} = Dash;

/**
 * @return {Identity}
 */
module.exports = async function getIdentityFixture() {
  const { Identity, IdentityPublicKey } = await Platform
    .initializeDppModule();

  const id = await generateRandomIdentifier();

  const rawIdentity = {
    protocolVersion: 1,
    id: id.toBuffer(),
    balance: 10,
    revision: 0,
    publicKeys: [
      {
        id: 0,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
        readOnly: false,
      },
      {
        id: 1,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'),
        purpose: IdentityPublicKey.PURPOSES.ENCRYPTION,
        securityLevel: IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
        readOnly: false,
      },
    ],
  };

  return new Identity(rawIdentity);
};
