const generateRandomIdentifier = require('../utils/generateRandomIdentifier');

const protocolVersion = require('../../version/protocolVersion');

const Identity = require('../../identity/Identity');
const IdentityPublicKey = require('../../identity/IdentityPublicKey');

const randomIdentifier = generateRandomIdentifier();

/**
 * @param {Identifier} id
 * @param {Identity} [IdentityClass]
 * @param {IdentityPublicKey} [IdentityPublicKeyClass]
 * @return {Identity}
 */
module.exports = function getIdentityFixture(
  id = randomIdentifier,
  IdentityClass = Identity,
  IdentityPublicKeyClass = IdentityPublicKey,
) {
  const rawIdentity = {
    protocolVersion: protocolVersion.latestVersion,
    id: id.toBuffer(),
    balance: 10,
    revision: 0,
    publicKeys: [
      {
        id: 0,
        type: IdentityPublicKeyClass.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
        purpose: IdentityPublicKeyClass.PURPOSES.AUTHENTICATION,
        securityLevel: IdentityPublicKeyClass.SECURITY_LEVELS.MASTER,
        readOnly: false,
      },
      {
        id: 1,
        type: IdentityPublicKeyClass.TYPES.ECDSA_SECP256K1,
        data: Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'),
        purpose: IdentityPublicKeyClass.PURPOSES.ENCRYPTION,
        securityLevel: IdentityPublicKeyClass.SECURITY_LEVELS.MEDIUM,
        readOnly: false,
      },
    ],
  };

  return new IdentityClass(rawIdentity);
};
