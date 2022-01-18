/* eslint-disable no-bitwise */
/**
 * @typedef {encodeDocumentPropertyValue}
 * @param {*} propertyValue
 * @param {Object} propertyDefinition
 * @return Buffer
 */
function encodeDocumentPropertyValue(propertyValue, propertyDefinition) {
  let result;

  switch (propertyDefinition.type) {
    case 'boolean':
      result = propertyValue ? Buffer.from([1]) : Buffer.from([0]);
      break;
    case 'integer':
      result = Buffer.alloc(8);
      // Encode the integer in big endian form
      result.writeBigInt64BE(BigInt(propertyValue), 0);
      // Flip the sign bit
      result[0] ^= 0b10000000;
      break;
    case 'number':
      result = Buffer.alloc(8);

      // Encode in big endian form, so most significant bits are compared first
      result.writeDoubleBE(propertyValue, 0);

      if (propertyValue < 0) {
        // Check if the value is negative, if it is
        // flip all the bits i.e sign, exponent and mantissa

        // eslint-disable-next-line no-bitwise
        result = result.map((item) => ~item);
      } else {
        // for positive values, just flip the sign bit
        // eslint-disable-next-line no-bitwise
        result[0] ^= 0b10000000;
      }
      break;
    case 'array':
      if (propertyDefinition.byteArray) {
        result = Buffer.from(propertyValue);
      } else {
        throw new Error('Not implemented yet');
      }
      break;
    default:
      throw new Error('Not implemented yet');
  }

  return result;
}

module.exports = encodeDocumentPropertyValue;
