const multihashes = require('multihashes');

/**
 * Hash payload using multihash
 *
 * @param {Buffer} payload
 *
 * @return {Buffer}
 */
function hash(payload) {
  return multihashes.encode(payload, 'dbl-sha2-256');
}

/**
 * Validate hash is a valid multihash
 *
 * @param {Buffer} hashBuffer
 *
 * @return {boolean}
 */
function validate(hashBuffer) {
  try {
    multihashes.validate(hashBuffer);
  } catch (e) {
    return false;
  }

  return true;
}

module.exports = {
  hash,
  validate,
};
