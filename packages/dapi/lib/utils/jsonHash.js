const crypto = require('crypto');
const cbor = require('cbor');

/**
 * @param {object} data
 * @param {string} [algorithm] - hash algorithm
 * @return {string}
 */
function hash(data, algorithm = 'sha256') {
  const encodedData = cbor.encodeCanonical(data);
  return crypto.createHash(algorithm).update(encodedData).digest().toString('hex');
}

module.exports = hash;
