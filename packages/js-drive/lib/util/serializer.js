const cbor = require('cbor');

/**
 * Encode an object to a binary Buffer
 *
 * @param {Object} data
 * @return {Buffer}
 */
function encode(data) {
  return cbor.encode(data);
}

/**
 * Decode a Buffer into an object
 *
 * @param {Buffer} encodedData
 * @return {Object}
 */
function decode(encodedData) {
  return cbor.decode(encodedData);
}

module.exports = {
  encode,
  decode,
};
