const crypto = require('crypto');
const BlsSignatures = require('@dashevo/bls');

/**
 * Generate BLS keys
 *
 * @typedef {generateBlsKeys}
 * @return {Promise<{privateKey: *, address: *}>}
 */
async function generateBlsKeys() {
  const blsSignatures = await BlsSignatures();
  const { PrivateKey: BlsPrivateKey, BasicSchemeMPL } = blsSignatures;

  const randomBytes = new Uint8Array(crypto.randomBytes(256));
  const operatorPrivateKey = BlsPrivateKey.from_bytes(randomBytes, true);
  const operatorPublicKey = BasicSchemeMPL.sk_to_g1(operatorPrivateKey);

  return {
    publicKey: Buffer.from(operatorPublicKey.serialize()).toString('hex'),
    privateKey: Buffer.from(operatorPrivateKey.serialize()).toString('hex'),
  };
}

module.exports = generateBlsKeys;
