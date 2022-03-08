const BlsSignatures = require('./bls');

/**
 * Create an instance of BlsPrivateKey
 *
 * @param {string|Buffer|Uint8Array|PrivateKey} privateKey string must be hex
 * @returns {Promise<PrivateKey>}
 */
async function blsPrivateKeyFactory(privateKey) {
  const blsSignatures = await BlsSignatures.getInstance();
  const { PrivateKey: BlsPrivateKey } = blsSignatures;

  let bytes;

  if (typeof privateKey === 'string') {
    const buf = Buffer.from(privateKey, 'hex');
    bytes = new Uint8Array(buf);
  } else if (Buffer.isBuffer(privateKey)) {
    bytes = new Uint8Array(privateKey);
  } else if (privateKey instanceof BlsPrivateKey) {
    return privateKey;
  } else {
    bytes = privateKey;
  }

  return BlsPrivateKey.fromBytes(bytes, true);
}

module.exports = blsPrivateKeyFactory;
