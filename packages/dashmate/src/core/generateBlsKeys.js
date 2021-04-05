const crypto = require('crypto');
const BlsSignatures = require('bls-signatures');

/**
 * Generate BLS keys
 *
 * @typedef {generateBlsKeys}
 * @return {Promise<{privateKey: *, address: *}>}
 */
async function generateBlsKeys() {
  const blsSignatures = await BlsSignatures;
  const { PrivateKey: BlsPrivateKey } = blsSignatures;

  const randomBytes = new Uint8Array(crypto.randomBytes(256));
  const operatorPrivateKey = BlsPrivateKey.fromBytes(randomBytes, true);
  const operatorPublicKey = operatorPrivateKey.getPublicKey();

  return {
    publicKey: Buffer.from(operatorPublicKey.serialize()).toString('hex'),
    privateKey: Buffer.from(operatorPrivateKey.serialize()).toString('hex'),
  };
}

module.exports = generateBlsKeys;
