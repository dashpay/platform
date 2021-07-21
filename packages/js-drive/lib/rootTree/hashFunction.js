const blake3 = require('blake3');

/**
 * @param {Buffer} data
 * @return {Buffer}
 */
function hashFunction(data) {
  return blake3.hash(data);
}

module.exports = hashFunction;
