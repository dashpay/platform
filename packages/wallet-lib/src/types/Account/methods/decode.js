const cbor = require('cbor');

/**
 * Allow to decode an input
 * Useful for encryption.
 * @param method
 * @param data
 */
const decode = function (method, data) {
  switch (method) {
    default:
      return cbor.decodeFirstSync(data);
  }
};
module.exports = decode;
