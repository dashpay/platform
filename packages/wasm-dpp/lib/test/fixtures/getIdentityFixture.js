const { default: loadWasmDpp } = require('../../..');
let { Identity, IdentityPublicKey } = require('../../..');
const generateRandomIdentifierAsync = require('../utils/generateRandomIdentifierAsync');

let staticId = null;

/**
 * @return {Identity}
 */
module.exports = async function getIdentityFixture(id = staticId, publicKeys = undefined) {
  ({ Identity, IdentityPublicKey } = await loadWasmDpp());

  if (!staticId) {
    staticId = await generateRandomIdentifierAsync();
  }

  if (!id) {
    // eslint-disable-next-line no-param-reassign
    id = staticId;
  }

  const preCreatedPublicKeys = [
    {
      $version: '0',
      id: 0,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MASTER,
      readOnly: false,
    },
    {
      $version: '0',
      id: 1,
      type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
      data: Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'),
      purpose: IdentityPublicKey.PURPOSES.ENCRYPTION,
      securityLevel: IdentityPublicKey.SECURITY_LEVELS.MEDIUM,
      readOnly: false,
    },
  ];

  const rawIdentity = {
    // TODO: obtain latest version from some wasm binding?
    $version: '0',
    id, // TODO: should be probably id.toBuffer(), but it causes panic in IdentityWasm
    balance: 10000,
    revision: 0,
    publicKeys: publicKeys === undefined ? preCreatedPublicKeys : publicKeys,
  };

  return new Identity(rawIdentity);
};
