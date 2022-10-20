const BlsSignatures = require('./bls');

/**
 * Create an instance of BlsPrivateKey
 *
 * @param {string|Buffer|Uint8Array|G1Element} publicKey string must be hex
 * @returns {Promise<G1Element>}
   */
async function blsPublicKeyFactory(publicKey) {
  const blsSignatures = await BlsSignatures.getInstance();
  const { G1Element } = blsSignatures;

  let bytes;

  if (typeof publicKey === 'string') {
    const buf = Buffer.from(publicKey, 'hex');
    bytes = new Uint8Array(buf);
  } else if (Buffer.isBuffer(publicKey)) {
    bytes = new Uint8Array(publicKey);
  } else if (publicKey instanceof G1Element) {
    return publicKey;
  } else {
    bytes = publicKey;
  }

  return G1Element.from_bytes(bytes);
}

module.exports = blsPublicKeyFactory;
