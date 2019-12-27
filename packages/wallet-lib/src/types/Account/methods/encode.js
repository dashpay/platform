const cbor = require('cbor');

/**
 * Allow to canonical encode an input
 * Useful for encryption.
 * @param method
 * @param data
 */
const encode = function (method, data) {
  switch (method) {
    default:
      return cbor.encodeCanonical(data);
  }
};
module.exports = encode;
