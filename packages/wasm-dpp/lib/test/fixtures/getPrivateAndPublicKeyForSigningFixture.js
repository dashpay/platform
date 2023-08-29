const { PrivateKey } = require('@dashevo/dashcore-lib');

const { default: loadWasmDpp } = require('../../..');

module.exports = async function getPrivateAndPublicKeyForSigningFixture(privateKeyHex = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2') {
  const { IdentityPublicKey } = await loadWasmDpp();

  const privateKey = new PrivateKey(privateKeyHex);
  const publicKey = privateKey.toPublicKey();

  const identityPublicKey = new IdentityPublicKey(1);

  identityPublicKey.setId(2);
  identityPublicKey.setType(IdentityPublicKey.TYPES.ECDSA_SECP256K1);
  identityPublicKey.setData(publicKey.toBuffer());
  identityPublicKey.setPurpose(IdentityPublicKey.PURPOSES.AUTHENTICATION);
  identityPublicKey.setSecurityLevel(IdentityPublicKey.SECURITY_LEVELS.CRITICAL);
  identityPublicKey.setReadOnly(false);

  return { privateKey: Buffer.from(privateKeyHex, 'hex'), identityPublicKey };
};
