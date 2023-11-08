const crypto = require('crypto');

function sha256(payload) {
  return crypto.createHash('sha256')
    .update(payload)
    .digest();
}
/**
 * Serialize and hash payload using double sha256
 *
 * @param {Buffer} buffer
 * @return {Buffer}
 */
function hash(buffer) {
  return sha256(sha256(buffer));
}

module.exports = { hash };
