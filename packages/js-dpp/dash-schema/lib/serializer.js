const cbor = require('cbor');

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
