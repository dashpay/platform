const byteArray = require('./byteArray');

/**
 * @param {Ajv2020} ajv
 */
function addByteArrayKeyword(ajv) {
  ajv.addKeyword(byteArray);
}

module.exports = addByteArrayKeyword;
