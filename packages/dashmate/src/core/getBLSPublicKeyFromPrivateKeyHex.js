const BlsSignatures = require('@dashevo/bls');

/**
 * @param {string} privateKeyHex
 * @returns {Promise<void>}
 */
async function getBLSPublicKeyFromPrivateKeyHex(privateKeyHex) {
  const { PrivateKey } = await BlsSignatures();

  const operatorPrivateKeyBuffer = Buffer.from(privateKeyHex, 'hex');

  const operatorPrivateKey = PrivateKey.fromBytes(
    operatorPrivateKeyBuffer,
    true,
  );

  const operatorPublicKey = operatorPrivateKey.getG1();

  return Buffer.from(operatorPublicKey.serialize()).toString('hex');
}

module.exports = getBLSPublicKeyFromPrivateKeyHex;
