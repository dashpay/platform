const cbor = require('cbor');

const MaxEncodedBytesReachedError = require('./errors/MaxEncodedBytesReachedError');

const MAX_ENCODED_KBYTE_LENGTH = 16; // 16Kb

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
    const encodedData = cbor.encodeCanonical(payload);
    const encodedDataByteLength = Buffer.byteLength(encodedData);

    if (encodedDataByteLength >= MAX_ENCODED_KBYTE_LENGTH * 1024) {
      throw new MaxEncodedBytesReachedError(payload, MAX_ENCODED_KBYTE_LENGTH);
    }

    return encodedData;
  },

  /**
   *
   * @param {Buffer|string} payload
   */
  decode(payload) {
    return cbor.decode(payload);
  },
};
