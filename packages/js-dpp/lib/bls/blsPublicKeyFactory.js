const BlsSignatures = require('./bls');

/**
 * Create an instance of BlsPrivateKey
 *
 * @param {string|Buffer|Uint8Array|PrivateKey} publicKey string must be hex
 * @returns {Promise<PublicKey>}
   */
async function blsPublicKeyFactory(publicKey) {
  const blsSignatures = await BlsSignatures.getInstance();
  const { PublicKey } = blsSignatures;

  let bytes;

  if (typeof publicKey === 'string') {
    const buf = Buffer.from(publicKey, 'hex');
    bytes = new Uint8Array(buf);
  } else if (Buffer.isBuffer(publicKey)) {
    bytes = new Uint8Array(publicKey);
  } else if (publicKey instanceof PublicKey) {
    return publicKey;
  } else {
    bytes = publicKey;
  }

  return PublicKey.fromBytes(bytes);
}

module.exports = blsPublicKeyFactory;
