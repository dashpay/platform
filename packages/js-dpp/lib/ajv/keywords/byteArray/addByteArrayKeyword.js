const byteArrayKeyword = require('./byteArray');
const minBytesLength = require('./minBytesLength');
const maxBytesLength = require('./maxBytesLength');

/**
 * @param {Ajv} ajv
 */
function addByteArrayKeyword(ajv) {
  ajv.addKeyword('byteArray', byteArrayKeyword);
  ajv.addKeyword('minBytesLength', minBytesLength);
  ajv.addKeyword('maxBytesLength', maxBytesLength);
}

module.exports = addByteArrayKeyword;
