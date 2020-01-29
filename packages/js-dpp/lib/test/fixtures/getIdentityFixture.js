const generateRandomId = require('../utils/generateRandomId');

const Identity = require('../../identity/Identity');
const IdentityPublicKey = require('../../identity/IdentityPublicKey');

const id = generateRandomId();

/**
 * @return {Identity}
 */
module.exports = function getIdentityFixture() {
  const rawIdentity = {
    id,
    type: Identity.TYPES.USER,
    publicKeys: [
      {
        id: 1,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di',
        isEnabled: true,
      },
      {
        id: 2,
        type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
        data: 'A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L',
        isEnabled: true,
      },
    ],
  };

  return new Identity(rawIdentity);
};
