const byteArray = require('./byteArray');

/**
 * @param {ajv.Ajv} ajv
 */
function addByteArrayKeyword(ajv) {
  ajv.addKeyword('byteArray', byteArray);
}

module.exports = addByteArrayKeyword;
