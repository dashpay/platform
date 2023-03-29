const { PrivateKey } = require('@dashevo/dashcore-lib');

const { default: loadWasmDpp } = require('../../..');

module.exports = async function getPrivateAndPublicKeyForSigningFixture(privateKeyHex = '9b67f852093bc61cea0eeca38599dbfba0de28574d2ed9b99d10d33dc1bde7b2') {
  const { IdentityPublicKey } = await loadWasmDpp();

  const privateKey = new PrivateKey(privateKeyHex);
  const publicKey = privateKey.toPublicKey();

  const rawPublicKey = {
    id: 1,
    type: IdentityPublicKey.TYPES.ECDSA_SECP256K1,
    data: publicKey.toBuffer(),
    purpose: IdentityPublicKey.PURPOSES.AUTHENTICATION,
    securityLevel: IdentityPublicKey.SECURITY_LEVELS.HIGH,
    readOnly: false,
  };

  const identityPublicKey = new IdentityPublicKey(rawPublicKey);

  return { privateKey: Buffer.from(privateKeyHex, 'hex'), identityPublicKey };
};
