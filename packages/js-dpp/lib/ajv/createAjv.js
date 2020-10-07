const Ajv = require('ajv');

const addByteArrayKeyword = require('./keywords/byteArray/addByteArrayKeyword');

/**
 * @return {ajv.Ajv}
 */
function createAjv() {
  const ajv = new Ajv();

  addByteArrayKeyword(ajv);

  return ajv;
}

module.exports = createAjv;
