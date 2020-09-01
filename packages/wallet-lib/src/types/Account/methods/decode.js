const cbor = require('cbor');

/**
 * Allow to decode an input
 * Useful for encryption.
 * @param {string} method
 * @param {any} data
 * @return {any}
 */
const decode = function decode(method, data) {
  switch (method) {
    default:
      return cbor.decodeFirstSync(data);
  }
};
module.exports = decode;
