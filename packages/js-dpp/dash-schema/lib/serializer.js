const cbor = require('cbor');

/**
 * @typedef serializer
 * @type {{encode(*): Buffer, decode((Buffer|string)): *}}
 */
module.exports = {
  /**
   *
   * @param {*} payload
   * @return {Buffer}
   */
  encode(payload) {
    return cbor.encodeCanonical(payload);
  },

  /**
   *
   * @param {Buffer|string} payload
   */
  decode(payload) {
    return cbor.decode(payload);
  },
};
