const cbor = require('cbor');
const crypto = require('crypto');

function sha256(payload) {
  return crypto.createHash('sha256')
    .update(payload)
    .digest();
}
/**
 * Serialize and hash payload using double sha256
 *
 * @param payload
 * @return {string}
 */
module.exports = function doubleSha256(payload) {
  const serializedPayload = cbor.encodeCanonical(payload);
  return sha256(sha256(serializedPayload)).toString('hex');
};
