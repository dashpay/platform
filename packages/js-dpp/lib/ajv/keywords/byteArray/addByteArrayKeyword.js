const byteArray = require('./byteArray');

/**
 * @param {ajv.Ajv} ajv
 */
function addByteArrayKeyword(ajv) {
  ajv.addKeyword(byteArray);
}

module.exports = addByteArrayKeyword;
