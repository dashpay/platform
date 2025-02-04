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

  const key1 = new IdentityPublicKey(1);
  key1.setData(Buffer.from('AuryIuMtRrl/VviQuyLD1l4nmxi9ogPzC9LT7tdpo0di', 'base64'));

  const key2 = new IdentityPublicKey(1);
  key2.setData(Buffer.from('A8AK95PYMVX5VQKzOhcVQRCUbc9pyg3RiL7jttEMDU+L', 'base64'));
  key2.setId(1);
  key2.setPurpose(IdentityPublicKey.PURPOSES.ENCRYPTION);
  key2.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.MEDIUM);

  const identity = new Identity(1);
  identity.setId(id);
  identity.setPublicKeys([key1, key2]);
  identity.setBalance(BigInt(10000));

  return identity;
};
