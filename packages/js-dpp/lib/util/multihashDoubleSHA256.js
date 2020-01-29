const multihashes = require('multihashes');
const doubleSha = require('./hash');

/**
 * Hash payload using multihash
 *
 * @param {Buffer} payload
 *
 * @return {Buffer}
 */
function hash(payload) {
  const digest = doubleSha(payload);

  return multihashes.encode(digest, 'dbl-sha2-256');
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
