const {
  default: loadWasmDpp, Identity, IdentityPublicKey,
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

  const key1 = new IdentityPublicKey(1);
  key1.setData(Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'));

  const key2 = new IdentityPublicKey(1);
  key2.setData(Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'));
  key2.setId(1);
  key2.setPurpose(IdentityPublicKey.PURPOSES.ENCRYPTION);
  key2.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);

  const newPublicKeys = publicKeys || [key1, key2];

  const identity = new Identity(1);
  identity.setId(id);
  identity.setPublicKeys(newPublicKeys);
  identity.setBalance(10000);

  return identity;
};
