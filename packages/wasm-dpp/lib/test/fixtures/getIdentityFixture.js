const {
  default: loadWasmDpp, Identity, IdentityPublicKey, IdentityFactory,
} = require('../../..');
const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');

let staticId = null;

/**
 * @return {Identity}
 */
module.exports = async function getIdentityFixture(id = staticId, publicKeys = undefined) {
  await loadWasmDpp();

  if (!staticId) {
    staticId = await generateRandomIdentifierAsync();
  }

  if (!id) {
    // eslint-disable-next-line no-param-reassign
    id = staticId;
  }

  const newPublicKeys = publicKeys || [
    new IdentityPublicKey({
      $version: '0',
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    }),
    new IdentityPublicKey({
      $version: '0',
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.ENCRYPTION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
      readOnly: false,
    }),
  ];

  const identityFactory = new IdentityFactory(1);
  const identity = identityFactory.create(
    id,
    newPublicKeys,
  );

  identity.setBalance(10000);
  return identity;
};
