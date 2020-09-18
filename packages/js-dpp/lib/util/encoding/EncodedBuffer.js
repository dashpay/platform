const bs58 = require('bs58');
const encodeToBase64WithoutPadding = require('./encodeToBase64WithoutPadding');
const InvalidBufferEncodingError = require('../../errors/InvalidBufferEncodingError');

// Buffer extending is not a trivial thing:
// * https://github.com/nodejs/node/commit/651a5b51eb838e8e23a5b94ba34e8e06630a004a
// * https://github.com/nodejs/node/issues/4701
// * https://github.com/nodejs/help/issues/1300
// * https://github.com/nodejs/node/issues/2882

/**
 * @param {Buffer} buffer
 * @param {EncodedBufferEncoding} encoding
 * @returns {EncodedBuffer}
 * @constructor
 */
function EncodedBuffer(buffer, encoding) {
  if (!Object.values(EncodedBuffer.ENCODING).includes(encoding)) {
    throw new InvalidBufferEncodingError(encoding);
  }

  const patchedBuffer = Buffer.from(buffer);

  patchedBuffer.encoding = encoding;

  Object.setPrototypeOf(patchedBuffer, EncodedBuffer.prototype);

  // noinspection JSValidateTypes
  return patchedBuffer;
}

/**
 * Get buffer encoding
 *
 * @return {EncodedBufferEncoding}
 */
EncodedBuffer.prototype.getEncoding = function getEncoding() {
  return this.encoding;
};

/**
 * Encode CBOR
 *
 * @param {Encoder} encoder
 * @return {boolean}
 */
EncodedBuffer.prototype.encodeCBOR = function encodeCBOR(encoder) {
  encoder.push(this.toBuffer());

  return true;
};

/**
 * Convert to normal Buffer
 *
 * @return {Buffer}
 */
EncodedBuffer.prototype.toBuffer = function toBuffer() {
  return Buffer.from(this);
};

/**
 * Encode to string according to defined encoding
 *
 * @return {string}
 */
EncodedBuffer.prototype.toString = function toString() {
  const encoding = this.getEncoding();

  switch (this.getEncoding()) {
    case EncodedBuffer.ENCODING.BASE64:
      return encodeToBase64WithoutPadding(this.toBuffer());
    case EncodedBuffer.ENCODING.BASE58:
      return bs58.encode(this.toBuffer());
    default:
      throw new InvalidBufferEncodingError(encoding);
  }
};

/**
 * Convert to JSON
 *
 * @return {string}
 */
EncodedBuffer.prototype.toJSON = function toBuffer() {
  return this.toString();
};

/**
 * Convert from string to EncodedBuffer class
 *
 * @param {string|buffer} value
 * @param {EncodedBufferEncoding} encoding
 * @return {EncodedBuffer}
 */
EncodedBuffer.from = function from(value, encoding) {
  let buffer;

  if (!Object.values(EncodedBuffer.ENCODING).includes(encoding)) {
    throw new InvalidBufferEncodingError(encoding);
  }

  if (typeof value === 'string') {
    // eslint-disable-next-line default-case
    switch (encoding) {
      case EncodedBuffer.ENCODING.BASE64:
        buffer = Buffer.from(value, 'base64');
        break;
      case EncodedBuffer.ENCODING.BASE58:
        buffer = bs58.decode(value);
        break;
    }
  } else {
    buffer = value;
  }

  return new EncodedBuffer(buffer, encoding);
};

Object.setPrototypeOf(EncodedBuffer.prototype, Buffer.prototype);
Object.setPrototypeOf(EncodedBuffer, Buffer);

/**
 * @readonly
 * @enum {EncodedBufferEncoding}
 * @typedef {string} EncodedBufferEncoding
 */
EncodedBuffer.ENCODING = {
  BASE58: 'base58',
  BASE64: 'base64',
};

module.exports = EncodedBuffer;
